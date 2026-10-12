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
    p.set_defaults(root=Path(os.environ.get("NC_GAMES_ROOT", "/srv/nc/community/games")),
                   host_binary=os.environ.get("NC_HOST_BINARY", "nc-host"), json=False, dry_run=False)
    groups = p.add_subparsers(dest="group", required=True)
    games = groups.add_parser("games", parents=[common]).add_subparsers(dest="command", required=True)
    for command in ("list", "inspect", "create", "publish", "start", "pause", "resume", "archive", "schedule", "history", "replace"):
        sub = games.add_parser(command, parents=[common], allow_abbrev=False)
        if command != "list": sub.add_argument("game")
        if command == "create":
            sub.add_argument("--name")
            sub.add_argument("--players", type=int, required=True, choices=range(1,26), metavar="1..25")
            sub.add_argument("--seed", type=int)
        if command in ("start", "resume", "schedule"):
            sub.add_argument("--first-due", type=deadline, required=True)
        if command in ("start", "schedule"):
            sub.add_argument("--interval-minutes", type=int, default=1440)
        if command == "replace":
            sub.add_argument("--seat", type=int, required=True)
            sub.add_argument("--discord-id", required=True)
            sub.add_argument("--confirm", required=True, help="Repeat the game ID to confirm replacement")
        if command == "archive": sub.add_argument("--confirm", required=True)
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
    if args.group == "maintenance":
        if args.command == "due": return {"action":"due"}
        return {"action":"maintain", "game":args.game, "expected_year":args.expect_year, "request_id":args.request_id}
    request = {"action":args.command}
    if args.command != "list": request["game"] = args.game
    if args.command == "create": request.update(name=args.name or args.game, players=args.players, seed=args.seed)
    if args.command in ("start", "resume", "schedule"): request["first_due"] = args.first_due
    if args.command in ("start", "schedule"): request["interval_minutes"] = args.interval_minutes
    if args.command in ("replace", "archive") and args.confirm != args.game:
        raise AdminError("confirmation must exactly match the game ID")
    if args.command == "replace": request.update(seat=args.seat, discord_id=args.discord_id)
    return request


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
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
