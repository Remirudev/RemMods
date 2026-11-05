#!/usr/bin/env python3
"""Inspects a RemMods .deb or .rpm without installing it (SPEC-P5 §7.2, §9.5 P5). Stdlib only.

Usage: python3 -I scripts/inspect-bundle.py FILE [--expect deb|rpm] [--json]
       python3 -I scripts/inspect-bundle.py --self-test

deb: reads the ar members, the control fields of control.tar.* and the file list of data.tar.*.
rpm: reads the header tags (name, version, release, requires, file list, payload compressor) and
the cpio payload. Prints a summary (or JSON). --expect asserts the release contract: package name
rem-mods, the glibc/libssl/xdg-utils/webkit/gtk dependencies, /usr/bin/remmods, the desktop file
(StartupWMClass=remmods, the safe-graphics action), the two notices under /usr/share/doc/remmods
(byte-equal to the repository's files), the four hicolor icons, and a binary patched as this
package type (no __TAURI_BUNDLE_TYPE_VAR_UNK left, the expected token exactly once more than each
other one). Exit 1 on any failed expectation. Reads only FILE.
"""

import argparse
import gzip
import io
import json
import lzma
import re
import struct
import sys
import tarfile
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
PACKAGE_NAME = "rem-mods"
BINARY = "/usr/bin/remmods"
DESKTOP = "/usr/share/applications/RemMods.desktop"
NOTICES = ("THIRD-PARTY-NOTICES", "THIRD-PARTY-NOTICES-NPM")
ICON_SIZES = ("32x32", "128x128", "256x256@2", "512x512")
DEB_DEPENDS = ("libc6 (>= 2.39)", "libssl3t64 | libssl3", "xdg-utils", "libwebkit2gtk-4.1-0", "libgtk-3-0")
RPM_REQUIRES = (
    "libc.so.6(GLIBC_2.39)(64bit)",
    "libssl.so.3()(64bit)",
    "xdg-utils",
    "libwebkit2gtk-4.1.so.0()(64bit)",
    "libgtk-3.so.0()(64bit)",
)
DESKTOP_LINES = ("StartupWMClass=remmods", "Actions=safe-graphics;", "Exec=remmods --safe-graphics")
TOKEN_PREFIX = b"__TAURI_BUNDLE_TYPE_VAR_"
TOKENS = ("DEB", "RPM", "APP", "MSI", "NSS")
EXPECTED_TOKEN = {"deb": "DEB", "rpm": "RPM"}

RPM_LEAD_MAGIC = b"\xed\xab\xee\xdb"
RPM_HEADER_MAGIC = b"\x8e\xad\xe8\x01"
TAG_NAME, TAG_VERSION, TAG_RELEASE = 1000, 1001, 1002
TAG_REQUIRENAME = 1049
TAG_DIRINDEXES, TAG_BASENAMES, TAG_DIRNAMES = 1116, 1117, 1118
TAG_PAYLOADCOMPRESSOR = 1125


class BundleError(Exception):
    pass


# ---------------------------------------------------------------- deb


def ar_members(data):
    if not data.startswith(b"!<arch>\n"):
        raise BundleError("not an ar archive")
    members, pos = {}, 8
    while pos < len(data):
        header = data[pos : pos + 60]
        if len(header) < 60 or header[58:60] != b"`\n":
            raise BundleError(f"bad ar member header at {pos}")
        name = header[0:16].decode("ascii").strip().rstrip("/")
        size = int(header[48:58].decode("ascii").strip())
        start = pos + 60
        members[name] = data[start : start + size]
        pos = start + size + (size & 1)
    return members


def open_tar(name, blob):
    if name.endswith(".gz"):
        return tarfile.open(fileobj=io.BytesIO(gzip.decompress(blob)))
    if name.endswith(".xz"):
        return tarfile.open(fileobj=io.BytesIO(lzma.decompress(blob)))
    if name.endswith(".tar"):
        return tarfile.open(fileobj=io.BytesIO(blob))
    raise BundleError(f"unsupported deb member compression: {name}")


