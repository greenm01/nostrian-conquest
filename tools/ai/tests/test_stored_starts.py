import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from stored_starts import latest_copies, seat_starts  # noqa: E402


class StoredStartsTest(unittest.TestCase):
    def test_a_fallen_seat_starts_in_the_years_before_its_fall(self):
        years = {3000 + y: {1: 2 if y < 8 else 0, 2: 3} for y in range(10)}
        starts = seat_starts(years, 2, before=3, every=4)
        self.assertEqual([y for s, y in starts if s == 0], [3005, 3006, 3007])
        self.assertEqual([y for s, y in starts if s == 1], [3000, 3004, 3008])

    def test_a_seat_that_never_held_a_world_gives_no_starts(self):
        years = {3000: {1: 1}, 3001: {1: 1}}
        self.assertEqual(seat_starts(years, 2, before=3, every=1), [(0, 3000), (0, 3001)])

    def test_the_newest_dated_copy_of_a_game_wins(self):
        paths = ["h/crunch-ai-2026-10-30.db", "h/crunch-ai-2026-11-02.db", "h/crunch-fresh-2026-10-30.db"]
        self.assertEqual([p.name for p in latest_copies(paths)],
                         ["crunch-ai-2026-11-02.db", "crunch-fresh-2026-10-30.db"])


if __name__ == "__main__":
    unittest.main()
