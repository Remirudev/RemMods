#!/usr/bin/env python3
"""Checks the updater manifest and the asset set of a RemMods release (SPEC-P5 §2.5, §7.2, A14, A22).

Usage:
  python3 -I scripts/check-latest-json.py --latest F --config src-tauri/tauri.release.conf.json \\
      --version X.Y.Z --assets F --repo OWNER/REPO [--minisign DIR]
  python3 -I scripts/check-latest-json.py --exact-assets F --version X.Y.Z [--subset]
  python3 -I scripts/check-latest-json.py --self-test

Manifest mode: the platform keys are exactly linux-x86_64{,-appimage,-deb,-rpm} and
windows-x86_64{,-nsis}; linux-x86_64 equals the AppImage entry and windows-x86_64 the NSIS entry;
`version` is X.Y.Z; every URL is https://api.github.com/repos/OWNER/REPO/releases/assets/<id> with
<id> naming the expected asset in the assets list (`gh api .../releases/<id>/assets`); every
signature is the base64 of a minisign signature whose key id equals the configured pubkey's and
whose trusted comment carries file:<asset name> and version:X.Y.Z. --minisign DIR also writes the
decoded .minisig/.pub files into DIR and runs `minisign -Vm` on the AppImage and setup.exe found
there (CI only; this script never verifies Ed25519 itself).

--exact-assets: the release's asset names are exactly the eight bundles and signatures plus
latest.json (and SHA256SUMS if present); --subset allows missing names (a reused draft) but still
refuses unexpected ones. Stdlib only; reads its inputs, writes only into --minisign DIR.
"""

import argparse
import base64
import binascii
import json
import re
import subprocess
import sys
import tempfile
from datetime import datetime
from pathlib import Path

VERSION_RE = re.compile(r"(0|[1-9]\d{0,8})\.(0|[1-9]\d{0,8})\.(0|[1-9]\d{0,8})", re.ASCII)
REPO_RE = re.compile(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", re.ASCII)
PLACEHOLDER = "REMMODS-UPDATER-PUBKEY-NOT-SET"
PLATFORMS = {
    "linux-x86_64": "appimage",
    "linux-x86_64-appimage": "appimage",
    "linux-x86_64-deb": "deb",
    "linux-x86_64-rpm": "rpm",
    "windows-x86_64": "nsis",
    "windows-x86_64-nsis": "nsis",
}
MINISIGN_CHECKED = ("appimage", "nsis")
# The updater parses pub_date with time's Rfc3339; fromisoformat alone also takes ISO 8601 forms
# (date only, no offset, a space separator, the basic format) that the updater rejects.
PUB_DATE_RE = re.compile(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})", re.ASCII)


class CheckError(Exception):
    pass


def bundle_names(version):
    return {
        "appimage": f"RemMods_{version}_amd64.AppImage",
        "deb": f"RemMods_{version}_amd64.deb",
        "rpm": f"RemMods-{version}-1.x86_64.rpm",
        "nsis": f"RemMods_{version}_x64-setup.exe",
    }


def expected_assets(version):
    names = set()
    for name in bundle_names(version).values():
        names |= {name, name + ".sig"}
    return names | {"latest.json"}


def check_version(version):
    if not VERSION_RE.fullmatch(version or ""):
        raise CheckError(f"version {version!r} is not X.Y.Z")


def load_assets(path):
    """`gh api` output: one array, an array of pages (--slurp) or concatenated arrays (--paginate)."""
    text = Path(path).read_text(encoding="utf-8")
    decoder, pos, assets = json.JSONDecoder(), 0, []
    while pos < len(text):
        while pos < len(text) and text[pos].isspace():
            pos += 1
        if pos >= len(text):
            break
        value, pos = decoder.raw_decode(text, pos)
        pages = value if value and all(isinstance(v, list) for v in value) else [value]
        for page in pages:
            if not isinstance(page, list):
                raise CheckError("assets: expected JSON arrays of release assets")
            assets.extend(page)
    result = {}
    for asset in assets:
        if not isinstance(asset, dict) or not isinstance(asset.get("id"), int) or not isinstance(asset.get("name"), str):
            raise CheckError(f"assets: malformed entry {asset!r}")
        result[asset["id"]] = asset["name"]
    return result


