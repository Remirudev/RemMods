#!/usr/bin/env python3
"""Writes THIRD-PARTY-NOTICES-NPM from package-lock.json and node_modules (SPEC-P5 §7.1). Stdlib only.

Usage: python3 -I scripts/npm-notices.py [--check] [--out FILE]

Lists every production package of package-lock.json (lockfileVersion 3: every `node_modules/...`
entry without `dev`/`devOptional`), sorted by package path: `name version — license`, the repository
URL if any, then the package's LICENSE*/LICENCE*/COPYING*/NOTICE* files (ASCII case-insensitive,
sorted). A package without such a file whose licence is `MIT` gets the standard MIT text with the
package.json author; any other licence without a file fails. Needs an installed node_modules that
matches the lockfile (`npm ci`).

--check   compare with the committed file instead of writing it; exit 1 and print the regenerate
          command on a difference.
--out F   write (or with --check, compare) F instead of THIRD-PARTY-NOTICES-NPM at the repo root.
"""

import argparse
import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_OUT = REPO / "THIRD-PARTY-NOTICES-NPM"
RULE = "=" * 80
NOTICE_FILE = re.compile(r"^(licen[cs]e|copying|notice)", re.IGNORECASE | re.ASCII)

HEADER = (
    "THIRD-PARTY NOTICES (npm packages)\n"
    "\n"
    "The RemMods user interface bundles the npm packages listed below; the fonts Inter and\n"
    "JetBrains Mono (@fontsource-variable) are distributed under the SIL Open Font License 1.1.\n"
)

MIT_TEXT = """MIT License

Copyright (c) {holder}

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"""


class NoticeError(Exception):
    pass


def production_packages(lock):
    if lock.get("lockfileVersion") != 3:
        raise NoticeError(f"package-lock.json: lockfileVersion {lock.get('lockfileVersion')!r}, expected 3")
    packages = []
    for path, meta in lock.get("packages", {}).items():
        if not path.startswith("node_modules/") or meta.get("dev") or meta.get("devOptional"):
            continue
        if meta.get("link"):
            raise NoticeError(f"{path}: linked packages are not supported")
        packages.append((path, meta))
    return sorted(packages, key=lambda item: item[0])


def package_name(path):
    return path.rsplit("node_modules/", 1)[1]


def normalize_text(raw, source):
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError as error:
        raise NoticeError(f"{source}: not UTF-8 ({error})") from None
    text = text.removeprefix("﻿").replace("\r\n", "\n").replace("\r", "\n")
    lines = [line.rstrip() for line in text.split("\n")]
    return "\n".join(lines).strip("\n") + "\n"


def repository_url(manifest):
    repo = manifest.get("repository")
    if isinstance(repo, dict):
        repo = repo.get("url")
    if not isinstance(repo, str) or not repo.strip():
        return None
    url = repo.strip().removeprefix("git+")
    if url.startswith("github:"):
        url = url.removeprefix("github:")
    if re.fullmatch(r"[\w.-]+/[\w.-]+", url):
        url = f"https://github.com/{url}"
    url = re.sub(r"^git://", "https://", url)
    url = re.sub(r"^ssh://git@", "https://", url)
    return url.removesuffix(".git")


def author_name(manifest):
    author = manifest.get("author")
    if isinstance(author, dict):
        author = author.get("name")
    if not isinstance(author, str):
        return None
    name = re.sub(r"\s*[<(].*$", "", author).strip()
    return name or None


def notice_files(folder):
    names = [p.name for p in folder.iterdir() if p.is_file() and NOTICE_FILE.match(p.name)]
    return sorted(names, key=lambda n: (n.lower(), n))


def package_section(path, meta, root):
    folder = root / path
    name = package_name(path)
    manifest_path = folder / "package.json"
    if not manifest_path.is_file():
        raise NoticeError(f"{path}: not installed (run npm ci)")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    version = meta.get("version")
    if manifest.get("version") != version:
        raise NoticeError(f"{path}: node_modules has {manifest.get('version')}, the lockfile {version} (run npm ci)")
    license_id = meta.get("license") or manifest.get("license")
    if not isinstance(license_id, str) or not license_id:
        raise NoticeError(f"{path}: no licence in the lockfile or package.json")

    out = [RULE, f"{name} {version} — {license_id}"]
    url = repository_url(manifest)
    if url:
        out.append(url)
    files = notice_files(folder)
    if files:
        for file_name in files:
            out += ["", f"----- {file_name} -----", normalize_text((folder / file_name).read_bytes(), folder / file_name).rstrip("\n")]
    elif license_id == "MIT":
        holder = author_name(manifest)
        if not holder:
            raise NoticeError(f"{path}: MIT without a licence file and without an author")
        out += ["", "----- MIT licence text (the package ships no licence file) -----", MIT_TEXT.format(holder=holder).rstrip("\n")]
    else:
        raise NoticeError(f"{path}: licence {license_id} but no licence file")
    return "\n".join(out) + "\n"


def render(root):
    lock = json.loads((root / "package-lock.json").read_text(encoding="utf-8"))
    packages = production_packages(lock)
    sections = [package_section(path, meta, root) for path, meta in packages]
    summary = f"\n{len(packages)} packages.\n"
    return HEADER + summary + "\n" + "\n".join(sections), len(packages)


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = parser.parse_args(argv)
    try:
        text, count = render(REPO)
    except (NoticeError, OSError, ValueError) as error:
        print(f"npm-notices: {error}", file=sys.stderr)
        return 1
    data = text.encode("utf-8")
    if args.check:
        current = args.out.read_bytes() if args.out.is_file() else b""
        if current != data:
            print(f"npm-notices: {args.out} is stale; regenerate with: python3 -I scripts/npm-notices.py", file=sys.stderr)
            return 1
        print(f"npm-notices: {args.out.name} is current ({count} packages)")
        return 0
    args.out.write_bytes(data)
    print(f"npm-notices: wrote {args.out} ({count} packages)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
