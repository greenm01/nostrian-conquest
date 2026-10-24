import tempfile
import unittest
from pathlib import Path

import check_note_links


class DeadLinksTests(unittest.TestCase):
    def test_reports_only_missing_relative_targets(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "docs/notes").mkdir(parents=True)
            (root / "rust").mkdir()
            (root / "rust/lib.rs").write_text("")
            note = root / "docs/notes/n.md"
            note.write_text(
                "[code](../../rust/lib.rs#L3) [web](https://x.org) [here](#top)\n"
                "[gone](../../rust/old.rs) `[inline](nope.md)`\n"
                "```\n[fenced](nope.md)\n```\n"
            )
            self.assertEqual(check_note_links.dead_links(note), ["../../rust/old.rs"])
            (root / "todo.md").write_text("[ok](docs/notes/n.md)\n")
            self.assertEqual(check_note_links.main(root), 1)


if __name__ == "__main__":
    unittest.main()