def check_exact_assets(names, version, subset):
    check_version(version)
    allowed = expected_assets(version) | {"SHA256SUMS"}
    unexpected = sorted(set(names) - allowed)
    if len(set(names)) != len(list(names)):
        raise CheckError("assets: duplicate names")
    if unexpected:
        raise CheckError(f"unexpected release assets: {', '.join(unexpected)}")
    if not subset:
        missing = sorted(expected_assets(version) - set(names))
        if missing:
            raise CheckError(f"missing release assets: {', '.join(missing)}")


def b64decode(text, what):
    try:
        return base64.b64decode(text.strip(), validate=True)
    except (binascii.Error, ValueError):
        raise CheckError(f"{what}: not base64") from None


def key_id_hex(raw):
    return raw[::-1].hex().upper()


def parse_pubkey(config_value):
    if config_value == PLACEHOLDER:
        raise CheckError("pubkey is the placeholder")
    text = b64decode(config_value, "pubkey").decode("utf-8", "replace")
    lines = text.strip("\n").split("\n")
    if len(lines) != 2 or not lines[0].startswith("untrusted comment:"):
        raise CheckError("pubkey: not a minisign public key box")
    raw = b64decode(lines[1], "pubkey key line")
    if len(raw) != 42 or raw[:2] != b"Ed":
        raise CheckError("pubkey: not an Ed25519 minisign key")
    return {"key_id": key_id_hex(raw[2:10]), "box": text if text.endswith("\n") else text + "\n"}


def parse_signature(encoded, what):
    text = b64decode(encoded, what).decode("utf-8", "replace")
    lines = text.strip("\n").split("\n")
    if len(lines) != 4 or not lines[0].startswith("untrusted comment:") or not lines[2].startswith("trusted comment: "):
        raise CheckError(f"{what}: not a minisign signature box")
    raw = b64decode(lines[1], f"{what} signature line")
    if len(raw) != 74 or raw[:2] not in (b"ED", b"Ed"):
        raise CheckError(f"{what}: not an Ed25519 minisign signature")
    if len(b64decode(lines[3], f"{what} global signature")) != 64:
        raise CheckError(f"{what}: bad global signature length")
    fields = {}
    for part in lines[2].removeprefix("trusted comment: ").split("\t"):
        key, sep, value = part.partition(":")
        if sep:
            fields[key] = value
    return {"key_id": key_id_hex(raw[2:10]), "trusted": fields, "box": text if text.endswith("\n") else text + "\n"}


def check_manifest(doc, version, repo, assets, pubkey):
    check_version(version)
    if not REPO_RE.fullmatch(repo):
        raise CheckError(f"repo {repo!r} is not OWNER/REPO")
    if not isinstance(doc, dict):
        raise CheckError("latest.json is not an object")
    if doc.get("version") != version:
        raise CheckError(f"latest.json version {doc.get('version')!r}, expected {version!r}")
    if not isinstance(doc.get("notes"), str):
        raise CheckError("latest.json notes must be a string")
    pub_date = doc.get("pub_date")
    if not isinstance(pub_date, str) or not PUB_DATE_RE.fullmatch(pub_date):
        raise CheckError(f"latest.json pub_date {pub_date!r} is not RFC 3339")
    try:
        datetime.fromisoformat(pub_date.replace("Z", "+00:00"))
    except ValueError:
        raise CheckError(f"latest.json pub_date {pub_date!r} is not RFC 3339") from None
    platforms = doc.get("platforms")
    if not isinstance(platforms, dict):
        raise CheckError("latest.json has no platforms object")
    if set(platforms) != set(PLATFORMS):
        extra, missing = sorted(set(platforms) - set(PLATFORMS)), sorted(set(PLATFORMS) - set(platforms))
        raise CheckError(f"platform keys differ: extra {extra}, missing {missing}")
    if platforms["linux-x86_64"] != platforms["linux-x86_64-appimage"]:
        raise CheckError("linux-x86_64 is not the AppImage entry")
    if platforms["windows-x86_64"] != platforms["windows-x86_64-nsis"]:
        raise CheckError("windows-x86_64 is not the NSIS entry")
    names = bundle_names(version)
    prefix = f"https://api.github.com/repos/{repo}/releases/assets/"
    signatures = {}
    for key, bundle in PLATFORMS.items():
        entry = platforms[key]
        if not isinstance(entry, dict) or set(entry) != {"signature", "url"}:
            raise CheckError(f"{key}: expected exactly signature and url")
        url = entry["url"]
        if not isinstance(url, str) or not url.startswith(prefix) or not url[len(prefix) :].isdigit():
            raise CheckError(f"{key}: url {url!r} is not {prefix}<id>")
        asset_id = int(url[len(prefix) :])
        if assets.get(asset_id) != names[bundle]:
            raise CheckError(f"{key}: asset {asset_id} is {assets.get(asset_id)!r}, expected {names[bundle]!r}")
        if not isinstance(entry["signature"], str):
            raise CheckError(f"{key}: signature must be a string")
        signature = parse_signature(entry["signature"], f"{key} signature")
        if signature["key_id"] != pubkey["key_id"]:
            raise CheckError(f"{key}: signed by key {signature['key_id']}, the config has {pubkey['key_id']}")
        if signature["trusted"].get("file") != names[bundle]:
            raise CheckError(f"{key}: trusted comment file {signature['trusted'].get('file')!r}, expected {names[bundle]!r}")
        if signature["trusted"].get("version") != version:
            raise CheckError(f"{key}: trusted comment version {signature['trusted'].get('version')!r}, expected {version!r}")
        signatures[bundle] = signature
    return signatures


