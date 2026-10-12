#!/usr/bin/env python3
"""Enter one Discord secret without shell history, echo, or chat exposure."""
import argparse
import getpass
import os
from pathlib import Path
import sys
import tempfile

FIELDS = {
    "bot": ("discord.env", "NC_DISCORD_BOT_TOKEN", "Discord bot token"),
    "oauth": ("web.env", "NC_DISCORD_CLIENT_SECRET", "Discord OAuth client secret"),
}


def save_secret(directory, field, value):
    filename, key, _ = FIELDS[field]
    value = value.strip()
    if not value or any(char.isspace() or ord(char) < 32 for char in value):
        raise ValueError("Paste the credential alone, with no spaces or line breaks")
    directory = Path(directory)
    directory.mkdir(mode=0o700, parents=True, exist_ok=True)
    target = directory / filename
    if target.is_symlink():
        raise ValueError("Refusing a symlinked credential file")
    lines = target.read_text().splitlines() if target.exists() else []
    lines = [line for line in lines if not line.lstrip().startswith(key + "=")]
    escaped = value.replace("\\", "\\\\").replace('"', '\\"')
    lines.append(f'{key}="{escaped}"')
    fd, name = tempfile.mkstemp(prefix=".credential-", dir=directory)
    try:
        with os.fdopen(fd, "w") as output:
            os.fchmod(output.fileno(), 0o600)
            output.write("\n".join(lines) + "\n")
            output.flush()
            os.fsync(output.fileno())
        os.replace(name, target)
    finally:
        if os.path.exists(name):
            os.unlink(name)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("field", choices=FIELDS)
    args = parser.parse_args()
    if os.geteuid() != 0:
        parser.error("run through sudo")
    if not sys.stdin.isatty():
        parser.error("an attended terminal is required; use ssh -t")
    while True:
        value = getpass.getpass(FIELDS[args.field][2] + " (input hidden): ")
        try:
            save_secret("/etc/nc-community", args.field, value)
            break
        except ValueError as error:
            print(str(error) + ". Try again.", file=sys.stderr)
    print("Credential saved privately. No service was started or restarted.")


if __name__ == "__main__":
    main()
