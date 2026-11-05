#!/usr/bin/env python3
"""Release gate for RemMods (SPEC-P5 §5.5, §7.2, W9). Stdlib only (Python 3.11+ for tomllib); read-only.

Usage: python3 -I scripts/release-check.py [--tag vX.Y.Z --strict]
       python3 -I scripts/release-check.py --self-test

CI mode (every PR): the version agrees in package.json, package-lock.json (root and packages[""]),
[workspace.package] of Cargo.toml and the Cargo.lock entries of remmods, remmods-core, remmods-ipc,
remmods-pak and remmods-unpack, and tauri.conf.json takes it from "../package.json";
src-tauri/tauri.release.conf.json
has exactly the expected structure (pubkey: the placeholder or a valid minisign key); no
plugins.updater in tauri{,.linux,.windows}.conf.json; no @tauri-apps/plugin-updater in package.json;
no `updater:` permission in capabilities/main.json; packaging/release-notes.md exists and names the
glibc floor (2.39), the Windows label (beta, CI-tested only) and the @VERSION@ placeholder.

--strict (the release workflow): additionally the tag is vX.Y.Z and equals the version, the pubkey
is a real key (not the placeholder), and docs/windows-gaps.md has no row whose Release-blocking cell
starts with `yes` while its Status does not start with `closed`.
"""

import argparse
import copy
import importlib.util
import json
import re
import shutil
import sys
import tempfile
import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
HERE = Path(__file__).resolve().parent
TAG_RE = re.compile(r"v(\d+)\.(\d+)\.(\d+)", re.ASCII)
LOCKED_CRATES = ("remmods", "remmods-core", "remmods-ipc", "remmods-pak", "remmods-unpack")
BASE_CONFIGS = ("tauri.conf.json", "tauri.linux.conf.json", "tauri.windows.conf.json")
NOTES_TOKENS = ("2.39", "beta, CI-tested only", "@VERSION@")
PLACEHOLDER = "REMMODS-UPDATER-PUBKEY-NOT-SET"
PUBKEY_SLOT = object()
EXPECTED_RELEASE_CONFIG = {
    "$schema": "https://schema.tauri.app/config/2",
    "bundle": {"createUpdaterArtifacts": True},
    "plugins": {
        "updater": {
            "pubkey": PUBKEY_SLOT,
            "endpoints": ["https://github.com/Remirudev/RemMods/releases/latest/download/latest.json"],
            "requireSignedVersion": True,
            "windows": {"installMode": "passive"},
        }
    },
}


def load_checker():
    sys.dont_write_bytecode = True  # no __pycache__ in scripts/ (python3 -I does not imply -B)
    spec = importlib.util.spec_from_file_location("check_latest_json", HERE / "check-latest-json.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


checker = load_checker()


def read_json(path, problems):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        problems.append(f"{path.name}: {error}")
        return None


def check_versions(root, problems):
    package = read_json(root / "package.json", problems) or {}
    lock = read_json(root / "package-lock.json", problems) or {}
    version = package.get("version")
    places = {
        "package-lock.json version": lock.get("version"),
        'package-lock.json packages[""].version': lock.get("packages", {}).get("", {}).get("version"),
    }
    try:
        cargo = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
        places["Cargo.toml [workspace.package].version"] = cargo.get("workspace", {}).get("package", {}).get("version")
        cargo_lock = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))
        locked = {p["name"]: p.get("version") for p in cargo_lock.get("package", []) if "source" not in p}
        for crate in LOCKED_CRATES:
            places[f"Cargo.lock {crate}"] = locked.get(crate)
    except (OSError, tomllib.TOMLDecodeError) as error:
        problems.append(f"Cargo files: {error}")
    for place, value in places.items():
        if value != version:
            problems.append(f"{place} is {value!r}, package.json says {version!r}")
    tauri = read_json(root / "src-tauri" / "tauri.conf.json", problems) or {}
    if tauri.get("version") != "../package.json":
        problems.append(f'tauri.conf.json version is {tauri.get("version")!r}, expected "../package.json"')
    return version, package


def with_slot(config):
    config = copy.deepcopy(config)
    pubkey = config.get("plugins", {}).get("updater", {}).get("pubkey") if isinstance(config, dict) else None
    if isinstance(pubkey, str):
        config["plugins"]["updater"]["pubkey"] = PUBKEY_SLOT
    return config, pubkey