def run_minisign(directory, version, signatures, pubkey):
    directory = Path(directory)
    pub = directory / "remmods-updater.pub"
    pub.write_text(pubkey["box"], encoding="utf-8")
    names = bundle_names(version)
    for bundle in MINISIGN_CHECKED:
        target = directory / names[bundle]
        if not target.is_file():
            raise CheckError(f"--minisign: {target} not found")
        sig = directory / (names[bundle] + ".minisig")
        sig.write_text(signatures[bundle]["box"], encoding="utf-8")
        result = subprocess.run(["minisign", "-Vm", str(target), "-x", str(sig), "-p", str(pub)], check=False)
        if result.returncode != 0:
            raise CheckError(f"minisign rejected {names[bundle]}")


def config_pubkey(config_path):
    config = json.loads(Path(config_path).read_text(encoding="utf-8"))
    try:
        return config["plugins"]["updater"]["pubkey"]
    except (KeyError, TypeError):
        raise CheckError(f"{config_path}: no plugins.updater.pubkey") from None


# ---------------------------------------------------------------- self-test (public test vector)

# tauri-plugin-updater 2.14.1 src/updater.rs tests: a throwaway public key and one signature of it.
TEST_PUBLIC_KEY = (
    "untrusted comment: minisign public key: A49EF4746FA6E6D7\n"
    "RWTX5qZvdPSepDGWg2bz8n6ArQnBMwgQK1geVosorcbLzcunD+KJ+cYe"
)
TEST_SIGNATURE = (
    "untrusted comment: signature from tauri secret key\n"
    "RUTX5qZvdPSepPF2zsjvP/L/TQtBLRnWRvaq+4omK1UEugtQp/USrGiI1v762sDy3id3aor7bRC9ETyeot6lMHHrWoZ5hjUM1Qc=\n"
    "trusted comment: timestamp:1791649051\tfile:payload\tversion:2.0.0\n"
    "DDq2CLOEl5cQmC2eJRHlnZPVViOZsu1CjpPKQoei4A/XBfkDqGO0B+QMrcrHgEQ5qV3mne75tdFWhJpqUXh9AA=="
)
OTHER_PUBLIC_KEY = (
    "untrusted comment: minisign public key E7620F1842B4E81F\n"
    "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3"
)
FIXTURE_REPO = "Remirudev/RemMods"
FIXTURE_VERSION = "1.2.3"
FIXTURE_IDS = {"appimage": 101, "deb": 102, "rpm": 103, "nsis": 104}


def encode(text):
    return base64.b64encode(text.encode("utf-8")).decode("ascii")


