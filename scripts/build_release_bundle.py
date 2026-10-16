#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
import re
import uuid


REPO_ROOT = Path(__file__).resolve().parents[1]
RUST_ROOT = REPO_ROOT / "rust"
RELEASES_DIR = REPO_ROOT / "releases"
PLAYER_MANUAL = REPO_ROOT / "docs" / "manuals" / "nc_player_manual.pdf"
QUICK_START = REPO_ROOT / "docs" / "manuals" / "nc_quick_start.pdf"
SYSOP_MANUAL = REPO_ROOT / "docs" / "manuals" / "nc_sysop_manual.pdf"
SYSOP_DOCS_ROOT = REPO_ROOT / "docs" / "sysop"
SYSOP_ASSET_ROOT = REPO_ROOT / "packaging" / "sysop"
SYSOP_CONFIG_EXAMPLE = SYSOP_ASSET_ROOT / "examples" / "config.kdl"
SYSOP_LINUX_README = SYSOP_ASSET_ROOT / "linux" / "README.md"
SYSOP_WINDOWS_README = SYSOP_ASSET_ROOT / "windows" / "README.md"
EC_CONNECT_LICENSES = (
    REPO_ROOT / "rust" / "nc-connect" / "assets" / "licenses" / "OFL-0xProto.txt",
    REPO_ROOT / "rust" / "nc-connect" / "assets" / "licenses" / "OFL-JetBrainsMono.txt",
    REPO_ROOT / "rust" / "nc-connect" / "assets" / "licenses" / "LICENSE-NotoSansMono.txt",
)
FORBIDDEN_CLASSIC_PACKAGE_NAMES = {
    "ECGAME.EXE",
    "ECMAINT.EXE",
    "ECUTIL.EXE",
    "ECREADME.DOC",
    "ECPLAYER.DOC",
    "ECQSTART.DOC",
    "WHATSNEW.DOC",
}
FORBIDDEN_CLASSIC_PACKAGE_DIRS = {"original", "NC_UNLOCKED"}


@dataclass(frozen=True)
class TargetPlatform:
    target_triple: str
    slug: str
    display_name: str
    issue_platform_label: str


SUPPORTED_TARGETS = {
    "x86_64-unknown-linux-gnu": TargetPlatform(
        target_triple="x86_64-unknown-linux-gnu",
        slug="linux-x64",
        display_name="Linux x64",
        issue_platform_label="Linux distro",
    ),
    "aarch64-apple-darwin": TargetPlatform(
        target_triple="aarch64-apple-darwin",
        slug="macos-arm64",
        display_name="macOS Apple Silicon",
        issue_platform_label="macOS version",
    ),
    "x86_64-apple-darwin": TargetPlatform(
        target_triple="x86_64-apple-darwin",
        slug="macos-x64",
        display_name="macOS Intel",
        issue_platform_label="macOS version",
    ),
    "x86_64-pc-windows-msvc": TargetPlatform(
        target_triple="x86_64-pc-windows-msvc",
        slug="windows-x64",
        display_name="Windows x64",
        issue_platform_label="Windows version",
    ),
    "i686-pc-windows-msvc": TargetPlatform(
        target_triple="i686-pc-windows-msvc",
        slug="windows-x86",
        display_name="Windows x86 (32-bit)",
        issue_platform_label="Windows version",
    ),
    "i686-win7-windows-msvc": TargetPlatform(
        target_triple="i686-win7-windows-msvc",
        slug="windows7-x86",
        display_name="Windows 7+ x86 (32-bit)",
        issue_platform_label="Windows version",
    ),
}


@dataclass(frozen=True)
class BundleSpec:
    version: str
    platform: TargetPlatform
    artifact: str

    @property
    def bundle_root_name(self) -> str:
        if self.artifact == "nc-connect":
            return f"nc-connect-v{self.version}-{self.platform.slug}"
        if self.artifact == "sysop":
            return f"nc-sysop-v{self.version}-{self.platform.slug}"
        return f"nostrian-conquest-v{self.version}-{self.platform.slug}"

    @property
    def is_windows(self) -> bool:
        return self.platform.target_triple.endswith("windows-msvc")

    @property
    def builds_windows_std(self) -> bool:
        return self.is_windows and self.platform.target_triple.startswith("i686-")

    @property
    def archive_name(self) -> str:
        ext = "zip" if self.is_windows else "tar.gz"
        return f"{self.bundle_root_name}.{ext}"


