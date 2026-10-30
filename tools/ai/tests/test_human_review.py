import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from human_review import compare, turn_orders  # noqa: E402

TURNS = [
    'turn player=2 year=3044\ntax rate=40\nfleet record=3 {\n  order speed=6 kind="hold"\n}',
    'turn player=1 year=3044\ntax rate=65\nfleet record=9 {\n'
    '  order speed=6 kind="patrol" x=10 y=15\n  roe value=5\n}\nfleet record=18 {\n  roe value=0\n}',
    'turn player=1 year=3044\nfleet record=9 {\n  order speed=3 kind="bombard" x=7 y=8\n}',
]


class HumanReviewTest(unittest.TestCase):
    def test_later_turns_override_and_other_players_are_ignored(self):
        tax, orders = turn_orders(TURNS, 1)
        self.assertEqual(tax, 65)
        self.assertEqual(orders, {9: ("bombard", 7, 8)})

    def test_fleets_are_kept_given_the_same_order_or_changed(self):
        stored = {9: ("bombard", 7, 8, 43, 1), 18: ("hold", 9, 9, 1, 2), 20: ("join", 9, 9, 2, 3)}
        agent = {9: ("bombard", 7, 8), 20: ("rendezvous", 9, 9)}
        self.assertEqual(compare(stored, agent), ([18], [9], [20]))


if __name__ == "__main__":
    unittest.main()
