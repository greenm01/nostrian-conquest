"""Score a fleet-level agent: one hybrid seat, with the given families
handed over, against three seats of an opponent, on held-out maps with the
learner's seat rotated. An even seat wins 0.25.

    uv run fleet_evaluate.py /tmp/nc-ai/fleet/imitate.pt \\
        --strategist strategist-final.pt --families colonize,scout --opponent strategist
"""

import argparse
import time

import policy
from fleet_policy import load_fleet_agent
from fleet_ppo import head_to_head
from policy import device


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("checkpoint")
    parser.add_argument("--strategist", required=True, help="frozen strategist checkpoint")
    parser.add_argument("--families", default="colonize,scout,strike")
    parser.add_argument("--opponent", default="strategist",
                        help="strategist, executor, baseline, classic or aggressor")
    parser.add_argument("--games", type=int, default=1024)
    parser.add_argument("--years", type=int, default=60)
    parser.add_argument("--first-map", type=int, default=100000)
    parser.add_argument("--greedy", action="store_true",
                        help="take each head's most likely choice")
    parser.add_argument("--style", help="the learner's executor personality (t167)")
    args = parser.parse_args()

    dev = device()
    model = load_fleet_agent(args.checkpoint, dev)
    strategist = policy.load(args.strategist, dev)
    families = [f for f in args.families.split(",") if f]
    started = time.perf_counter()
    war_years = []
    wins = head_to_head(model, strategist, families, args.opponent, args.games, args.years,
                        args.first_map, dev, greedy=args.greedy, style=args.style,
                        war_years=war_years)
    mode = "greedy" if args.greedy else "sampled"
    wars = [year for year in war_years if year is not None]
    war = (f"first war goal in year {sum(wars) / len(wars):.1f} ({len(wars)} games)"
           if wars else "no war goal")
    print(f"{args.checkpoint} [{','.join(families)}{'@' + args.style if args.style else ''}] "
          f"{mode} vs 3x {args.opponent}, {args.games} maps from {args.first_map}: "
          f"win share {wins:.3f}, {war} ({time.perf_counter() - started:.0f}s)")


if __name__ == "__main__":
    main()
