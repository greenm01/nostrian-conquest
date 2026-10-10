"""Hand-calculated boundaries and accounting, not engine conformance."""

import random
import unittest

from analyze import classic_step, fractional_step, nc_step, path


class EconomyArithmetic(unittest.TestCase):
    def test_nc_new_colony_gets_post_growth_income(self):
        self.assertEqual(nc_step(1, 100, 50), (14, 7))

    def test_nc_seventy_percent_fixed_point(self):
        self.assertEqual(nc_step(100, 100, 70), (99, 69))
        self.assertEqual(nc_step(99, 100, 70), (99, 69))

    def test_nc_penalty_uses_old_production(self):
        # Growth crosses the 50-P boundary; penalty must still be one.
        self.assertEqual(nc_step(49, 100, 75), (52, 39))

    def test_nc_base_ceiling_doubles_one_point_growth(self):
        self.assertEqual(nc_step(98, 100, 50, True), (100, 50))
        self.assertEqual(nc_step(98, 100, 50, False), (99, 49))

    def test_nc_small_world_can_earn_nothing(self):
        self.assertEqual(nc_step(0, 10, 10), (3, 0))

    def test_fractional_income_is_carried(self):
        rows = path(fractional_step, 0, [10, 10], potential=10)
        self.assertAlmostEqual(rows[0]["income"], 0.225)
        self.assertAlmostEqual(rows[1]["treasury"], 0.624375)

    def test_fractional_equilibrium(self):
        p = 100 * 0.075 / 0.085
        self.assertAlmostEqual(fractional_step(p, 100, 70)[0], p)

    def test_classic_zero_tax_and_bounds(self):
        rng = random.Random(20261010)
        production = 1
        for _ in range(20):
            production, income = classic_step(production, 100, 0, rng)
            self.assertEqual(income, 0)
            self.assertTrue(1 <= production <= 100)
        self.assertEqual(production, 100)


if __name__ == "__main__":
    unittest.main()