def member(members, prefix):
    for name, blob in members.items():
        if name.startswith(prefix):
            return name, blob
    raise BundleError(f"no {prefix}* member")


def parse_control(text):
    fields, last = {}, None
    for line in text.splitlines():
        if line.startswith((" ", "\t")) and last:
            fields[last] += "\n" + line.strip()
        elif ":" in line:
            last, value = line.split(":", 1)
            fields[last] = value.strip()
    return fields


def split_depends(value):
    return [re.sub(r"\s+", " ", item.strip()) for item in value.split(",") if item.strip()]


def inspect_deb(data):
    members = ar_members(data)
    if members.get("debian-binary", b"").strip() != b"2.0":
        raise BundleError("debian-binary is not 2.0")
    control_name, control_blob = member(members, "control.tar")
    data_name, data_blob = member(members, "data.tar")
    with open_tar(control_name, control_blob) as tar:
        control_member = next((m for m in tar.getmembers() if m.name.lstrip("./") == "control"), None)
        if control_member is None:
            raise BundleError("control.tar has no control file")
        control = parse_control(tar.extractfile(control_member).read().decode("utf-8"))
    files, contents = [], {}
    with open_tar(data_name, data_blob) as tar:
        for entry in tar.getmembers():
            path = "/" + entry.name.lstrip("./").lstrip("/")
            if entry.isfile():
                files.append(path)
                if path == BINARY or path == DESKTOP or path.startswith("/usr/share/doc/remmods/"):
                    contents[path] = tar.extractfile(entry).read()
    return {
        "kind": "deb",
        "name": control.get("Package"),
        "version": control.get("Version"),
        "arch": control.get("Architecture"),
        "requires": split_depends(control.get("Depends", "")),
        "members": sorted(members),
        "payload": data_name,
        "files": sorted(files),
    }, contents


# ---------------------------------------------------------------- rpm


def rpm_header(data, pos):
    if data[pos : pos + 4] != RPM_HEADER_MAGIC:
        raise BundleError(f"no rpm header magic at {pos}")
    count, store_size = struct.unpack(">II", data[pos + 8 : pos + 16])
    index_start = pos + 16
    store = index_start + 16 * count
    tags = {}
    for i in range(count):
        tag, kind, offset, n = struct.unpack(">IIII", data[index_start + 16 * i : index_start + 16 * i + 16])
        tags[tag] = rpm_value(data, store + offset, kind, n)
    return tags, store + store_size


def rpm_value(data, at, kind, count):
    if kind in (6, 8, 9):
        values, pos = [], at
        for _ in range(1 if kind == 6 else count):
            end = data.index(b"\0", pos)
            values.append(data[pos:end].decode("utf-8", "replace"))
            pos = end + 1
        return values[0] if kind == 6 else values
    widths = {2: ("B", 1), 1: ("B", 1), 3: ("H", 2), 4: ("I", 4), 5: ("Q", 8)}
    if kind in widths:
        code, width = widths[kind]
        return list(struct.unpack(f">{count}{code}", data[at : at + width * count]))
    if kind == 7:
        return data[at : at + count]
    return None


def cpio_entries(payload):
    entries, pos = {}, 0
    while pos + 110 <= len(payload):
        magic = payload[pos : pos + 6]
        if magic != b"070701" and magic != b"070702":
            raise BundleError(f"unsupported cpio entry {magic!r} at {pos}")
        fields = [int(payload[pos + 6 + 8 * i : pos + 14 + 8 * i], 16) for i in range(13)]
        mode, size, name_size = fields[1], fields[6], fields[11]
        name_start = pos + 110
        name = payload[name_start : name_start + name_size - 1].decode("utf-8", "replace")
        data_start = (name_start + name_size + 3) & ~3
        if name == "TRAILER!!!":
            break
        if (mode & 0o170000) == 0o100000:
            entries["/" + name.lstrip("./").lstrip("/")] = payload[data_start : data_start + size]
        pos = (data_start + size + 3) & ~3
    return entries


