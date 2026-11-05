#!/usr/bin/env python3
"""Builds crafted zip archives for RemMods' archive reader tests (docs/PLAN.md §9). Standard library only.

Usage: python3 -I scripts/make-archive-fixtures.py <empty dir>

Writes one zip per hostile or odd case plus `manifest.json`, a list of
`{ "file", "expect": "ok" | "unsafe" | "error", "error"?, "problems"?, "files"? }`:
  ok      opens, no unsafe entry; `files` are the normalized file paths ("/" joined)
  unsafe  opens, but an entry fails normalization; `problems` are EntryProblem names
  error   refused on open or on extraction; `error` is an ArchiveError variant, or for
          Unsupported the reason (e.g. "MultiVolume")
The Rust test `archive_fixtures` (REMMODS_ARCHIVE_FIXTURES=<dir>) checks every item on both OSes.

The zips are written by hand (not with zipfile) so names, flags, attributes, CRCs and disk numbers
can be forged. Pak contents are a few dummy bytes, never real game data. 7z and RAR cases stay in
the Rust tests: Python has no writer for them.

Refuses a missing or non-empty <out>. Writes only below <out> and never replaces a file.
"""

import json
import os
import struct
import sys
import unicodedata
import zlib

DATA = b"dummy"
UTF8_FLAG = 0x0800
ENCRYPTED_FLAG = 0x0001
# 1980-01-01 00:00:00 in DOS format, so the files are byte-identical on every run.
DOS_TIME = 0
DOS_DATE = (0 << 9) | (1 << 5) | 1
UNIX = 3 << 8


class FixtureError(Exception):
    pass


class Item:
    """One zip entry. `crc`, `disk` and `stored_data` override what a honest writer would put."""

    def __init__(self, name, data=DATA, flags=0, mode=None, crc=None, disk=0, attr=None):
        self.name = name if isinstance(name, bytes) else name.encode("utf-8")
        self.data = data
        self.flags = flags
        self.crc = zlib.crc32(data) & 0xFFFFFFFF if crc is None else crc
        self.disk = disk
        self.external = attr if attr is not None else ((mode << 16) if mode is not None else 0)
        self.version_made = UNIX if mode is not None else 0


def local_record(item):
    head = struct.pack(
        "<IHHHHHIIIHH",
        0x04034B50, 20, item.flags, 0, DOS_TIME, DOS_DATE,
        item.crc, len(item.data), len(item.data), len(item.name), 0,
    )
    return head + item.name + item.data


def central_record(item, offset):
    head = struct.pack(
        "<IHHHHHHIIIHHHHHII",
        0x02014B50, item.version_made | 20, 20, item.flags, 0, DOS_TIME, DOS_DATE,
        item.crc, len(item.data), len(item.data), len(item.name), 0, 0,
        item.disk, 0, item.external, offset,
    )
    return head + item.name


def build_zip(items, eocd_disk=0, prefix=b"", lead=b""):
    """A stored-only zip. `lead` replaces the first bytes (a spanned marker); `prefix` is junk before the zip."""
    body = bytearray()
    central = bytearray()
    for item in items:
        central += central_record(item, len(lead) + len(body))
        body += local_record(item)
    start = len(lead) + len(body)
    end = struct.pack(
        "<IHHHHIIH", 0x06054B50, eocd_disk, eocd_disk, len(items), len(items), len(central), start, 0
    )
    return prefix + lead + bytes(body) + bytes(central) + end


def inner_zip():
    return build_zip([Item("Inner/inner.pak")])


def legacy_text(raw):
    """What `Auto` shows for unflagged non-UTF-8 bytes: CP437 (Python's table is IBM CP437 too)."""
    return unicodedata.normalize("NFC", raw.decode("cp437"))


