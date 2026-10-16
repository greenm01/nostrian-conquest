"""The agent league (t156): seat-rotated four-player matches between
registered agents, stored as replays, rated with OpenSkill.

    uv run league.py --db /tmp/nc-ai/league.db add-agent executor --scripted executor
    uv run league.py --db ... add-agent final --checkpoint strategist-final.pt
    uv run league.py --db ... add-agent fleet --fleet fleet-final.pt
    uv run league.py --db ... play --games 256
    uv run league.py --db ... ratings
    uv run league.py --db ... verify

An agent is a scripted policy (executor, baseline, classic, aggressor,
autopilot, or a personality such as rusher, turtle, armada or raider), a strategist
checkpoint, whose goals the tuned executor
carries out, or a fleet-level agent checkpoint (t157), which also orders
the fleets of its handed-over families. Both play greedy, as a deployed
seat does (t193: strategists were sampled before, which flattered the
fleet agents' margin; matches stored earlier were played that way). Its play therefore also depends on the code: each match
records the commit it was played at. Stored replays hold the actual
orders, so they stay valid whatever the code does later.

A match stores its map and campaign seeds, its seats, its result, and every
turn applied, as turn KDL. The engine is deterministic, so `verify` rebuilds
each match from its seeds and turns, checks the stored result, and rebuilds
the rating table from the replayed results.

Ratings use OpenSkill's Plackett-Luce model (Weng and Lin, 2011), a
Bayesian rating for free-for-all games like TrueSkill, without TrueSkill's
patent and trademark. A match ranks seats by win share, then production at
the end. The displayed rating is mu - 3 sigma, a cautious estimate.
"""

import argparse
import datetime
import hashlib
import json
import sqlite3
import subprocess
import zlib
from pathlib import Path

import numpy as np
import torch
from openskill.models import PlackettLuce

import nc_ai_env
import fleet_policy
from fleet_imitate import actions_for
from policy import act, bucket, device, goals_for, load, seat_batch

PLAYERS = 4
SCRIPTED = ("executor", "baseline", "classic", "aggressor", "autopilot", "rusher", "turtle",
            "armada", "raider")
# League maps start here, clear of training (1+, 1,000,000+) and evaluation
# (100000+, 200000+) maps.
FIRST_MAP = 5_000_000
CAMPAIGN_XOR = 0x1EA6

SCHEMA = """
create table if not exists agents (
    id integer primary key,
    name text not null unique,
    kind text not null check (kind in ('scripted', 'strategist', 'fleet')),
    spec text not null,          -- scripted policy, or checkpoint path
    sha256 text,                 -- checkpoint digest
    added text not null
);
create table if not exists matches (
    id integer primary key,
    map_seed integer not null,
    campaign_seed integer not null,
    years integer not null,      -- the year limit
    years_played integer not null,
    winner integer not null,     -- empire slot, 0 for none
    played text not null,
    code text not null           -- git commit, with +dirty for local changes
);
create table if not exists seats (
    match_id integer not null references matches (id),
    seat integer not null,
    agent_id integer not null references agents (id),
    win_share real not null,
    production integer not null,
    worlds integer not null,
    rank integer not null,
    primary key (match_id, seat)
);
create table if not exists replays (
    match_id integer primary key references matches (id),
    turns blob not null          -- zlib-compressed JSON list of turn KDL
);
create table if not exists ratings (
    agent_id integer primary key references agents (id),
    mu real not null,
    sigma real not null,
    matches integer not null
);
"""


def connect(path):
    db = sqlite3.connect(path)
    db.executescript(SCHEMA)
    return db


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def add_agent(db, name, scripted=None, checkpoint=None, fleet=None):
    if sum(x is not None for x in (scripted, checkpoint, fleet)) != 1:
        raise SystemExit("give exactly one of --scripted, --checkpoint and --fleet")
    if scripted is not None and scripted not in SCRIPTED:
        raise SystemExit(f"--scripted must be one of {', '.join(SCRIPTED)}")
    if scripted:
        kind, spec, digest = "scripted", scripted, None
    else:
        path = checkpoint or fleet
        kind = "strategist" if checkpoint else "fleet"
        spec, digest = str(Path(path).resolve()), sha256(path)
    db.execute(
        "insert into agents (name, kind, spec, sha256, added) values (?, ?, ?, ?, ?)",
        (name, kind, spec, digest, now()),
    )
    db.commit()


def code_version():
    root = Path(__file__).resolve().parents[2]
    git = lambda *args: subprocess.run(
        ["git", "-C", str(root), *args], capture_output=True, text=True, check=True
    ).stdout.strip()
    dirty = git("status", "--porcelain", "--", "rust", "tools/ai")
    return git("rev-parse", "--short", "HEAD") + ("+dirty" if dirty else "")


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds")


