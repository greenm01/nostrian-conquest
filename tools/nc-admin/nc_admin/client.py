"""Bounded JSON subprocess protocol; no shell or direct game database writes."""
from __future__ import annotations
import json
import subprocess
from pathlib import Path


class AdminError(Exception):
    pass


class Client:
    def __init__(self, root: Path, binary: str, timeout: float = 300):
        self.root, self.binary, self.timeout = root, binary, timeout

    def call(self, request: dict) -> object:
        try:
            proc = subprocess.run(
                [self.binary, "admin", "--root", str(self.root)],
                input=json.dumps(request), text=True, capture_output=True,
                timeout=self.timeout, check=False,
            )
        except (OSError, subprocess.TimeoutExpired) as exc:
            raise AdminError(f"host command did not finish: {exc}; inspect before retrying") from exc
        try:
            reply = json.loads(proc.stdout)
        except (ValueError, TypeError) as exc:
            raise AdminError("host returned invalid JSON; check the installed nc-host version") from exc
        if not isinstance(reply, dict) or reply.get("version") != 1:
            raise AdminError("unsupported host response version")
        if proc.returncode or reply.get("ok") is not True:
            raise AdminError(str(reply.get("error", "host command failed")))
        return reply.get("data")
