import unittest
from unittest.mock import patch

from campaign import distribution, sampled_map, simulate, travel_ticks, travel_years


class CampaignAccounting(unittest.TestCase):
    def test_depth_cost_and_speed(self):
        self.assertEqual(travel_ticks((0, 0), (3, 0), 3), 87)
        self.assertEqual(travel_years((0, 0), (3, 0), 3), 2)
        self.assertEqual(travel_years((0, 0), (3, 0), 1), 5)

    def test_seeded_map_unique_valid_and_reproducible(self):
        worlds = sampled_map(91, 24)
        self.assertEqual(worlds, sampled_map(91, 24))
        self.assertEqual(len({coord for coord, _ in worlds}), 24)
        self.assertNotIn((16, 16), {coord for coord, _ in worlds})
        self.assertTrue(all(10 <= potential <= 150 for _, potential in worlds))

    @patch("campaign.sampled_map", return_value=[((17, 16), 100)])
    def test_colony_does_not_collect_on_arrival_and_needs_capacity(self, _):
        run = simulate(1)
        self.assertEqual(run["milestones"]["first_colony"], 1)
        self.assertEqual(run["milestones"]["first_colony_can_order_CA"], 4)
        self.assertEqual(run["milestones"]["first_colony_can_order_BB30"], 5)
        self.assertEqual(run["milestones"]["first_colony_can_order_BB45"], 7)
        self.assertEqual(run["snapshots"][25]["colonies"], 1)

    @patch("campaign.sampled_map", return_value=[])
    def test_extra_colonizers_are_paid_for(self, _):
        baseline = simulate(1)["snapshots"][5]
        extra = simulate(1, colonizers=4)["snapshots"][5]
        self.assertEqual(baseline["local_reserves_sum"] - extra["local_reserves_sum"], 40)

    def test_unreached_milestone_not_discarded_from_denominator(self):
        row = distribution([None, 3, 7, None], 4)
        self.assertEqual(row["reach_fraction"], 0.5)
        self.assertEqual(row["median_when_reached"], 5)


if __name__ == "__main__":
    unittest.main()
