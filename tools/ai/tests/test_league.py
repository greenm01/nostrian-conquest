"""The league stores matches it can replay and rebuilds its ratings from
the replays (t156)."""

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import league  # noqa: E402


class LeagueReplays(unittest.TestCase):
    def test_ratings_rebuild_from_stored_replays(self):
        with tempfile.TemporaryDirectory() as tmp:
            db = league.connect(Path(tmp) / "league.db")
            for name in ("executor", "baseline", "classic", "aggressor", "autopilot"):
                league.add_agent(db, name, scripted=name)
            league.play(db, games=8, years=12, batch=8, seed=3)
            league.rate(db)
            league.verify(db)
            matches, = db.execute("select count(*) from matches").fetchone()
            self.assertEqual(matches, 8)

            # A tampered result no longer matches its replay.
            db.execute("update seats set production = production + 1"
                       " where match_id = 1 and seat = 0")
            with self.assertRaises(SystemExit):
                league.verify(db)
            db.close()

    def test_ranks_order_win_share_then_production(self):
        result = {"winner": 0, "production": [50, 90, 90, 10], "worlds": [1, 2, 2, 1]}
        self.assertEqual(league.ranks(result), [3, 1, 1, 4])
        self.assertEqual(league.win_shares(result), [0.0, 0.5, 0.5, 0.0])
        won = {"winner": 4, "production": [50, 90, 90, 10], "worlds": [1, 2, 2, 1]}
        # Behind a declared winner, losers still rank by production.
        self.assertEqual(league.ranks(won), [4, 2, 2, 1])


if __name__ == "__main__":
    unittest.main()