def win_shares(result):
    """MatchResult::win_share for every seat."""
    if result["winner"]:
        return [1.0 if seat + 1 == result["winner"] else 0.0 for seat in range(PLAYERS)]
    best = max(result["production"])
    leaders = sum(p == best for p in result["production"])
    return [1.0 / leaders if p == best else 0.0 for p in result["production"]]


def ranks(result):
    """1 for the best seat, by win share then production; ties share."""
    keys = list(zip(win_shares(result), result["production"]))
    return [1 + sum(other > key for other in keys) for key in keys]


def play(db, games, years, batch, seed):
    agents = db.execute("select id, name, kind, spec, sha256 from agents order by id").fetchall()
    if len(agents) < 2:
        raise SystemExit("register at least two agents")
    dev = device()
    models, fleets, seat_names = {}, {}, {}
    for agent_id, name, kind, spec, digest in agents:
        if kind != "scripted" and sha256(spec) != digest:
            raise SystemExit(f"checkpoint for {name} changed since it was added")
        if kind == "strategist":
            models[agent_id] = load(spec, dev)
            seat_names[agent_id] = "strategist"
        elif kind == "fleet":
            families = torch.load(spec, map_location="cpu", weights_only=False)["families"]
            fleets[agent_id] = fleet_policy.load_fleet_agent(spec, dev)
            seat_names[agent_id] = "hybrid:" + ",".join(families)
        else:
            seat_names[agent_id] = spec
    code = code_version()
    rng = np.random.default_rng(seed)
    torch.manual_seed(seed)
    first = FIRST_MAP + (db.execute("select coalesce(max(id), 0) from matches").fetchone()[0])
    env = nc_ai_env.Env(PLAYERS, years)
    for start in range(0, games, batch):
        count = min(batch, games - start)
        maps = [first + start + game for game in range(count)]
        campaigns = [m ^ CAMPAIGN_XOR for m in maps]
        # Four agents per game, drawn with replacement when the pool is
        # small, in a seat order rotated by the game.
        lineups = []
        for game in range(count):
            picked = [agents[i] for i in rng.choice(len(agents), PLAYERS, len(agents) < PLAYERS)]
            shift = (start + game) % PLAYERS
            lineups.append(picked[shift:] + picked[:shift])
        env.reset(
            maps,
            campaigns,
            seats=[[seat_names[a[0]] for a in lineup] for lineup in lineups],
        )
        owner = np.array([[a[0] for a in lineup] for lineup in lineups])
        done = np.zeros(count, bool)
        while not done.all():
            goals, actions = None, None
            if models or fleets:
                obs, masks, goal_masks = env.observe(), env.masks(), env.goal_masks()
                goals = {name: np.zeros_like(value) for name, value in env.goal_labels().items()}
                fleet_choices = {name: np.zeros((count, PLAYERS, env.fleets), np.int64)
                                 for name in fleet_policy.FLEET_HEADS}
                with torch.no_grad():
                    for agent_id, model in fleets.items():
                        select = (owner == agent_id) & ~done[:, None]
                        if not select.any():
                            continue
                        batch_ = fleet_policy.bucket(
                            fleet_policy.fleet_batch(obs, masks, goal_masks, select, dev))
                        chosen_goals, chosen_fleets, *_ = model.act(batch_, greedy=True)
                        chosen = goals_for(chosen_goals, select)
                        for name in goals:
                            goals[name][select] = chosen[name][select]
                        placed = fleet_policy.fleet_arrays(chosen_fleets, select, env.fleets)
                        for name in fleet_choices:
                            fleet_choices[name][select] = placed[name][select]
                    actions = actions_for(env, fleet_choices) if fleets else None
                    for agent_id, model in models.items():
                        select = (owner == agent_id) & ~done[:, None]
                        if not select.any():
                            continue
                        batch_ = bucket(seat_batch(obs, masks, goal_masks, select, dev))
                        choices, _, _ = act(model, batch_, greedy=True)
                        chosen = goals_for(choices, select)
                        for name in goals:
                            goals[name][select] = chosen[name][select]
            done = env.step(actions, goals)["done"].copy()
        for game in range(count):
            store_match(db, maps[game], campaigns[game], years, lineups[game],
                        env.result(game), env.turns(game), code)
        db.commit()
        print(f"played {start + count} of {games} games", flush=True)


def store_match(db, map_seed, campaign_seed, years, lineup, result, turns, code):
    cursor = db.execute(
        "insert into matches"
        " (map_seed, campaign_seed, years, years_played, winner, played, code)"
        " values (?, ?, ?, ?, ?, ?, ?)",
        (map_seed, campaign_seed, years, result["years"], result["winner"], now(), code),
    )
    match_id = cursor.lastrowid
    for seat, (agent, share, rank) in enumerate(zip(lineup, win_shares(result), ranks(result))):
        db.execute(
            "insert into seats values (?, ?, ?, ?, ?, ?, ?)",
            (match_id, seat, agent[0], share, result["production"][seat],
             result["worlds"][seat], rank),
        )
    db.execute(
        "insert into replays values (?, ?)",
        (match_id, zlib.compress(json.dumps(turns).encode(), 9)),
    )