def fixture_signature(file_name, version, signature=TEST_SIGNATURE):
    lines = signature.split("\n")
    lines[2] = f"trusted comment: timestamp:1791649051\tfile:{file_name}\tversion:{version}"
    return encode("\n".join(lines))


def fixture_assets(version=FIXTURE_VERSION):
    """The shared fixture: the assets list of a complete release (also used by make-latest-json)."""
    names = bundle_names(version)
    assets = [{"id": FIXTURE_IDS[b], "name": names[b]} for b in names]
    assets += [{"id": FIXTURE_IDS[b] + 100, "name": names[b] + ".sig"} for b in names]
    assets.append({"id": 900, "name": "latest.json"})
    return assets


def fixture_signatures(version=FIXTURE_VERSION):
    return {b: fixture_signature(name, version) for b, name in bundle_names(version).items()}


def fixture_manifest():
    sigs = fixture_signatures()

    def entry(bundle):
        url = f"https://api.github.com/repos/{FIXTURE_REPO}/releases/assets/{FIXTURE_IDS[bundle]}"
        return {"signature": sigs[bundle], "url": url}

    return {
        "version": FIXTURE_VERSION,
        "notes": "RemMods 1.2.3",
        "pub_date": "2026-11-02T08:00:00Z",
        "platforms": {key: entry(bundle) for key, bundle in PLATFORMS.items()},
    }


def fixture_asset_map():
    return {a["id"]: a["name"] for a in fixture_assets()}


def expect_fail(label, action):
    try:
        action()
    except CheckError:
        return
    raise AssertionError(f"{label}: expected a failure")