def parse_args(default_target: str | None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Build release archives for internal beta, public player, or public sysop packaging."
        )
    )
    parser.add_argument(
        "--artifact",
        choices=("public-beta", "nc-connect", "sysop"),
        default="public-beta",
        help=(
            "Artifact type to package. `public-beta` keeps the internal combined "
            "bundle; `nc-connect` builds the public player archive; `sysop` "
            "builds the public BBS/sysop archive."
        ),
    )
    parser.add_argument(
        "--target",
        choices=sorted(SUPPORTED_TARGETS),
        default=default_target,
        help=(
            "Rust target triple to package. Defaults to the current host target "
            "when omitted."
        ),
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=RELEASES_DIR,
        help="Directory where the generated tar.gz will be written.",
    )
    parser.add_argument(
        "--verify",
        action="store_true",
        help="Unpack and verify the generated public beta bundle after building it.",
    )
    return parser.parse_args()


def load_version() -> str:
    cargo_toml = (RUST_ROOT / "nc-game" / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r'(?m)^version = "([^"]+)"$', cargo_toml)
    if match is None:
        raise SystemExit("could not parse version from rust/nc-game/Cargo.toml")
    return match.group(1)


def run(
    argv: list[str],
    *,
    cwd: Path | None = None,
    capture_output: bool = False,
    extra_env: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[str]:
    env = dict(os.environ)
    env["RUSTC_WRAPPER"] = ""
    if extra_env:
        env.update(extra_env)
    return subprocess.run(
        argv,
        cwd=cwd or REPO_ROOT,
        check=True,
        text=True,
        capture_output=capture_output,
        env=env,
    )


def capture(argv: list[str], *, cwd: Path | None = None) -> str:
    return run(argv, cwd=cwd, capture_output=True).stdout.strip()


def detect_host_target() -> str:
    rustc_verbose = capture(["rustc", "-vV"])
    for line in rustc_verbose.splitlines():
        if line.startswith("host: "):
            return line.removeprefix("host: ").strip()
    raise SystemExit("unable to determine rustc host target from `rustc -vV`")


def resolve_target(target_triple: str | None) -> TargetPlatform:
    selected = target_triple or detect_host_target()
    try:
        return SUPPORTED_TARGETS[selected]
    except KeyError as err:
        supported = ", ".join(sorted(SUPPORTED_TARGETS))
        raise SystemExit(
            f"unsupported target '{selected}'. supported targets: {supported}"
        ) from err


def artifact_binaries(spec: BundleSpec) -> tuple[str, ...]:
    if spec.artifact == "nc-connect":
        return ("nc-connect",)
    if spec.artifact == "sysop":
        return ("nc-door", "nc-sysop")
    return ("nc-game", "nc-door", "nc-sysop", "nc-connect")


def artifact_packages(spec: BundleSpec) -> tuple[str, ...]:
    if spec.artifact == "nc-connect":
        return ("nc-connect",)
    if spec.artifact == "sysop":
        return ("nc-game", "nc-sysop")
    return ("nc-game", "nc-sysop", "nc-connect")


def validate_artifact_platform(spec: BundleSpec) -> None:
    if spec.platform.target_triple in (
        "i686-pc-windows-msvc",
        "i686-win7-windows-msvc",
    ) and spec.artifact != "sysop":
        raise SystemExit(
            f"{spec.platform.target_triple} packaging is currently supported only "
            "for the public sysop archive"
        )
    if spec.artifact == "sysop" and spec.platform.target_triple not in (
        "x86_64-unknown-linux-gnu",
        "x86_64-pc-windows-msvc",
        "i686-pc-windows-msvc",
        "i686-win7-windows-msvc",
    ):
        raise SystemExit(
            "sysop packaging is only supported for x86_64-unknown-linux-gnu, "
            "x86_64-pc-windows-msvc, i686-pc-windows-msvc, and "
            "i686-win7-windows-msvc"
        )


def build_command(spec: BundleSpec, packages: tuple[str, ...]) -> list[str]:
    command = ["cargo"]
    if spec.builds_windows_std:
        command.extend(["+nightly", "build", "-Z", "build-std=std,panic_abort"])
    else:
        command.append("build")
    command.extend(["--release", "--target", spec.platform.target_triple])
    command.extend(arg for name in packages for arg in ("-p", name))
    return command


def cargo_home() -> Path:
    if "CARGO_HOME" in os.environ:
        return Path(os.environ["CARGO_HOME"])
    return Path.home() / ".cargo"


def find_windows_i686_link_dir() -> Path:
    registry_src = cargo_home() / "registry" / "src"
    candidates = sorted(
        path
        for path in registry_src.glob("*/*")
        if path.name.startswith("windows_i686_msvc-") and (path / "lib").is_dir()
    )
    if not candidates:
        raise SystemExit(
            "Windows x86 packaging requires the windows_i686_msvc crate "
            "to be present in the local Cargo registry cache."
        )
    return candidates[-1] / "lib"


def build_binaries(spec: BundleSpec) -> dict[str, Path]:
    validate_artifact_platform(spec)
    binaries = artifact_binaries(spec)
    packages = artifact_packages(spec)
    host_target = detect_host_target()
    if spec.is_windows and not host_target.endswith("-pc-windows-msvc"):
        raise SystemExit(
            "Windows release bundles must be built on a native Windows MSVC "
            "host. GNU and non-Windows cross-builds are "
            "not supported for release packaging."
        )
    extra_env: dict[str, str] = {}
    if spec.builds_windows_std:
        link_dir = find_windows_i686_link_dir()
        rustflags = os.environ.get("RUSTFLAGS", "").strip()
        link_flag = f"-Lnative={link_dir}"
        extra_env["RUSTFLAGS"] = f"{rustflags} {link_flag}".strip()
    command = build_command(spec, packages)
    command.append("--message-format=json-render-diagnostics")
    try:
        result = run(command, cwd=RUST_ROOT, extra_env=extra_env, capture_output=True)
    except subprocess.CalledProcessError as error:
        sys.stderr.write(error.stderr or "")
        raise
    sys.stderr.write(result.stderr)
    paths: dict[str, Path] = {}
    for line in result.stdout.splitlines():
        message = json.loads(line)
        if message.get("reason") != "compiler-artifact" or not message.get("executable"):
            continue
        name = message["target"]["name"]
        if name in binaries:
            paths[name] = Path(message["executable"])
    missing = set(binaries) - paths.keys()
    if missing:
        raise SystemExit(f"Cargo did not report release binaries: {', '.join(sorted(missing))}")
    return paths


def build_info_text(spec: BundleSpec) -> str:
    commit = capture(["git", "rev-parse", "HEAD"])
    short_commit = capture(["git", "rev-parse", "--short", "HEAD"])
    cargo_toolchain = "nightly" if spec.builds_windows_std else "default"
    rustc_command = ["rustc", "+nightly", "-V"] if spec.builds_windows_std else ["rustc", "-V"]
    rustc = capture(rustc_command)
    built_at = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    lines = [
        f"artifact={spec.artifact}",
        f"version={spec.version}",
        f"git_commit={commit}",
        f"git_commit_short={short_commit}",
        f"target={spec.platform.target_triple}",
        f"cargo_toolchain={cargo_toolchain}",
        f"built_at_utc={built_at}",
        f"rustc={rustc}",
    ]
    if spec.builds_windows_std:
        lines.append("build_std=std,panic_abort")
    return "\n".join(lines) + "\n"


def package_readme(spec: BundleSpec) -> str:
    if spec.artifact == "sysop":
        readme_path = SYSOP_WINDOWS_README if spec.is_windows else SYSOP_LINUX_README
        return readme_path.read_text(encoding="utf-8")

    connect_binary = "nc-connect.exe" if spec.is_windows else "./bin/nc-connect"
    player_manual_path = "nc_player_manual.pdf" if spec.is_windows else "docs/nc_player_manual.pdf"
    windows_note = ""
    if spec.is_windows:
        windows_note = """
## Windows

Extract the `.zip` archive to a folder of your choice. Double-click `nc-connect.exe`
to launch. No installation required.

If Windows Defender flags the binary, click "More info" → "Run anyway".
""".rstrip()

    macos_quarantine_note = ""
    if spec.platform.target_triple.endswith("-apple-darwin"):
        if spec.artifact == "nc-connect":
            binary_list = "./bin/nc-connect"
            descriptor = "a standalone GUI binary"
        else:
            binary_list = "./bin/nc-game ./bin/nc-door ./bin/nc-sysop ./bin/nc-connect"
            descriptor = "command-line binaries"
        macos_quarantine_note = f"""
## macOS First Run Note

These are {descriptor}, not bundled `.app` applications. If macOS
blocks them after download, remove the quarantine attribute from the unpacked
bundle root:

```bash
xattr -d com.apple.quarantine {binary_list}
```
""".rstrip()

    if spec.artifact == "nc-connect":
        return f"""# nc-connect {spec.platform.display_name}

This archive contains the public player client for {spec.platform.display_name}.

It includes only original Nostrian Conquest client files. No preserved Esterian
Conquest binaries, manuals, or DOS assets are bundled here.

It contains:

- `{connect_binary}`
- `{player_manual_path}`
- `licenses/OFL-0xProto.txt`
- `licenses/OFL-JetBrainsMono.txt`
- `licenses/LICENSE-NotoSansMono.txt`
- `BUILD-INFO.txt` with version/build metadata

## Quick Start

Join a hosted game with the invite code from your sysop:

```bash
{connect_binary}
```

Then press `N` in the app and paste the raw invite code:

```text
amber-river@relay.example.com
```

The player manual PDF in `{player_manual_path}` is the companion manual for this binary.
{macos_quarantine_note}{windows_note}

## Bug Reports

When reporting a player-client issue, include:

- the version and commit from `BUILD-INFO.txt`
- your {spec.platform.issue_platform_label}
- whether you were on X11, Wayland, Finder, Explorer, or a terminal launch
- the exact launch action you used
- any stderr output
- a screenshot if the issue is visual
"""

    game_binary = "nc-game.exe" if spec.is_windows else "./bin/nc-game"
    door_binary = "nc-door.exe" if spec.is_windows else "./bin/nc-door"
    sysop_binary = "nc-sysop.exe" if spec.is_windows else "./bin/nc-sysop"
    player_manual_path = "nc_player_manual.pdf" if spec.is_windows else "docs/nc_player_manual.pdf"
    sysop_manual_path = "nc_sysop_manual.pdf" if spec.is_windows else "docs/nc_sysop_manual.pdf"
    sample_game_dir = "C:\\nc\\games\\friday-night" if spec.is_windows else "/srv/ec/games/friday-night"
    sample_local_dir = "C:\\nc\\games\\local-test" if spec.is_windows else "/tmp/nc-game"

    return f"""# Nostrian Conquest {spec.platform.display_name} Public Beta Bundle

This bundle is for public beta testing on {spec.platform.display_name}.

It contains the four Rust tester binaries, `BUILD-INFO.txt`, both public PDF
manuals, and the bundled font license files. It does not contain preserved
Esterian Conquest executables, manuals, or DOS helper assets.

This is not a public release package. Public GitHub Releases publish the
player-facing `nc-connect` archives and the Windows/Linux BBS/sysop `nc-sysop`
archives. VPS hosting remains a tagged-source workflow.

## Quick Start

1. Create a fresh campaign:

```text
{sysop_binary} new-game {sample_game_dir} --name "Friday Night NC" --players 4 --seed 1515
```

2. Run maintenance:

```text
{sysop_binary} maint {sample_game_dir}
```

3. For localhost or hotseat play, launch the direct game client:

```text
{game_binary} --dir {sample_local_dir} --player 1
```

## BBS Door Note

For BBS hosting, stage `{door_binary}` as the live door binary.

Hosted Rust campaigns are DB-only. `nc-sysop new-game` creates just
`<game_dir>/ncgame.db`. The player manual lives at `{player_manual_path}` and
the sysop manual lives at `{sysop_manual_path}`.
{macos_quarantine_note}

## Bug Reports

When reporting a playtest issue, include:

- the version and commit from `BUILD-INFO.txt`
- your {spec.platform.issue_platform_label} and terminal emulator
- the exact command you ran
- any stderr output
- a screenshot if the issue is visual
"""


def copy_file(src: Path, dest: Path, *, executable: bool = False) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dest)
    if executable:
        mode = dest.stat().st_mode
        dest.chmod(mode | 0o755)