def decompress(compressor, blob):
    if compressor == "gzip":
        return gzip.decompress(blob)
    if compressor in ("xz", "lzma"):
        return lzma.decompress(blob)
    if compressor == "zstd":
        try:
            from compression import zstd  # Python 3.14+
        except ImportError:
            raise BundleError("zstd payload needs Python 3.14") from None
        return zstd.decompress(blob)
    raise BundleError(f"unsupported rpm payload compressor {compressor!r}")


def inspect_rpm(data):
    if not data.startswith(RPM_LEAD_MAGIC):
        raise BundleError("not an rpm (lead magic)")
    _, end = rpm_header(data, 96)
    end = (end + 7) & ~7
    tags, payload_start = rpm_header(data, end)
    dirnames, basenames, indexes = tags.get(TAG_DIRNAMES, []), tags.get(TAG_BASENAMES, []), tags.get(TAG_DIRINDEXES, [])
    files = sorted(dirnames[i] + base for i, base in zip(indexes, basenames))
    compressor = tags.get(TAG_PAYLOADCOMPRESSOR, "gzip")
    entries = cpio_entries(decompress(compressor, data[payload_start:]))
    contents = {p: b for p, b in entries.items() if p == BINARY or p == DESKTOP or p.startswith("/usr/share/doc/remmods/")}
    regular = sorted(entries)
    return {
        "kind": "rpm",
        "name": tags.get(TAG_NAME),
        "version": tags.get(TAG_VERSION),
        "release": tags.get(TAG_RELEASE),
        "requires": list(tags.get(TAG_REQUIRENAME, [])),
        "payload": compressor,
        "files": files,
        "payload_files": regular,
    }, contents


# ---------------------------------------------------------------- expectations


def token_counts(binary):
    counts = {name: binary.count(TOKEN_PREFIX + name.encode()) for name in TOKENS}
    counts["UNK"] = binary.count(TOKEN_PREFIX + b"UNK")
    return counts


def check_expectations(kind, summary, contents, repo):
    problems = []
    expected = EXPECTED_TOKEN[kind]
    if summary["kind"] != kind:
        return [f"expected a {kind}, got a {summary['kind']}"]
    if summary["name"] != PACKAGE_NAME:
        problems.append(f"package name {summary['name']!r}, expected {PACKAGE_NAME!r}")
    wanted = DEB_DEPENDS if kind == "deb" else RPM_REQUIRES
    missing = [item for item in wanted if item not in summary["requires"]]
    if missing:
        problems.append(f"missing dependencies: {missing}")
    files = set(summary["files"])
    needed = [BINARY, DESKTOP] + [f"/usr/share/doc/remmods/{n}" for n in NOTICES]
    needed += [f"/usr/share/icons/hicolor/{size}/apps/remmods.png" for size in ICON_SIZES]
    absent = [path for path in needed if path not in files]
    if absent:
        problems.append(f"missing files: {absent}")
    desktop = contents.get(DESKTOP, b"").decode("utf-8", "replace").splitlines()
    for line in DESKTOP_LINES:
        if line not in desktop:
            problems.append(f"desktop file lacks {line!r}")
    if any("{{" in line for line in desktop):
        problems.append("desktop file has unrendered template markers")
    for name in NOTICES:
        packaged = contents.get(f"/usr/share/doc/remmods/{name}")
        source = repo / name
        if packaged is not None and source.is_file() and packaged != source.read_bytes():
            problems.append(f"/usr/share/doc/remmods/{name} differs from the repository's {name}")
    binary = contents.get(BINARY)
    if binary is None:
        problems.append(f"no {BINARY} in the payload")
    else:
        counts = token_counts(binary)
        summary["bundle_tokens"] = counts
        if counts["UNK"] != 0:
            problems.append(f"binary still has __TAURI_BUNDLE_TYPE_VAR_UNK ({counts['UNK']}x): not patched")
        others = {counts[t] for t in TOKENS if t != expected}
        if len(others) != 1 or counts[expected] != next(iter(others)) + 1:
            problems.append(f"binary is not patched as {expected}: token counts {counts}")
    return problems


