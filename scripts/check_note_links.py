#!/usr/bin/env python3
"""Report relative Markdown links in the work-tracking files whose target is missing.

`zk list --broken-links` also flags working links to code, data and manuals,
because zk only resolves links to its own notes; this checks the file system.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LINK = re.compile(r"\]\(<?([^)\s>]+)>?(?:\s+\"[^\"]*\")?\)")
CODE = re.compile(r"```.*?```|`[^`\n]*`", re.DOTALL)


def tracked_files(root: Path) -> list[Path]:
    return [
        *sorted((root / "docs/notes").rglob("*.md")),
        root / "todo.md",
        *sorted(root.glob("done-*.md")),
    ]


def dead_links(path: Path) -> list[str]:
    text = CODE.sub("", path.read_text(encoding="utf-8"))
    dead = []
    for target in LINK.findall(text):
        target = target.split("#", 1)[0]
        if not target or re.match(r"[a-z][a-z0-9+.-]*:", target):
            continue
        if not (path.parent / target).exists():
            dead.append(target)
    return dead


def main(root: Path = ROOT) -> int:
    count = 0
    for path in tracked_files(root):
        for target in dead_links(path):
            print(f"{path.relative_to(root)}: {target}")
            count += 1
    return 1 if count else 0


if __name__ == "__main__":
    sys.exit(main())
