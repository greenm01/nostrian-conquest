# Manuals

This directory holds the authoritative manuals for the Rust edition of
Nostrian Conquest.

The primary sources are:

- [nc_player_manual.typ](nc_player_manual.typ)
- [nc_sysop_manual.typ](nc_sysop_manual.typ)

The introductory [nc_quick_start.typ](nc_quick_start.typ) follows the player
manual; it summarizes the first turn rather than replacing the full rules.

The published outputs linked from the main README are:

- [nc_quick_start.pdf](nc_quick_start.pdf)
- [nc_player_manual.pdf](nc_player_manual.pdf)
- [nc_sysop_manual.pdf](nc_sysop_manual.pdf)

Edit the Typst sources first. When a manual changes, regenerate the matching
PDF so the published output stays in sync.

## Writing Style

Future manual edits should follow
[project-style-guide.md](project-style-guide.md).

In short:

- use standard prose
- keep the manuals human-facing
- prefer short clear paragraphs
- do not be more verbose than the material requires

## Maintenance

Use the revision-date helper from the repo root:

```bash
python3 scripts/refresh_manual_revision_date.py
python3 scripts/refresh_manual_revision_date.py --doc player
python3 scripts/refresh_manual_revision_date.py --doc sysop
python3 scripts/refresh_manual_revision_date.py --doc quick-start
python3 scripts/refresh_manual_revision_date.py --doc all
python3 scripts/refresh_manual_revision_date.py --doc both --date 2026-03-28
python3 scripts/refresh_manual_revision_date.py --doc player --no-build
```

The default and `--doc both` select the player and sysop manuals; `--doc all`
also selects the quick-start guide. The script updates each selected Typst source and rebuilds the
matching PDF. Pass `--no-build` when you only want to refresh the Typst source.

## Archive

The [archive/](archive/) subdirectory holds historical transcriptions and
reference material derived from the original game documents. Treat that archive
as provenance and ambiguity fallback, not as the primary source for current
manual edits.