def inspect(path):
    data = Path(path).read_bytes()
    if data.startswith(b"!<arch>\n"):
        summary, contents = inspect_deb(data)
    elif data.startswith(RPM_LEAD_MAGIC):
        summary, contents = inspect_rpm(data)
    else:
        raise BundleError(f"{path}: neither a deb nor an rpm")
    summary["file"] = Path(path).name
    summary["size"] = len(data)
    summary["file_count"] = len(summary["files"])
    if BINARY in contents:
        summary["binary_size"] = len(contents[BINARY])
        summary.setdefault("bundle_tokens", token_counts(contents[BINARY]))
    return summary, contents


def print_summary(summary):
    print(f"{summary['file']}: {summary['kind']} {summary['name']} {summary['version']}, {summary['size']} bytes")
    print(f"  payload: {summary['payload']}, {summary['file_count']} files")
    if "binary_size" in summary:
        print(f"  {BINARY}: {summary['binary_size']} bytes, bundle tokens {summary['bundle_tokens']}")
    print("  requires: " + ", ".join(summary["requires"]))
    for path in summary["files"]:
        print(f"  {path}")


# ---------------------------------------------------------------- self-test


def ar_archive(members):
    out = bytearray(b"!<arch>\n")
    for name, blob in members:
        out += f"{name:<16}{0:<12}{0:<6}{0:<6}{100644:<8}{len(blob):<10}".encode() + b"`\n" + blob
        if len(blob) & 1:
            out += b"\n"
    return bytes(out)


def tar_gz(files):
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w:gz") as tar:
        for name, blob in files.items():
            info = tarfile.TarInfo(name)
            info.size = len(blob)
            tar.addfile(info, io.BytesIO(blob))
    return buffer.getvalue()


def fixture_files(token, repo):
    binary = b"\x7fELF" + TOKEN_PREFIX + token + b"\0" + b"".join(TOKEN_PREFIX + t.encode() for t in TOKENS)
    desktop = "[Desktop Entry]\nName=RemMods\nExec=remmods\nStartupWMClass=remmods\nActions=safe-graphics;\n\n"
    desktop += "[Desktop Action safe-graphics]\nExec=remmods --safe-graphics\n"
    files = {"./usr/bin/remmods": binary, "./usr/share/applications/RemMods.desktop": desktop.encode()}
    for name in NOTICES:
        files[f"./usr/share/doc/remmods/{name}"] = (repo / name).read_bytes()
    for size in ICON_SIZES:
        files[f"./usr/share/icons/hicolor/{size}/apps/remmods.png"] = b"\x89PNG"
    return files


def fixture_deb(token, depends, repo):
    control = f"Package: rem-mods\nVersion: 1.2.3\nArchitecture: amd64\nDepends: {depends}\n"
    return ar_archive([
        ("debian-binary", b"2.0\n"),
        ("control.tar.gz", tar_gz({"./control": control.encode()})),
        ("data.tar.gz", tar_gz(fixture_files(token, repo))),
    ])


def rpm_header_bytes(entries):
    index, store = b"", b""
    for tag, kind, value in entries:
        if kind == 6:
            blob, count = value.encode() + b"\0", 1
        elif kind == 8:
            blob, count = b"".join(v.encode() + b"\0" for v in value), len(value)
        else:
            while len(store) % 4:
                store += b"\0"
            blob, count = struct.pack(f">{len(value)}I", *value), len(value)
        index += struct.pack(">IIII", tag, kind, len(store), count)
        store += blob
    return RPM_HEADER_MAGIC + b"\0" * 4 + struct.pack(">II", len(entries), len(store)) + index + store


def cpio_newc(files):
    out = bytearray()
    for i, (name, blob) in enumerate(list(files.items()) + [("TRAILER!!!", b"")]):
        mode = 0o100644 if name != "TRAILER!!!" else 0
        raw = name.encode() + b"\0"
        fields = [i + 1, mode, 0, 0, 1, 0, len(blob), 0, 0, 0, 0, len(raw), 0]
        out += b"070701" + "".join(f"{f:08X}" for f in fields).encode() + raw
        out += b"\0" * (-len(out) % 4)
        out += blob
        out += b"\0" * (-len(out) % 4)
    return bytes(out)


