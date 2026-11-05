#!/usr/bin/env python3
"""Writes the updater manifest latest.json of a RemMods release (SPEC-P5 §2.5, §7.2, §7.5, A14, A22).

Usage:
  python3 -I scripts/make-latest-json.py --version X.Y.Z --assets F --sigs DIR \\
      --repo OWNER/REPO --notes F --out latest.json
  python3 -I scripts/make-latest-json.py --self-test

Layout as tauri-action v1 writes it: `version`, `notes` (the first non-empty line of --notes),
`pub_date` (UTC, RFC 3339, seconds) and `platforms` with linux-x86_64{,-appimage,-deb,-rpm} and
windows-x86_64{,-nsis}; each entry is {signature: <content of DIR/<asset>.sig>, url:
https://api.github.com/repos/OWNER/REPO/releases/assets/<id>} with the id taken from the assets list
(`gh api .../releases/<id>/assets`). Refuses a missing asset or .sig. The result is checked with
check-latest-json.py's rules (key id excepted: the pubkey is not an input here). Stdlib only;
writes only --out.
"""

import argparse
import importlib.util
import json
import sys
import tempfile
from datetime import UTC, datetime
from pathlib import Path

HERE = Path(__file__).resolve().parent


def load_checker():
    sys.dont_write_bytecode = True  # no __pycache__ in scripts/ (python3 -I does not imply -B)
    spec = importlib.util.spec_from_file_location("check_latest_json", HERE / "check-latest-json.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


checker = load_checker()
CheckError = checker.CheckError


def first_line(path):
    for line in Path(path).read_text(encoding="utf-8").splitlines():
        if line.strip():
            return line.strip()
    raise CheckError(f"{path}: no notes line")


def build_manifest(version, assets, sigs_dir, repo, notes, now):
    checker.check_version(version)
    if not checker.REPO_RE.fullmatch(repo):
        raise CheckError(f"repo {repo!r} is not OWNER/REPO")
    ids = {name: asset_id for asset_id, name in assets.items()}
    names = checker.bundle_names(version)
    entries = {}
    for bundle, name in names.items():
        if name not in ids:
            raise CheckError(f"asset {name} is not in the release")
        sig_path = Path(sigs_dir) / (name + ".sig")
        if not sig_path.is_file():
            raise CheckError(f"missing signature {sig_path}")
        signature = sig_path.read_text(encoding="ascii").strip()
        url = f"https://api.github.com/repos/{repo}/releases/assets/{ids[name]}"
        entries[bundle] = {"signature": signature, "url": url}
    return {
        "version": version,
        "notes": notes,
        "pub_date": now.astimezone(UTC).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "platforms": {key: dict(entries[bundle]) for key, bundle in checker.PLATFORMS.items()},
    }


def self_check(doc, version, repo, assets):
    """check-latest-json's checks with the key id taken from the signatures themselves."""
    signature = doc["platforms"]["linux-x86_64"]["signature"]
    key_id = checker.parse_signature(signature, "linux-x86_64 signature")["key_id"]
    checker.check_manifest(doc, version, repo, assets, {"key_id": key_id, "box": ""})


def write_fixture_sigs(directory, version):
    names = checker.bundle_names(version)
    for bundle, signature in checker.fixture_signatures(version).items():
        (directory / (names[bundle] + ".sig")).write_text(signature, encoding="ascii")


def self_test():
    version, repo = checker.FIXTURE_VERSION, checker.FIXTURE_REPO
    assets = checker.fixture_asset_map()
    now = datetime(2026, 11, 2, 8, 0, 0, tzinfo=UTC)
    with tempfile.TemporaryDirectory() as tmp:
        sigs = Path(tmp)
        write_fixture_sigs(sigs, version)
        doc = build_manifest(version, assets, sigs, repo, "RemMods 1.2.3", now)
        assert doc == checker.fixture_manifest(), "manifest differs from the shared fixture"
        checker.check_manifest(doc, version, repo, assets, checker.parse_pubkey(checker.encode(checker.TEST_PUBLIC_KEY)))
        self_check(doc, version, repo, assets)

        missing_asset = {k: v for k, v in assets.items() if v != checker.bundle_names(version)["rpm"]}
        checker.expect_fail("missing asset", lambda: build_manifest(version, missing_asset, sigs, repo, "n", now))
        (sigs / (checker.bundle_names(version)["nsis"] + ".sig")).unlink()
        checker.expect_fail("missing .sig", lambda: build_manifest(version, assets, sigs, repo, "n", now))
        checker.expect_fail("bad version", lambda: build_manifest("1.2", assets, sigs, repo, "n", now))
    print("make-latest-json: self-test passed")
    return 0


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--version")
    parser.add_argument("--assets", type=Path)
    parser.add_argument("--sigs", type=Path)
    parser.add_argument("--repo")
    parser.add_argument("--notes", type=Path)
    parser.add_argument("--out", type=Path)
    args = parser.parse_args(argv)
    if args.self_test:
        return self_test()
    if not (args.version and args.assets and args.sigs and args.repo and args.notes and args.out):
        parser.error("--version, --assets, --sigs, --repo, --notes and --out are required")
    try:
        assets = checker.load_assets(args.assets)
        doc = build_manifest(args.version, assets, args.sigs, args.repo, first_line(args.notes), datetime.now(UTC))
        self_check(doc, args.version, args.repo, assets)
    except (CheckError, OSError, ValueError) as error:
        print(f"make-latest-json: {error}", file=sys.stderr)
        return 1
    args.out.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")
    print(f"make-latest-json: wrote {args.out} for {args.version}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