def cases():
    """(file, zip bytes or raw bytes, manifest item without `file`)"""
    cp437 = b"Cool/\x81\xe1.pak"
    sjis = b"Cool/\x83\x65\x83\x58\x83\x67.pak"
    utf8_name = "Cool/\u30c6\u30b9\u30c8.pak"
    plain = build_zip([Item("Cool/x.pak")])
    out = []

    def add(file, data, expect, **extra):
        out.append((file, data, {"expect": expect, **extra}))

    def unsafe(file, name, problem):
        add(file, build_zip([Item(name)]), "unsafe", problems=[problem])

    unsafe("traversal-dotdot.zip", "../evil.pak", "Traversal")
    unsafe("traversal-backslash.zip", "..\\evil.pak", "Traversal")
    unsafe("absolute.zip", "/abs/evil.pak", "Absolute")
    unsafe("drive.zip", "C:\\x.pak", "Drive")
    unsafe("unc.zip", "\\\\server\\share\\x.pak", "Unc")
    add("backslash-ok.zip", build_zip([Item("Cool\\x.pak")]), "ok", files=["Cool/x.pak"])
    add("dup-identical.zip", build_zip([Item("Cool/x.pak"), Item("Cool/x.pak")]), "ok", files=["Cool/x.pak"])
    add(
        "dup-differs.zip",
        build_zip([Item("Cool/x.pak", b"first"), Item("Cool/x.pak", b"second")]),
        "ok",
        files=["Cool/x.pak"],
    )
    add("case-clash.zip", build_zip([Item("Cool/A.pak"), Item("Cool/a.pak")]), "ok", files=["Cool/A.pak", "Cool/a.pak"])
    add(
        "symlink.zip",
        build_zip([Item("Cool/link.pak", b"/etc/passwd", mode=0o120777), Item("Cool/x.pak", mode=0o100644)]),
        "ok",
        files=["Cool/x.pak"],
    )
    add(
        "fifo.zip",
        build_zip([Item("Cool/pipe.pak", b"", mode=0o010644), Item("Cool/x.pak", mode=0o100644)]),
        "ok",
        files=["Cool/x.pak"],
    )
    add("legacy-cp437.zip", build_zip([Item(cp437)]), "ok", files=[legacy_text(cp437)])
    add("legacy-sjis.zip", build_zip([Item(sjis)]), "ok", files=[legacy_text(sjis)])
    add("utf8-flag.zip", build_zip([Item(utf8_name, flags=UTF8_FLAG)]), "ok", files=[utf8_name])
    add("encrypted.zip", build_zip([Item("Cool/x.pak", flags=ENCRYPTED_FLAG)]), "error", error="Encrypted")
    bad_crc = bytearray(plain)
    bad_crc[30 + len("Cool/x.pak")] ^= 0xFF  # first payload byte, after the 30-byte local header and the name
    add("corrupt-crc.zip", bytes(bad_crc), "error", error="Corrupt")
    add("truncated.zip", plain[: len(plain) - 30], "error", error="Corrupt")
    add("multi.z01", plain, "error", error="MultiVolume")
    add("multi.zip", build_zip([Item("Cool/x.pak", disk=1)], eocd_disk=1), "error", error="MultiVolume")
    add("spanned.zip", build_zip([Item("Cool/x.pak")], lead=b"PK\x07\x08"), "error", error="MultiVolume")
    add("sfx.zip", build_zip([Item("Cool/x.pak")], prefix=b"MZ" + bytes(62)), "error", error="NotAnArchive")
    add("nested.zip", build_zip([Item("inner.zip", inner_zip())]), "ok", files=["inner.zip"])
    add(
        "variants.zip",
        build_zip([Item("Red/x.pak"), Item("Blue/x.pak")]),
        "ok",
        files=["Red/x.pak", "Blue/x.pak"],
    )
    add("script.zip", build_zip([Item("Scripts/main.lua")]), "ok", files=["Scripts/main.lua"])
    add(
        "logic.zip",
        build_zip([Item("Cool/config.lua"), Item("Cool/x.pak")]),
        "ok",
        files=["Cool/config.lua", "Cool/x.pak"],
    )
    segment = "Content/Paks/~mods/Cool/x.pak"
    add("segment.zip", build_zip([Item(segment)]), "ok", files=[segment])
    # Install judges these names (Windows rules); the reader only normalizes. They sit below a folder
    # because a top-level `a:b.pak` already counts as a drive path.
    add(
        "invalid-names.zip",
        build_zip([Item("Cool/CON.pak"), Item("Cool/a:b.pak")]),
        "ok",
        files=["Cool/CON.pak", "Cool/a:b.pak"],
    )
    return out


def prepare_out_dir(out):
    if os.path.lexists(out):
        if not os.path.isdir(out) or os.path.islink(out):
            raise FixtureError(f"not a folder: {out}")
        if os.listdir(out):
            raise FixtureError(f"refusing to write into a folder that is not empty: {out}")
    else:
        os.makedirs(out)
    return os.path.abspath(out)


def write_new_file(base, name, content):
    if os.sep in name or "/" in name or name in ("", ".", ".."):
        raise FixtureError(f"bad fixture name: {name!r}")
    # "xb": never replaces a file; binary mode keeps bytes (and LF) identical on Windows.
    with open(os.path.join(base, name), "xb") as file:
        file.write(content)


def make(out):
    base = prepare_out_dir(out)
    manifest = []
    for name, data, expected in cases():
        write_new_file(base, name, data)
        # multi.z01 is a name-only case (the part file); the manifest lists it like the others.
        manifest.append({"file": name, **expected})
    text = json.dumps(manifest, indent=2, ensure_ascii=False) + "\n"
    write_new_file(base, "manifest.json", text.encode("utf-8"))
    return base


def usage(stream):
    print(__doc__.strip(), file=stream)


def main(argv):
    if len(argv) == 2 and argv[1] in ("-h", "--help"):
        usage(sys.stdout)
        return 0
    if len(argv) != 2:
        usage(sys.stderr)
        return 2
    try:
        result = make(argv[1])
    except (FixtureError, OSError) as error:
        print(f"make-archive-fixtures: {error}", file=sys.stderr)
        return 1
    print(result)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