def check_release_config(root, strict, problems):
    config = read_json(root / "src-tauri" / "tauri.release.conf.json", problems)
    if config is None:
        return
    shaped, pubkey = with_slot(config)
    if shaped != EXPECTED_RELEASE_CONFIG:
        problems.append("tauri.release.conf.json differs from the expected structure (SPEC-P5 §5.5)")
        return
    if pubkey == PLACEHOLDER:
        if strict:
            problems.append("tauri.release.conf.json pubkey is still the placeholder")
        return
    try:
        checker.parse_pubkey(pubkey)
    except checker.CheckError as error:
        problems.append(f"tauri.release.conf.json {error}")


def check_no_updater_leaks(root, package, problems):
    for name in BASE_CONFIGS:
        config = read_json(root / "src-tauri" / name, problems) or {}
        if "updater" in (config.get("plugins") or {}):
            problems.append(f"{name} has plugins.updater (only tauri.release.conf.json may)")
        if (config.get("bundle") or {}).get("createUpdaterArtifacts"):
            problems.append(f"{name} sets createUpdaterArtifacts (only tauri.release.conf.json may)")
    for section in ("dependencies", "devDependencies", "optionalDependencies", "peerDependencies"):
        if "@tauri-apps/plugin-updater" in (package.get(section) or {}):
            problems.append(f"package.json {section} has @tauri-apps/plugin-updater (the updater is Rust-only)")
    capabilities = read_json(root / "src-tauri" / "capabilities" / "main.json", problems) or {}
    for permission in capabilities.get("permissions", []):
        identifier = permission if isinstance(permission, str) else (permission or {}).get("identifier", "")
        if str(identifier).startswith("updater:"):
            problems.append(f"capabilities/main.json grants {identifier}")


def check_release_notes(root, problems):
    path = root / "packaging" / "release-notes.md"
    if not path.is_file():
        problems.append("packaging/release-notes.md is missing")
        return
    text = path.read_text(encoding="utf-8")
    for token in NOTES_TOKENS:
        if token not in text:
            problems.append(f"packaging/release-notes.md lacks {token!r}")


def open_blocking_gaps(text):
    columns, gaps = None, []
    for line in text.splitlines():
        if not line.lstrip().startswith("|"):
            columns = None if not line.strip() else columns
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if columns is None:
            columns = {name.lower(): i for i, name in enumerate(cells)}
            continue
        if all(set(cell) <= set("-: ") for cell in cells):
            continue
        blocking, status = columns.get("release-blocking"), columns.get("status")
        if blocking is None or status is None or max(blocking, status) >= len(cells):
            continue
        if cells[blocking].lower().startswith("yes") and not cells[status].lower().startswith("closed"):
            gaps.append(cells[0])
    return gaps


def check_strict(root, tag, version, problems):
    if not tag or not TAG_RE.fullmatch(tag):
        problems.append(f"tag {tag!r} is not vX.Y.Z")
    elif tag[1:] != version:
        problems.append(f"tag {tag} does not match the version {version}")
    gaps_path = root / "docs" / "windows-gaps.md"
    if not gaps_path.is_file():
        problems.append("docs/windows-gaps.md is missing")
        return
    gaps = open_blocking_gaps(gaps_path.read_text(encoding="utf-8"))
    if gaps:
        problems.append(f"open release-blocking Windows gaps: {', '.join(gaps)}")


def run_checks(root, tag=None, strict=False):
    problems = []
    version, package = check_versions(root, problems)
    check_release_config(root, strict, problems)
    check_no_updater_leaks(root, package, problems)
    check_release_notes(root, problems)
    if strict:
        check_strict(root, tag, version, problems)
    return version, problems


# ---------------------------------------------------------------- self-test

FIXTURE_FILES = (
    "package.json",
    "package-lock.json",
    "Cargo.toml",
    "Cargo.lock",
    "src-tauri/tauri.conf.json",
    "src-tauri/tauri.linux.conf.json",
    "src-tauri/tauri.windows.conf.json",
    "src-tauri/capabilities/main.json",
    "packaging/release-notes.md",
)
GAPS_CLOSED = """# Windows gaps

| ID | Feature | What's missing on Windows | Release-blocking | Status |
|----|---------|---------------------------|------------------|--------|
| WG-1 | A | x | no | open |
| WG-2 | B | y | yes | closed (v1.2.3) |
| WG-5 | C | z | yes (for 1.0) | closed (#9) |
"""


