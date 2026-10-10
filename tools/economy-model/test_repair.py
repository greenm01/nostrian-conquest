import unittest
from repair import repair_batch, repair_installments
from campaign import simulate


class Repairs(unittest.TestCase):
    def test_installments_finish_a_bb_at_a_capacity_ten_world(self):
        left, bill = repair_installments([(45, 0)], 10)
        self.assertEqual((left, bill), ([(45, 40)], 10))
        self.assertEqual(repair_installments(left, 10), ([], 2))

    def test_installments_aggregate_and_continue_to_the_next_hull(self):
        self.assertEqual(repair_installments([(5, 0)] * 4, 5), ([], 5))
        self.assertEqual(repair_installments([(15, 0), (45, 0)], 12), ([(15, 3)], 12))

    def test_aggregate_rounding_and_budget(self):
        self.assertEqual(repair_batch([5]*4, 5), ([], 5))
        self.assertEqual(repair_batch([15, 45], 15), ([], 15))
        self.assertEqual(repair_batch([45, 15], 10), ([45], 4))

    def test_capacity_can_make_a_whole_hull_ineligible_forever(self):
        self.assertEqual(repair_batch([45], 10), ([45], 0))

    def test_repair_first_delays_extra_colonizers_not_starter_ships(self):
        repair = simulate(1, colonizers=4, repair_costs=[15]*9)
        build = simulate(1, colonizers=4, repair_costs=[15]*9, repair_first=False)
        self.assertEqual(repair['repair'][0], dict(year=1, bill=34, damaged=0, etacs=2))
        self.assertEqual(build['repair'][0], dict(year=1, bill=8, damaged=7, etacs=4))
        self.assertEqual(repair['repair'][1]['etacs'], 4)