def fixture_rpm(token, requires, repo):
    files = fixture_files(token, repo)
    paths = ["/" + p.lstrip("./") for p in files]
    dirs = sorted({p.rsplit("/", 1)[0] + "/" for p in paths})
    header = rpm_header_bytes([
        (TAG_NAME, 6, "rem-mods"),
        (TAG_VERSION, 6, "1.2.3"),
        (TAG_RELEASE, 6, "1"),
        (TAG_REQUIRENAME, 8, requires),
        (TAG_DIRINDEXES, 4, [dirs.index(p.rsplit("/", 1)[0] + "/") for p in paths]),
        (TAG_BASENAMES, 8, [p.rsplit("/", 1)[1] for p in paths]),
        (TAG_DIRNAMES, 8, dirs),
        (TAG_PAYLOADCOMPRESSOR, 6, "gzip"),
    ])
    signature = rpm_header_bytes([])
    signature += b"\0" * (-len(signature) % 8)
    lead = RPM_LEAD_MAGIC + b"\0" * 92
    return lead + signature + header + gzip.compress(cpio_newc(files))


def self_test():
    good_depends = ", ".join(DEB_DEPENDS)
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        cases = [
            ("good deb", fixture_deb(b"DEB", good_depends, REPO), "deb", True),
            ("deb patched as RPM", fixture_deb(b"RPM", good_depends, REPO), "deb", False),
            ("deb left unpatched", fixture_deb(b"UNK", good_depends, REPO), "deb", False),
            ("deb without libssl", fixture_deb(b"DEB", "libc6 (>= 2.39), xdg-utils, libwebkit2gtk-4.1-0, libgtk-3-0", REPO), "deb", False),
            ("good rpm", fixture_rpm(b"RPM", list(RPM_REQUIRES), REPO), "rpm", True),
            ("rpm patched as APP", fixture_rpm(b"APP", list(RPM_REQUIRES), REPO), "rpm", False),
            ("rpm without the glibc floor", fixture_rpm(b"RPM", list(RPM_REQUIRES[1:]), REPO), "rpm", False),
            ("deb checked as rpm", fixture_deb(b"DEB", good_depends, REPO), "rpm", False),
        ]
        for label, blob, kind, should_pass in cases:
            path = root / f"case.{kind}"
            path.write_bytes(blob)
            summary, contents = inspect(path)
            problems = check_expectations(kind, summary, contents, REPO)
            if should_pass and problems:
                raise AssertionError(f"{label}: unexpected problems {problems}")
            if not should_pass and not problems:
                raise AssertionError(f"{label}: expected a failure")
        (root / "good.rpm").write_bytes(fixture_rpm(b"RPM", list(RPM_REQUIRES), REPO))
        summary, _ = inspect(root / "good.rpm")
        assert "/usr/bin/remmods" in summary["payload_files"], "rpm payload listing"
    print("inspect-bundle: self-test passed")
    return 0


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("file", nargs="?", type=Path)
    parser.add_argument("--expect", choices=("deb", "rpm"))
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)
    if args.self_test:
        return self_test()
    if args.file is None:
        parser.error("FILE is required")
    try:
        summary, contents = inspect(args.file)
        problems = check_expectations(args.expect, summary, contents, REPO) if args.expect else []
    except (BundleError, OSError, ValueError, struct.error, tarfile.TarError, EOFError) as error:
        print(f"inspect-bundle: {args.file}: {error}", file=sys.stderr)
        return 1
    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        print_summary(summary)
    for problem in problems:
        print(f"inspect-bundle: FAIL: {problem}", file=sys.stderr)
    if problems:
        return 1
    if args.expect:
        print(f"inspect-bundle: {args.file.name} meets the {args.expect} expectations")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