def make_fixture(root):
    files = {
        "package.json": {"name": "remmods", "version": "1.2.3", "dependencies": {"@tauri-apps/api": "2.12.4"}},
        "package-lock.json": {"name": "remmods", "version": "1.2.3", "lockfileVersion": 3,
                              "packages": {"": {"name": "remmods", "version": "1.2.3"}}},
        "src-tauri/tauri.conf.json": {"productName": "RemMods", "version": "../package.json", "bundle": {"active": True}},
        "src-tauri/tauri.linux.conf.json": {"bundle": {"targets": ["deb"]}},
        "src-tauri/tauri.windows.conf.json": {"bundle": {"targets": ["nsis"]}},
        "src-tauri/capabilities/main.json": {"permissions": ["core:app:default", "allow-update-check"]},
    }
    for name, value in files.items():
        (root / name).parent.mkdir(parents=True, exist_ok=True)
        (root / name).write_text(json.dumps(value, indent=2), encoding="utf-8")
    (root / "Cargo.toml").write_text('[workspace]\nmembers = []\n\n[workspace.package]\nversion = "1.2.3"\n', encoding="utf-8")
    lock = "version = 4\n" + "".join(f'\n[[package]]\nname = "{c}"\nversion = "1.2.3"\n' for c in LOCKED_CRATES)
    lock += '\n[[package]]\nname = "serde"\nversion = "1.0.0"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\n'
    (root / "Cargo.lock").write_text(lock, encoding="utf-8")
    release = copy.deepcopy(EXPECTED_RELEASE_CONFIG)
    release["plugins"]["updater"]["pubkey"] = PLACEHOLDER
    (root / "src-tauri" / "tauri.release.conf.json").write_text(json.dumps(release, indent=2), encoding="utf-8")
    (root / "packaging").mkdir(exist_ok=True)
    (root / "packaging" / "release-notes.md").write_text(
        "RemMods @VERSION@\nglibc 2.39\nWindows: beta, CI-tested only\n", encoding="utf-8")
    (root / "docs").mkdir(exist_ok=True)
    (root / "docs" / "windows-gaps.md").write_text(GAPS_CLOSED, encoding="utf-8")


def edit_json(path, change):
    value = json.loads(path.read_text(encoding="utf-8"))
    change(value)
    path.write_text(json.dumps(value), encoding="utf-8")


def edit_text(path, old, new):
    text = path.read_text(encoding="utf-8")
    assert old in text, (path, old)
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


def set_real_pubkey(root):
    edit_json(root / "src-tauri" / "tauri.release.conf.json",
              lambda c: c["plugins"]["updater"].__setitem__("pubkey", checker.encode(checker.TEST_PUBLIC_KEY)))