def copy_tree(src: Path, dest: Path) -> None:
    for path in src.rglob("*"):
        if path.is_file():
            copy_file(path, dest / path.relative_to(src))


def stage_bundle(spec: BundleSpec, binary_paths: dict[str, Path], workspace_root: Path) -> Path:
    bundle_root = workspace_root / spec.bundle_root_name
    if spec.artifact == "sysop":
        sysop_docs_dir = bundle_root / "docs" / "sysop"
        if spec.is_windows:
            for path in binary_paths.values():
                copy_file(path, bundle_root / path.name)
            copy_file(PLAYER_MANUAL, bundle_root / PLAYER_MANUAL.name)
            copy_file(QUICK_START, bundle_root / QUICK_START.name)
            copy_file(SYSOP_MANUAL, bundle_root / SYSOP_MANUAL.name)
            copy_file(SYSOP_CONFIG_EXAMPLE, bundle_root / "config.kdl")
        else:
            docs_dir = bundle_root / "docs"
            bin_dir = bundle_root / "bin"
            examples_dir = bundle_root / "examples"
            for name, path in binary_paths.items():
                copy_file(path, bin_dir / name, executable=True)
            copy_file(PLAYER_MANUAL, docs_dir / PLAYER_MANUAL.name)
            copy_file(QUICK_START, docs_dir / QUICK_START.name)
            copy_file(SYSOP_MANUAL, docs_dir / SYSOP_MANUAL.name)
            copy_file(SYSOP_CONFIG_EXAMPLE, examples_dir / "config.kdl")
        copy_tree(SYSOP_DOCS_ROOT, sysop_docs_dir)
    else:
        licenses_dir = bundle_root / "licenses"
        if spec.is_windows:
            for path in binary_paths.values():
                copy_file(path, bundle_root / path.name)
            copy_file(PLAYER_MANUAL, bundle_root / PLAYER_MANUAL.name)
            copy_file(QUICK_START, bundle_root / QUICK_START.name)
            if spec.artifact == "public-beta":
                copy_file(SYSOP_MANUAL, bundle_root / SYSOP_MANUAL.name)
        else:
            docs_dir = bundle_root / "docs"
            bin_dir = bundle_root / "bin"
            for name, path in binary_paths.items():
                copy_file(path, bin_dir / name, executable=True)
            copy_file(PLAYER_MANUAL, docs_dir / PLAYER_MANUAL.name)
            copy_file(QUICK_START, docs_dir / QUICK_START.name)
            if spec.artifact == "public-beta":
                copy_file(SYSOP_MANUAL, docs_dir / SYSOP_MANUAL.name)
        for license_path in EC_CONNECT_LICENSES:
            copy_file(license_path, licenses_dir / license_path.name)

    (bundle_root / "README.md").write_text(package_readme(spec), encoding="utf-8")
    (bundle_root / "BUILD-INFO.txt").write_text(build_info_text(spec), encoding="utf-8")
    return bundle_root


