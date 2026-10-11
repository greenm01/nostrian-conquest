#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
import shutil
import subprocess
from dataclasses import dataclass
from datetime import date, datetime
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
MANUALS_DIR = REPO_ROOT / "docs" / "manuals"


@dataclass(frozen=True)
class ManualSpec:
    key: str
    label: str
    typ_path: Path
    pdf_path: Path


MANUALS = {
    "quick-start": ManualSpec(
        key="quick-start",
        label="quick-start",
        typ_path=MANUALS_DIR / "nc_quick_start.typ",
        pdf_path=MANUALS_DIR / "nc_quick_start.pdf",
    ),
    "player": ManualSpec(
        key="player",
        label="player",
        typ_path=MANUALS_DIR / "nc_player_manual.typ",
        pdf_path=MANUALS_DIR / "nc_player_manual.pdf",
    ),
    "sysop": ManualSpec(
        key="sysop",
        label="sysop",
        typ_path=MANUALS_DIR / "nc_sysop_manual.typ",
        pdf_path=MANUALS_DIR / "nc_sysop_manual.pdf",
    ),
}

DATE_METADATA_RE = re.compile(
    r"(?m)^(\s*date:\s*)datetime\(year:\s*\d{4}, month:\s*\d{1,2}, day:\s*\d{1,2}\)(,?)$"
)
REVISION_DATE_RE = re.compile(r"Revision date: [A-Za-z]+ \d{1,2}, \d{4}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Refresh the revision date in the selected Typst guides and, by "
            "default, rebuild the matching PDF outputs."
        )
    )
    parser.add_argument(
        "--doc",
        choices=("player", "sysop", "quick-start", "both", "all"),
        default="both",
        help="Which guide to update. Both means player and sysop; all includes quick-start.",
    )
    parser.add_argument(
        "--date",
        type=parse_iso_date,
        default=date.today(),
        help="Revision date to apply in YYYY-MM-DD format. Defaults to today.",
    )
    parser.add_argument(
        "--no-build",
        action="store_true",
        help="Update the Typst source(s) only; skip PDF rebuild.",
    )
    return parser.parse_args()


def parse_iso_date(value: str) -> date:
    try:
        return datetime.strptime(value, "%Y-%m-%d").date()
    except ValueError as err:
        raise argparse.ArgumentTypeError(
            f"invalid date '{value}'; expected YYYY-MM-DD"
        ) from err


def target_specs(doc_arg: str) -> list[ManualSpec]:
    if doc_arg == "all":
        return list(MANUALS.values())
    if doc_arg == "both":
        return [MANUALS["player"], MANUALS["sysop"]]
    return [MANUALS[doc_arg]]


def format_revision_date(target_date: date) -> str:
    return f"{target_date.strftime('%B')} {target_date.day}, {target_date.year}"


def ensure_single_match(pattern: re.Pattern[str], text: str, desc: str, path: Path) -> None:
    matches = list(pattern.finditer(text))
    if len(matches) != 1:
        raise SystemExit(
            f"{path}: expected exactly one {desc} match, found {len(matches)}"
        )


def update_typ_source(spec: ManualSpec, target_date: date) -> bool:
    if not spec.typ_path.is_file():
        raise SystemExit(f"missing manual source: {spec.typ_path}")

    original = spec.typ_path.read_text(encoding="utf-8")
    ensure_single_match(DATE_METADATA_RE, original, "Typst document date", spec.typ_path)
    ensure_single_match(REVISION_DATE_RE, original, "visible revision date", spec.typ_path)

    updated = DATE_METADATA_RE.sub(
        lambda m: (
            f"{m.group(1)}datetime(year: {target_date.year}, "
            f"month: {target_date.month}, day: {target_date.day}){m.group(2)}"
        ),
        original,
        count=1,
    )
    updated = REVISION_DATE_RE.sub(
        f"Revision date: {format_revision_date(target_date)}",
        updated,
        count=1,
    )

    if updated == original:
        return False

    spec.typ_path.write_text(updated, encoding="utf-8")
    return True


def build_pdf(spec: ManualSpec) -> None:
    if shutil.which("typst") is None:
        raise SystemExit(
            "typst is required to rebuild manual PDFs. Install it or rerun with --no-build."
        )
    subprocess.run(
        ["typst", "compile", str(spec.typ_path), str(spec.pdf_path)],
        cwd=REPO_ROOT,
        check=True,
        text=True,
    )


def main() -> int:
    args = parse_args()
    specs = target_specs(args.doc)

    updated_specs: list[ManualSpec] = []
    unchanged_specs: list[ManualSpec] = []
    built_specs: list[ManualSpec] = []

    for spec in specs:
        changed = update_typ_source(spec, args.date)
        if changed:
            updated_specs.append(spec)
        else:
            unchanged_specs.append(spec)

    if not args.no_build:
        for spec in specs:
            build_pdf(spec)
            built_specs.append(spec)

    print(f"Applied revision date: {args.date.isoformat()}")
    if updated_specs:
        print("Updated sources:")
        for spec in updated_specs:
            print(f"  {spec.typ_path}")
    if unchanged_specs:
        print("Unchanged sources:")
        for spec in unchanged_specs:
            print(f"  {spec.typ_path}")
    if built_specs:
        print("Rebuilt PDFs:")
        for spec in built_specs:
            print(f"  {spec.pdf_path}")
    else:
        print("Skipped PDF rebuild.")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
