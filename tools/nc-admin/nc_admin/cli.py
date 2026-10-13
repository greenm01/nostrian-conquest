"""Agent-friendly campaign administration over the local nc-host protocol."""
from __future__ import annotations
import argparse
import datetime as dt
import json
import os
import sys
from pathlib import Path
from .client import AdminError, Client


def deadline(value: str) -> int:
    try:
        parsed = dt.datetime.fromisoformat(value.replace("Z", "+00:00"))
        if parsed.tzinfo is None:
            raise ValueError("timezone required")
        return int(parsed.timestamp())
    except ValueError as exc:
        raise argparse.ArgumentTypeError("use an ISO timestamp with timezone, e.g. 2026-10-20T04:00:00Z") from exc


def parser() -> argparse.ArgumentParser:
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--root", type=Path, default=argparse.SUPPRESS)
    common.add_argument("--host-binary", default=argparse.SUPPRESS)
    common.add_argument("--json", action="store_true", default=argparse.SUPPRESS)
    common.add_argument("--dry-run", action="store_true", default=argparse.SUPPRESS)
    p = argparse.ArgumentParser(prog="nc-admin", parents=[common], allow_abbrev=False)
    groups = p.add_subparsers(dest="group", required=True)
    games = groups.add_parser("games", parents=[common]).add_subparsers(dest="command", required=True)
    for command in ("list", "inspect", "create", "publish", "start", "pause", "resume", "archive", "schedule", "history", "replace", "link-account", "unlink-account", "invite-only"):
        sub = games.add_parser(command, parents=[common], allow_abbrev=False)
        if command != "list": sub.add_argument("game")
        if command == "create":
            sub.add_argument("--name")
            sub.add_argument("--players", type=int, required=True, choices=range(1,26), metavar="1..25")
            sub.add_argument("--seed", type=int)
        if command in ("start", "resume"):
            sub.add_argument("--first-due", type=deadline)
        if command == "schedule":
            schedule = sub.add_mutually_exclusive_group(required=True)
            schedule.add_argument("--first-due", type=deadline)
            schedule.add_argument("--midnight", metavar="IANA_ZONE", help="Daily local midnight, adjusted for daylight saving")
            schedule.add_argument("--manual", action="store_true")
        if command in ("start", "schedule"):
            sub.add_argument("--interval-minutes", type=int, default=1440)
        if command in ("replace", "link-account", "unlink-account"):
            sub.add_argument("--seat", type=int, required=True)
            identity = sub.add_mutually_exclusive_group(required=True)
            identity.add_argument("--discord-id")
            identity.add_argument("--nostr-pubkey", help="64-character lowercase hex public key")
            identity.add_argument("--matrix-id", help="full Matrix user ID, such as @alice:matrix.org")
            sub.add_argument("--confirm", required=True, help="Repeat the game ID to confirm the account change")
        if command == "archive": sub.add_argument("--confirm", required=True)
    invites = groups.add_parser("invites", parents=[common]).add_subparsers(dest="command", required=True)
    for command in ("create", "list", "revoke"):
        sub = invites.add_parser(command, parents=[common])
        sub.add_argument("game")
        if command == "create": sub.add_argument("--count", type=int, default=1, choices=range(1,26), metavar="1..25")
        if command == "revoke": sub.add_argument("invite_id")
    maintenance = groups.add_parser("maintenance", parents=[common]).add_subparsers(dest="command", required=True)
    maintenance.add_parser("due", parents=[common])
    run = maintenance.add_parser("run", parents=[common])
    run.add_argument("game")
    run.add_argument("--expect-year", type=int, required=True)
    run.add_argument("--request-id", required=True)
    backups = groups.add_parser("backup", parents=[common]).add_subparsers(dest="command", required=True)
    backups.add_parser("daily", parents=[common])
    create = backups.add_parser("create", parents=[common]); create.add_argument("game")
    verify = backups.add_parser("verify", parents=[common]); verify.add_argument("path")
    restore = backups.add_parser("restore", parents=[common]); restore.add_argument("path"); restore.add_argument("--into-root", type=Path, required=True)
    restore.add_argument("--confirm", required=True, help="Original game ID; restoration never overwrites a game")
    groups.add_parser("doctor", parents=[common])
    return p