def compute_ratings(match_ranks):
    """OpenSkill ratings from ``(match id, [(agent id, rank), ...])`` in
    match order: ``{agent id: (mu, sigma, matches)}``."""
    model = PlackettLuce()
    ratings, counts = {}, {}
    for _, seats in sorted(match_ranks):
        teams = [[ratings.get(agent) or model.rating(name=str(agent))] for agent, _ in seats]
        rated = model.rate(teams, ranks=[rank for _, rank in seats])
        for (agent, _), team in zip(seats, rated):
            ratings[agent] = team[0]
            counts[agent] = counts.get(agent, 0) + 1
    return {agent: (r.mu, r.sigma, counts[agent]) for agent, r in ratings.items()}


def stored_ranks(db):
    by_match = {}
    for match_id, agent, rank in db.execute(
        "select match_id, agent_id, rank from seats order by match_id, seat"
    ):
        by_match.setdefault(match_id, []).append((agent, rank))
    return list(by_match.items())


def rate(db):
    db.execute("delete from ratings")
    for agent, (mu, sigma, matches) in compute_ratings(stored_ranks(db)).items():
        db.execute("insert into ratings values (?, ?, ?, ?)", (agent, mu, sigma, matches))
    db.commit()


def show(db):
    rows = db.execute(
        "select a.name, a.kind, r.mu, r.sigma, r.matches,"
        " (select avg(win_share) from seats s where s.agent_id = a.id)"
        " from ratings r join agents a on a.id = r.agent_id"
        " order by r.mu - 3 * r.sigma desc"
    ).fetchall()
    print(f"{'agent':<16}{'kind':<12}{'rating':>8}{'mu':>8}{'sigma':>7}{'matches':>9}{'win':>7}")
    for name, kind, mu, sigma, matches, win in rows:
        print(f"{name:<16}{kind:<12}{mu - 3 * sigma:8.2f}{mu:8.2f}{sigma:7.2f}"
              f"{matches:9}{win:7.3f}")


def verify(db):
    """Replay every match from its seeds and turns, check its stored result,
    and rebuild the rating table from the replayed results."""
    agents = dict(db.execute("select id, spec from agents"))
    replayed = []
    for match_id, map_seed, campaign_seed, years, years_played, winner, blob in db.execute(
        "select m.id, m.map_seed, m.campaign_seed, m.years, m.years_played, m.winner, r.turns"
        " from matches m join replays r on r.match_id = m.id order by m.id"
    ).fetchall():
        seats = db.execute(
            "select agent_id, production, worlds from seats where match_id = ? order by seat",
            (match_id,),
        ).fetchall()
        autopilot = [agents[agent] == "autopilot" for agent, _, _ in seats]
        turns = json.loads(zlib.decompress(blob))
        result = nc_ai_env.replay(map_seed, campaign_seed, autopilot, years, turns)
        expected = (years_played, winner, [p for _, p, _ in seats], [w for _, _, w in seats])
        got = (result["years"], result["winner"], result["production"], result["worlds"])
        if got != expected:
            raise SystemExit(f"match {match_id} replays differently: {got} != {expected}")
        replayed.append((match_id, list(zip([a for a, _, _ in seats], ranks(result)))))
    rebuilt = compute_ratings(replayed)
    stored = {agent: (mu, sigma, n) for agent, mu, sigma, n in db.execute("select * from ratings")}
    if set(rebuilt) != set(stored) or any(
        abs(rebuilt[a][0] - stored[a][0]) > 1e-9
        or abs(rebuilt[a][1] - stored[a][1]) > 1e-9
        or rebuilt[a][2] != stored[a][2]
        for a in rebuilt
    ):
        raise SystemExit("the rating table differs from the one rebuilt from replays")
    print(f"verified {len(replayed)} matches; the rating table rebuilds exactly")


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--db", required=True)
    sub = parser.add_subparsers(dest="command", required=True)
    add = sub.add_parser("add-agent")
    add.add_argument("name")
    add.add_argument("--scripted")
    add.add_argument("--checkpoint")
    add.add_argument("--fleet", help="fleet-level agent checkpoint")
    run = sub.add_parser("play")
    run.add_argument("--games", type=int, default=256)
    run.add_argument("--years", type=int, default=60)
    run.add_argument("--batch", type=int, default=128)
    run.add_argument("--seed", type=int, default=1)
    sub.add_parser("ratings")
    sub.add_parser("verify")
    args = parser.parse_args()

    db = connect(args.db)
    if args.command == "add-agent":
        add_agent(db, args.name, args.scripted, args.checkpoint, args.fleet)
    elif args.command == "play":
        play(db, args.games, args.years, args.batch, args.seed)
        rate(db)
        show(db)
    elif args.command == "ratings":
        rate(db)
        show(db)
    else:
        verify(db)


if __name__ == "__main__":
    main()
