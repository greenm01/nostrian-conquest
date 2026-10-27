import sys
import unittest
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from value_trace import falls  # noqa: E402


class FallsTest(unittest.TestCase):
    def test_largest_drops_first_and_ended_games_skipped(self):
        nan = np.nan
        # [years + 1, games]: game 0 falls 0.4 over year 1, game 1 0.2 over
        # year 0, game 2 ends after year 0 and never counts.
        values = np.array([[0.5, 0.6, 0.9],
                           [0.6, 0.4, nan],
                           [0.2, 0.5, nan]])
        self.assertEqual(falls(values, 5), [(0, 1), (1, 0), (1, 1), (0, 0)])
        self.assertEqual(falls(values, 1), [(0, 1)])


if __name__ == "__main__":
    unittest.main()