def self_test():
    with tempfile.TemporaryDirectory() as tmp:
        base = Path(tmp) / "base"
        base.mkdir()
        make_fixture(base)

        counter = iter(range(1000))

        def case(label, mutate, strict=False, tag=None, should_pass=False):
            root = Path(tmp) / f"case-{next(counter)}"
            shutil.copytree(base, root)
            mutate(root)
            _, problems = run_checks(root, tag=tag, strict=strict)
            if should_pass and problems:
                raise AssertionError(f"{label}: unexpected problems {problems}")
            if not should_pass and not problems:
                raise AssertionError(f"{label}: expected a failure")
            shutil.rmtree(root)

        case("clean CI mode with the placeholder", lambda r: None, should_pass=True)
        case("placeholder under --strict", lambda r: None, strict=True, tag="v1.2.3")
        case("real key under --strict", set_real_pubkey, strict=True, tag="v1.2.3", should_pass=True)
        case("real key in CI mode", set_real_pubkey, should_pass=True)
        case("invalid key", lambda r: edit_json(r / "src-tauri/tauri.release.conf.json",
             lambda c: c["plugins"]["updater"].__setitem__("pubkey", "bm90IGEga2V5")))

        case("package.json version", lambda r: edit_json(r / "package.json", lambda c: c.__setitem__("version", "1.2.4")))
        case("package-lock root version", lambda r: edit_json(r / "package-lock.json", lambda c: c.__setitem__("version", "1.2.4")))
        case('package-lock packages[""] version', lambda r: edit_json(r / "package-lock.json",
             lambda c: c["packages"][""].__setitem__("version", "1.2.4")))
        case("Cargo.toml version", lambda r: edit_text(r / "Cargo.toml", 'version = "1.2.3"', 'version = "1.2.4"'))
        for crate in LOCKED_CRATES:
            case(f"Cargo.lock {crate}", lambda r, c=crate: edit_text(
                r / "Cargo.lock", f'name = "{c}"\nversion = "1.2.3"', f'name = "{c}"\nversion = "1.2.4"'))
        case("tauri.conf.json version", lambda r: edit_json(r / "src-tauri/tauri.conf.json",
             lambda c: c.__setitem__("version", "1.2.3")))

        case("JS updater package", lambda r: edit_json(r / "package.json",
             lambda c: c["dependencies"].__setitem__("@tauri-apps/plugin-updater", "2.10.0")))
        case("updater: permission", lambda r: edit_json(r / "src-tauri/capabilities/main.json",
             lambda c: c["permissions"].append("updater:default")))
        for name in BASE_CONFIGS:
            case(f"plugins.updater in {name}", lambda r, n=name: edit_json(r / "src-tauri" / n,
                 lambda c: c.setdefault("plugins", {}).__setitem__("updater", {"pubkey": "x"})))
        case("createUpdaterArtifacts in the base config", lambda r: edit_json(r / "src-tauri/tauri.conf.json",
             lambda c: c["bundle"].__setitem__("createUpdaterArtifacts", True)))
        case("release config endpoint", lambda r: edit_json(r / "src-tauri/tauri.release.conf.json",
             lambda c: c["plugins"]["updater"].__setitem__("endpoints", ["https://example.invalid/latest.json"])))
        case("release config requireSignedVersion", lambda r: edit_json(r / "src-tauri/tauri.release.conf.json",
             lambda c: c["plugins"]["updater"].__setitem__("requireSignedVersion", False)))
        case("release config extra key", lambda r: edit_json(r / "src-tauri/tauri.release.conf.json",
             lambda c: c["plugins"]["updater"].__setitem__("dangerousInsecureTransportProtocol", True)))

        case("release notes missing", lambda r: (r / "packaging/release-notes.md").unlink())
        for token in NOTES_TOKENS:
            case(f"release notes without {token}", lambda r, t=token: edit_text(r / "packaging/release-notes.md", t, "-"))

        def open_gap(r):
            set_real_pubkey(r)
            edit_text(r / "docs/windows-gaps.md", "| yes (for 1.0) | closed (#9) |", "| yes (for 1.0) | open |")

        case("open release-blocking gap under --strict", open_gap, strict=True, tag="v1.2.3")
        case("open release-blocking gap in CI mode", open_gap, should_pass=True)
        case("tag mismatch", set_real_pubkey, strict=True, tag="v1.2.4")
        case("tag without v", set_real_pubkey, strict=True, tag="1.2.3")
        case("pre-release tag", set_real_pubkey, strict=True, tag="v1.2.3-rc.1")
        case("strict without a tag", set_real_pubkey, strict=True, tag=None)

    real = open_blocking_gaps((REPO / "docs" / "windows-gaps.md").read_text(encoding="utf-8")) \
        if (REPO / "docs" / "windows-gaps.md").is_file() else []
    print(f"release-check: self-test passed (the repository's open release-blocking gaps: {', '.join(real) or 'none'})")
    return 0


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--tag")
    parser.add_argument("--strict", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)
    if args.self_test:
        return self_test()
    if args.strict and not args.tag:
        parser.error("--strict needs --tag")
    version, problems = run_checks(REPO, tag=args.tag, strict=args.strict)
    for problem in problems:
        print(f"release-check: FAIL: {problem}", file=sys.stderr)
    if problems:
        return 1
    mode = "strict" if args.strict else "CI"
    print(f"release-check: version {version} consistent ({mode} mode)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