def request_for(args: argparse.Namespace) -> dict:
    if args.group == "doctor": return {"action":"list"}
    if args.group == "backup":
        if args.command == "daily": return {"action":"daily_backups"}
        if args.command == "create": return {"action":"backup", "game":args.game}
        return {"action":"verify_backup" if args.command == "verify" else "restore", "path":str(Path(args.path).resolve()),
                **({"game":args.confirm} if args.command == "restore" else {})}
    if args.group == "invites":
        return {"action":{"create":"create_invites","list":"list_invites","revoke":"revoke_invite"}[args.command],
                "game":args.game, **({"count":args.count} if args.command == "create" else {}),
                **({"invite_id":args.invite_id} if args.command == "revoke" else {})}
    if args.group == "games":
        if args.command == "invite-only": return {"action":"invite_only", "game":args.game}
        if args.command in ("start", "resume") and args.first_due is None:
            return {"action":"start_configured", "game":args.game}
        if args.command == "schedule" and args.first_due is None:
            return {"action":"configure_schedule", "game":args.game, "time_zone":args.midnight}
    if args.group == "maintenance":
        if args.command == "due": return {"action":"due"}
        return {"action":"maintain", "game":args.game, "expected_year":args.expect_year, "request_id":args.request_id}
    request = {"action":args.command.replace("-", "_")}
    if args.command != "list": request["game"] = args.game
    if args.command == "create": request.update(name=args.name or args.game, players=args.players, seed=args.seed)
    if args.command in ("start", "resume", "schedule"): request["first_due"] = args.first_due
    if args.command in ("start", "schedule"): request["interval_minutes"] = args.interval_minutes
    if args.command in ("replace", "link-account", "unlink-account", "archive") and args.confirm != args.game:
        raise AdminError("confirmation must exactly match the game ID")
    if args.command in ("replace", "link-account", "unlink-account"):
        provider, id = next((p, v) for p, v in (("discord", args.discord_id), ("nostr", args.nostr_pubkey), ("matrix", args.matrix_id)) if v)
        request.update(seat=args.seat, account={"provider":provider, "id":id})
    return request


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    # Apply defaults after parsing. Parent parsers share action objects; setting
    # their defaults earlier lets subparsers overwrite explicitly given options.
    for name, value in {
        "root": Path(os.environ.get("NC_GAMES_ROOT", "/srv/nc/community/games")),
        "host_binary": os.environ.get("NC_HOST_BINARY", "nc-host"),
        "json": False, "dry_run": False,
    }.items():
        if not hasattr(args, name): setattr(args, name, value)
    try:
        request = request_for(args)
        root = args.into_root if args.group == "backup" and args.command == "restore" else args.root
        if args.dry_run:
            data = {"preview":True,"root":str(root),"request":request}
        else:
            data = Client(root, args.host_binary).call(request)
        if args.group == "doctor" and not args.dry_run:
            issues = [row for row in data if "error" in row or row.get("state") == "maintenance" or not row.get("last_backup")]
            data = {"healthy":not issues,"issues":issues,"games":data}
        reply = {"version":1,"ok":True,"data":data}
        print(json.dumps(reply, indent=None if args.json else 2))
        if args.group == "doctor" and not args.dry_run and not data["healthy"]: return 1
        if not args.dry_run and isinstance(data, list) and any("error" in row or row.get("paused") for row in data): return 1
        return 0
    except (AdminError, OSError, ValueError) as exc:
        if args.json: print(json.dumps({"version":1,"ok":False,"error":str(exc)}))
        else: print(f"nc-admin: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