def write_archive(bundle_root: Path, archive_path: Path) -> None:
    archive_path.parent.mkdir(parents=True, exist_ok=True)
    if archive_path.suffix == ".zip":
        with zipfile.ZipFile(archive_path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
            for file in bundle_root.rglob("*"):
                if file.is_file():
                    zf.write(file, file.relative_to(bundle_root.parent))
    else:
        with tarfile.open(archive_path, "w:gz") as tf:
            tf.add(bundle_root, arcname=bundle_root.name)


def make_temp_workspace(prefix: str) -> Path:
    temp_root = Path(
        os.environ.get("TMPDIR")
        or os.environ.get("TEMP")
        or os.environ.get("TMP")
        or tempfile.gettempdir()
    )
    temp_root.mkdir(parents=True, exist_ok=True)
    while True:
        candidate = temp_root / f"{prefix}{uuid.uuid4().hex[:8]}"
        try:
            candidate.mkdir(parents=True, exist_ok=False)
            return candidate
        except FileExistsError:
            continue


def cleanup_temp_workspace(path: Path) -> None:
    shutil.rmtree(path, ignore_errors=True)


def verify_no_original_ec_content(bundle_root: Path, archive_path: Path) -> None:
    offenders: list[str] = []
    for path in bundle_root.rglob("*"):
        relative = path.relative_to(bundle_root).as_posix()
        parts = relative.split("/")
        if any(part in FORBIDDEN_CLASSIC_PACKAGE_DIRS for part in parts):
            offenders.append(relative)
            continue
        if path.is_file():
            if path.name.upper() in FORBIDDEN_CLASSIC_PACKAGE_NAMES:
                offenders.append(relative)
                continue
            if path.suffix.upper() == ".DAT":
                offenders.append(relative)

    if offenders:
        sample = ", ".join(sorted(offenders)[:5])
        raise SystemExit(
            f"{archive_path.name}: unexpected original EC content in Nostrian package ({sample})"
        )


def verify_archive(spec: BundleSpec, archive_path: Path, *, run_smoke: bool) -> None:
    temp_root = make_temp_workspace("nc-release-verify-")
    try:
        if spec.is_windows:
            with zipfile.ZipFile(archive_path, "r") as zf:
                zf.extractall(temp_root)
        else:
            with tarfile.open(archive_path, "r:gz") as tf:
                tf.extractall(temp_root)

        bundle_root = temp_root / spec.bundle_root_name
        if not bundle_root.exists():
            raise SystemExit(f"{archive_path.name}: missing bundle root {spec.bundle_root_name}")
        verify_no_original_ec_content(bundle_root, archive_path)

        docs_prefix = "" if spec.is_windows else "docs/"
        binary_prefix = "" if spec.is_windows else "bin/"
        binary_ext = ".exe" if spec.is_windows else ""
        if spec.artifact == "sysop":
            required_files = [
                "README.md",
                "BUILD-INFO.txt",
                f"{docs_prefix}nc_player_manual.pdf",
                f"{docs_prefix}nc_quick_start.pdf",
                f"{docs_prefix}nc_sysop_manual.pdf",
                "docs/sysop/README.md",
                "docs/sysop/bbs/mystic-bbs-setup.md",
                "docs/sysop/bbs/enigma-bbs-setup.md",
                "docs/sysop/bbs/synchronet-bbs-setup.md",
                "docs/sysop/bbs/wwiv-bbs-setup.md",
                f"{binary_prefix}nc-door{binary_ext}",
                f"{binary_prefix}nc-sysop{binary_ext}",
            ]
            if spec.is_windows:
                required_files.append("config.kdl")
            else:
                required_files.append("examples/config.kdl")
            forbidden_files = [
                f"{binary_prefix}nc-game{binary_ext}",
                f"{binary_prefix}nc-connect{binary_ext}",
                f"{binary_prefix}nc-connect-cli{binary_ext}",
                "licenses/OFL-0xProto.txt",
                "licenses/OFL-JetBrainsMono.txt",
                "licenses/LICENSE-NotoSansMono.txt",
                "tools/bbs/run_nc_rust.sh",
            ]
        else:
            required_files = [
                "README.md",
                "BUILD-INFO.txt",
                f"{docs_prefix}nc_player_manual.pdf",
                f"{docs_prefix}nc_quick_start.pdf",
                "licenses/OFL-0xProto.txt",
                "licenses/OFL-JetBrainsMono.txt",
                "licenses/LICENSE-NotoSansMono.txt",
            ]
            forbidden_files = [
                f"{binary_prefix}nc-connect-cli{binary_ext}",
            ]
            if spec.artifact == "public-beta":
                required_files.extend(
                    (
                        f"{docs_prefix}nc_sysop_manual.pdf",
                        f"{binary_prefix}nc-game{binary_ext}",
                        f"{binary_prefix}nc-door{binary_ext}",
                        f"{binary_prefix}nc-sysop{binary_ext}",
                        f"{binary_prefix}nc-connect{binary_ext}",
                    )
                )
            else:
                required_files.append(f"{binary_prefix}nc-connect{binary_ext}")

        for relative in required_files:
            path = bundle_root / relative
            if not path.exists():
                raise SystemExit(f"{archive_path.name}: missing {relative}")

        for relative in forbidden_files:
            path = bundle_root / relative
            if path.exists():
                raise SystemExit(f"{archive_path.name}: unexpected {relative}")

        if not run_smoke or spec.artifact == "nc-connect":
            print(
                f"{archive_path.name}: verified archive contents; skipped binary smoke run "
                f"because {'the target is not the current host' if not run_smoke else 'nc-connect is an interactive GUI archive'}."
            )
            return

        game_bin = str(bundle_root / f"{binary_prefix}nc-game{binary_ext}")
        door_bin = str(bundle_root / f"{binary_prefix}nc-door{binary_ext}")
        sysop_bin = str(bundle_root / f"{binary_prefix}nc-sysop{binary_ext}")
        run([door_bin, "--help"], cwd=bundle_root)
        run([sysop_bin, "--help"], cwd=bundle_root)

        if spec.artifact == "public-beta":
            run([game_bin, "--help"], cwd=bundle_root)
            campaign_dir = temp_root / "playtest-campaign"
            run(
                [
                    sysop_bin,
                    "new-game",
                    str(campaign_dir),
                    "--players",
                    "4",
                    "--seed",
                    "1515",
                ],
                cwd=bundle_root,
            )
            if not (campaign_dir / "ncgame.db").exists():
                raise SystemExit(f"{archive_path.name}: nc-sysop did not create ncgame.db")
        elif spec.artifact == "sysop":
            campaign_dir = temp_root / "sysop-bbs-campaign"
            campaign_dir.mkdir()
            example_src = (
                bundle_root / "config.kdl"
                if spec.is_windows
                else bundle_root / "examples" / "config.kdl"
            )
            shutil.copy2(example_src, campaign_dir / "config.kdl")
            run([sysop_bin, "new-game", "--bbs", str(campaign_dir)], cwd=bundle_root)
            if not (campaign_dir / "ncgame.db").exists():
                raise SystemExit(f"{archive_path.name}: nc-sysop --bbs did not create ncgame.db")
    finally:
        cleanup_temp_workspace(temp_root)


def main(*, default_target: str | None = None) -> None:
    args = parse_args(default_target)
    platform = resolve_target(args.target)
    spec = BundleSpec(version=load_version(), platform=platform, artifact=args.artifact)
    host_target = detect_host_target()
    binary_paths = build_binaries(spec)

    temp_root = make_temp_workspace("nc-release-build-")
    try:
        bundle_root = stage_bundle(spec, binary_paths, temp_root)
        archive_path = args.output_dir / spec.archive_name
        write_archive(bundle_root, archive_path)
        if args.verify:
            verify_archive(spec, archive_path, run_smoke=platform.target_triple == host_target)
        print(archive_path)
    finally:
        cleanup_temp_workspace(temp_root)


if __name__ == "__main__":
    main()
