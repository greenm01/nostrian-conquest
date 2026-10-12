#!/usr/bin/env python3
"""Build the standard-library-only nc-admin executable zipapp."""
import argparse
import shutil
import tempfile
import zipapp
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("output", type=Path)
args = parser.parse_args()
with tempfile.TemporaryDirectory(prefix="nc-admin-package-") as directory:
    stage = Path(directory)
    shutil.copytree(Path(__file__).parent / "nc_admin", stage / "nc_admin", ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
    (stage / "__main__.py").write_text("from nc_admin.cli import main\nraise SystemExit(main())\n")
    zipapp.create_archive(stage, args.output, interpreter="/usr/bin/env python3", compressed=True)
