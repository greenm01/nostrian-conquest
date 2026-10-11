"""Integrity checks for the campaign evidence reader."""
import tempfile
import unittest
from pathlib import Path
import summarize


class EvidenceTests(unittest.TestCase):
    def write(self, folder, rows):
        p = Path(folder) / "run.tsv"
        p.write_text("\n".join("\t".join(row) for row in rows) + "\n")
        return str(p)

    def rows(self):
        game = dict.fromkeys(summarize.G_COLUMNS, "0")
        game.update(kind="G", label="base", prices="5/15/45", goods="50", players="2",
                    opening="escort", doctrine="value", seed="7", years="80", winner="-", winner_year="-")
        empire = dict.fromkeys(summarize.E_COLUMNS, "0")
        empire.update(game)
        empire.update(kind="E", empire_opening="escort", empire_doctrine="value", first_colony="2")
        rows = [[game[c] for c in summarize.G_COLUMNS]]
        for seat in (1, 2):
            empire["empire"] = str(seat)
            rows.append([empire[c] for c in summarize.E_COLUMNS])
        return rows

    def test_complete_campaign_has_one_sample_and_two_seats(self):
        with tempfile.TemporaryDirectory() as d:
            result = summarize.compact_summary([self.write(d, self.rows())])
        self.assertEqual((result["campaigns"], result["empires"]), (1, 2))
        self.assertEqual(result["cells"][0]["game"]["winner_year"], {"mean": None, "observed": 0})
        self.assertEqual(result["cells"][0]["empire"]["first_colony"]["mean"], 2)

    def test_truncated_row_is_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            with self.assertRaises(ValueError):
                summarize.parse([self.write(d, [["G", "base"]])])

    def test_missing_seat_is_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            with self.assertRaises(ValueError):
                summarize.compact_summary([self.write(d, self.rows()[:-1])])

    def test_duplicate_campaign_is_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            with self.assertRaises(ValueError):
                summarize.compact_summary([self.write(d, self.rows() * 2)])


if __name__ == "__main__":
    unittest.main()