def self_test():
    pubkey = parse_pubkey(encode(TEST_PUBLIC_KEY))
    assert pubkey["key_id"] == "A49EF4746FA6E6D7", pubkey
    vector = parse_signature(encode(TEST_SIGNATURE), "vector")
    assert vector["key_id"] == "A49EF4746FA6E6D7", vector
    assert vector["trusted"] == {"timestamp": "1791649051", "file": "payload", "version": "2.0.0"}, vector
    expect_fail("placeholder pubkey", lambda: parse_pubkey(PLACEHOLDER))
    expect_fail("garbage pubkey", lambda: parse_pubkey(encode("untrusted comment: x\nAAAA")))

    assets = fixture_asset_map()
    good = fixture_manifest()
    check_manifest(good, FIXTURE_VERSION, FIXTURE_REPO, assets, pubkey)

    def mutated(change):
        doc = json.loads(json.dumps(good))
        change(doc)
        return doc

    def fails(label, change, version=FIXTURE_VERSION, repo=FIXTURE_REPO, key=pubkey):
        expect_fail(label, lambda: check_manifest(mutated(change), version, repo, assets, key))

    fails("missing platform", lambda d: d["platforms"].pop("linux-x86_64-rpm"))
    fails("extra platform", lambda d: d["platforms"].__setitem__("darwin-aarch64", d["platforms"]["linux-x86_64"]))
    fails("linux-x86_64 not the AppImage", lambda d: d["platforms"].__setitem__("linux-x86_64", d["platforms"]["linux-x86_64-deb"]))
    fails("windows-x86_64 not NSIS", lambda d: d["platforms"]["windows-x86_64"].__setitem__("url", d["platforms"]["linux-x86_64"]["url"]))
    other_key = parse_pubkey(encode(OTHER_PUBLIC_KEY))
    fails("another key id", lambda d: None, key=other_key)
    fails("another version in the comment", lambda d: [
        p.__setitem__("signature", fixture_signature(bundle_names(FIXTURE_VERSION)[PLATFORMS[k]], "1.2.2"))
        for k, p in d["platforms"].items()
    ])
    fails("another file in the comment", lambda d: d["platforms"]["linux-x86_64-deb"].__setitem__(
        "signature", fixture_signature(bundle_names(FIXTURE_VERSION)["rpm"], FIXTURE_VERSION)))
    fails("manifest version", lambda d: d.__setitem__("version", "1.2.4"))
    fails("url of another repo", lambda d: [
        p.__setitem__("url", p["url"].replace(FIXTURE_REPO, "someone/RemMods")) for p in d["platforms"].values()
    ])
    fails("http url", lambda d: [p.__setitem__("url", p["url"].replace("https:", "http:")) for p in d["platforms"].values()])
    fails("unknown asset id", lambda d: [p.__setitem__("url", p["url"][: p["url"].rfind("/") + 1] + "999") for p in d["platforms"].values()])
    fails("asset id of a .sig", lambda d: [p.__setitem__("url", p["url"][: p["url"].rfind("/") + 1] + "201") for p in d["platforms"].values()])
    fails("bad pub_date", lambda d: d.__setitem__("pub_date", "yesterday"))
    for bad_date in ("2026-11-02", "2026-11-02T08:00:00", "2026-11-02 08:00:00Z", "20261102T080000Z",
                     "2026-11-02T08:00Z", "2026-13-02T08:00:00Z"):
        fails(f"pub_date {bad_date}", lambda d, value=bad_date: d.__setitem__("pub_date", value))
    fails("pub_date not a string", lambda d: d.__setitem__("pub_date", None))
    fails("extra entry field", lambda d: d["platforms"]["linux-x86_64-rpm"].__setitem__("with_elevated_task", False))
    fails("not base64 signature", lambda d: d["platforms"]["linux-x86_64-rpm"].__setitem__("signature", "%%%"))

    names = [a["name"] for a in fixture_assets()]
    check_exact_assets(names, FIXTURE_VERSION, subset=False)
    check_exact_assets(names + ["SHA256SUMS"], FIXTURE_VERSION, subset=False)
    expect_fail("extra asset", lambda: check_exact_assets(names + ["RemMods_1.2.3_amd64.AppImage.zsync"], FIXTURE_VERSION, subset=False))
    expect_fail("missing asset", lambda: check_exact_assets(names[1:], FIXTURE_VERSION, subset=False))
    check_exact_assets(names[:3], FIXTURE_VERSION, subset=True)
    check_exact_assets([], FIXTURE_VERSION, subset=True)
    expect_fail("subset with an extra asset", lambda: check_exact_assets(names[:3] + ["evil.exe"], FIXTURE_VERSION, subset=True))
    expect_fail("other version's asset", lambda: check_exact_assets(["RemMods_1.2.2_amd64.deb"], FIXTURE_VERSION, subset=True))

    with tempfile.TemporaryDirectory() as tmp:
        paged = Path(tmp) / "assets.json"
        all_assets = fixture_assets()
        paged.write_text(json.dumps(all_assets[:5]) + "\n" + json.dumps(all_assets[5:]), encoding="utf-8")
        assert load_assets(paged) == fixture_asset_map(), "concatenated pages"
        paged.write_text(json.dumps([all_assets[:5], all_assets[5:]]), encoding="utf-8")
        assert load_assets(paged) == fixture_asset_map(), "slurped pages"
    print("check-latest-json: self-test passed")
    return 0


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--latest", type=Path)
    parser.add_argument("--config", type=Path)
    parser.add_argument("--version")
    parser.add_argument("--assets", type=Path)
    parser.add_argument("--repo")
    parser.add_argument("--minisign", type=Path)
    parser.add_argument("--exact-assets", type=Path)
    parser.add_argument("--subset", action="store_true")
    args = parser.parse_args(argv)
    if args.self_test:
        return self_test()
    try:
        if args.exact_assets:
            if not args.version:
                parser.error("--exact-assets needs --version")
            names = list(load_assets(args.exact_assets).values())
            check_exact_assets(names, args.version, args.subset)
            print(f"check-latest-json: {len(names)} release assets as expected")
            return 0
        if not (args.latest and args.config and args.version and args.assets and args.repo):
            parser.error("--latest, --config, --version, --assets and --repo are required")
        pubkey = parse_pubkey(config_pubkey(args.config))
        doc = json.loads(args.latest.read_text(encoding="utf-8"))
        signatures = check_manifest(doc, args.version, args.repo, load_assets(args.assets), pubkey)
        if args.minisign:
            run_minisign(args.minisign, args.version, signatures, pubkey)
    except (CheckError, OSError, ValueError) as error:
        print(f"check-latest-json: {error}", file=sys.stderr)
        return 1
    print(f"check-latest-json: latest.json for {args.version} is consistent (key {pubkey['key_id']})")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
