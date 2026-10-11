import unittest

from surprise_memory_review import controls, unseen_probability


class SurpriseMemoryTests(unittest.TestCase):
    def test_ten_cruisers_guarded_in_orbit(self):
        p, unseen = unseen_probability(ticks=52, factor=5, layer=1, ca=10)
        self.assertAlmostEqual(p, (300 ** .5) * 25 / 5200)
        self.assertAlmostEqual(unseen, .01087712, places=7)

    def test_scout_visibility_has_no_orbit_multiplier(self):
        self.assertEqual(unseen_probability(ticks=52, factor=5, layer=0, sc=1),
                         unseen_probability(ticks=52, factor=5, layer=1, sc=1))

    def test_more_scans_reduce_unseen_probability(self):
        rows = controls()['rows']
        for start in range(0, len(rows), 3):
            probabilities = [r['unsighted_holdover_probability'] for r in rows[start:start+3]]
            self.assertGreater(probabilities[0], probabilities[1])
            self.assertGreater(probabilities[1], probabilities[2])

    def test_zero_scans_preserve_unknown(self):
        self.assertEqual(unseen_probability(ticks=0, factor=5, layer=1, dd=1)[1], 1)


if __name__ == '__main__':
    unittest.main()
