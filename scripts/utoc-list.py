#!/usr/bin/env python3
"""Independent lister of IoStore .utoc/.utoc-x files (docs: local/SPEC-P2B.md §4). Standard library only.

Usage: python3 -I scripts/utoc-list.py [--hashes] <dir>

Finds every regular file below <dir> whose name ends in .utoc or .utoc-x (ASCII case-insensitive),
never descending into symlinked folders. Prints one line per file entry, "<rel>\\t<display path>",
or one line "<rel>\\t!<status>" for a container that cannot be listed (notIoStore,
unsupportedVersion, encrypted, noIndex, malformed, tooLarge, unreadable). <rel> is the path below
<dir> with "/" separators. With --hashes each file line gets a third column: the 20-byte content
hash of its chunk in lowercase hex (the chunk metas after the directory index: TocEntryCount
entries of 33 bytes up to version 7, 24 bytes from version 8, hash first), or "-" when the metas
do not fit in the file or the hash is all zero. Lines are sorted by UTF-8 bytes, LF line
endings. This is the Python side of scripts/utoc-compare.sh; it is written from the format
description, not from the Rust reader, so the two cross-check each other.

Read-only: files are only opened with open(path, "rb"). Exit 0 after printing, 2 on bad arguments.
"""

import os
import struct
import sys

MAGIC = b"-==--==--==--==-"
HEADER_SIZE = 144
MAX_INDEX_BYTES = 64 * 1024 * 1024
MAX_PATH_BYTES = 4096
MAX_LISTING_BYTES = 256 * 1024 * 1024
INVALID = 0xFFFFFFFF
FLAG_ENCRYPTED = 0x02
FLAG_SIGNED = 0x04
FLAG_INDEXED = 0x08
HASH_BYTES = 20


class Status(Exception):
    """A container that cannot be listed; args[0] is the status token."""


def malformed():
    return Status("malformed")


def u32(data, offset):
    return struct.unpack_from("<I", data, offset)[0]


def i32(data, offset):
    return struct.unpack_from("<i", data, offset)[0]


def parse_header(head):
    if len(head) < HEADER_SIZE or head[:16] != MAGIC:
        raise Status("notIoStore")
    version = head[16]
    if not 2 <= version <= 8:
        raise Status("unsupportedVersion")
    if u32(head, 20) != HEADER_SIZE or u32(head, 32) != 12:
        raise malformed()
    flags = head[80]
    if flags & FLAG_ENCRYPTED:
        raise Status("encrypted")
    index_size = u32(head, 48)
    if not flags & FLAG_INDEXED or index_size == 0:
        raise Status("noIndex")
    if index_size > MAX_INDEX_BYTES:
        raise Status("tooLarge")
    return {
        "version": version,
        "entries": u32(head, 24),
        "blocks": u32(head, 28),
        "method_count": u32(head, 36),
        "method_length": u32(head, 40),
        "index_size": index_size,
        "seeds": u32(head, 84),
        "overflow": u32(head, 96),
        "flags": flags,
    }


def index_offset(header):
    version = header["version"]
    entries = header["entries"]
    offset = HEADER_SIZE
    offset += entries * 12 + entries * 10
    if version >= 4:
        offset += header["seeds"] * 4
    if version >= 5:
        offset += header["overflow"] * 4
    offset += header["blocks"] * 12
    offset += header["method_count"] * header["method_length"]
    if offset >= 1 << 64:
        raise malformed()
    return offset


class Reader:
    def __init__(self, data):
        self.data = data
        self.pos = 0

    def remaining(self):
        return len(self.data) - self.pos

    def take(self, count):
        if count < 0 or count > self.remaining():
            raise malformed()
        chunk = self.data[self.pos : self.pos + count]
        self.pos += count
        return chunk

    def int32(self):
        return i32(self.take(4), 0)

    def count(self, entry_size):
        n = self.int32()
        if n < 0 or n * entry_size > self.remaining():
            raise malformed()
        return n

    def fstring(self):
        n = self.int32()
        if n == 0:
            return ""
        if n == -(1 << 31):
            raise malformed()
        if n > 0:
            raw = self.take(n)
            if raw[-1] != 0:
                raise malformed()
            return raw[:-1].decode("latin-1")
        raw = self.take(-n * 2)
        if raw[-2:] != b"\x00\x00":
            raise malformed()
        try:
            return raw[:-2].decode("utf-16-le", errors="strict")
        except UnicodeDecodeError:
            raise malformed() from None


def parse_index(data):
    reader = Reader(data)
    mount = reader.fstring()
    dirs = []
    for _ in range(reader.count(16)):
        dirs.append(struct.unpack("<IIII", reader.take(16)))
    files = []
    for _ in range(reader.count(12)):
        files.append(struct.unpack("<III", reader.take(12)))
    strings = [reader.fstring() for _ in range(reader.count(4))]
    if reader.remaining() != 0:
        raise malformed()
    return mount, dirs, files, strings


def display_path(raw):
    while raw.startswith("../"):
        raw = raw[3:]
    return raw.lstrip("/")


def walk(mount, dirs, files, strings, entry_count, metas):
    """Sorted (display path, hash text) pairs."""
    if "\\" in mount:
        raise malformed()
    if not dirs:
        if files:
            raise malformed()
        return []
    if not mount.endswith("/"):
        mount += "/"
    state = {"total": 0}

    def name_of(index):
        if index >= len(strings):
            raise malformed()
        text = strings[index]
        if text in ("", ".", "..") or any(c in text for c in "/\\\0"):
            raise malformed()
        return text

    def join(prefix, index, suffix):
        path = prefix + name_of(index) + suffix
        size = len(path.encode("utf-8"))
        state["total"] += size
        if size > MAX_PATH_BYTES or state["total"] > MAX_LISTING_BYTES:
            raise malformed()
        return path

    def link(value, length):
        if value == INVALID:
            return None
        if value >= length:
            raise malformed()
        return value

    dir_seen = [False] * len(dirs)
    file_seen = [False] * len(files)
    dir_seen[0] = True
    out = []
    stack = [(0, mount)]
    while stack:
        dir_index, prefix = stack.pop()
        _name, first_child, _sibling, first_file = dirs[dir_index]
        current = link(first_file, len(files))
        while current is not None:
            if file_seen[current]:
                raise malformed()
            file_seen[current] = True
            name, next_file, user_data = files[current]
            if user_data >= entry_count:
                raise malformed()
            out.append((display_path(join(prefix, name, "")), hash_of(metas, user_data)))
            current = link(next_file, len(files))
        child = link(first_child, len(dirs))
        while child is not None:
            if dir_seen[child]:
                raise malformed()
            dir_seen[child] = True
            child_name, _child, sibling, _first = dirs[child]
            stack.append((child, join(prefix, child_name, "/")))
            child = link(sibling, len(dirs))
    return sorted(out, key=lambda item: item[0].encode("utf-8"))


def list_container(path):
    """Returns the sorted (display path, hash text) pairs of one container; raises Status."""
    with open(path, "rb") as handle:
        total = os.fstat(handle.fileno()).st_size
        header = parse_header(handle.read(HEADER_SIZE))
        offset = index_offset(header)
        if header["flags"] & FLAG_SIGNED:
            if offset + 4 > total:
                raise malformed()
            handle.seek(offset)
            hash_size = i32(handle.read(4), 0)
            if not 0 <= hash_size <= 64:
                raise malformed()
            offset += 4 + 2 * hash_size + header["blocks"] * 20
        if offset + header["index_size"] > total:
            raise malformed()
        handle.seek(offset)
        data = handle.read(header["index_size"])
        if len(data) != header["index_size"]:
            raise malformed()
        metas = read_metas(handle, header, offset + header["index_size"], total)
    mount, dirs, files, strings = parse_index(data)
    return walk(mount, dirs, files, strings, header["entries"], metas)


def read_metas(handle, header, start, total):
    """The chunk-meta table and its entry size, or None when it does not fit in the file."""
    size = 24 if header["version"] >= 8 else 33
    length = header["entries"] * size
    if start + length > total:
        return None
    handle.seek(start)
    data = handle.read(length)
    if len(data) != length:
        return None
    return data, size


def hash_of(metas, chunk):
    if metas is None:
        return "-"
    data, size = metas
    digest = data[chunk * size : chunk * size + HASH_BYTES]
    if len(digest) != HASH_BYTES or digest == bytes(HASH_BYTES):
        return "-"
    return digest.hex()


def is_container(name):
    lower = name.lower()
    return lower.endswith(".utoc") or lower.endswith(".utoc-x")


def collect(directory, rel, found):
    try:
        entries = list(os.scandir(directory))
    except OSError:
        return
    for entry in entries:
        child_rel = entry.name if not rel else rel + "/" + entry.name
        try:
            is_dir = entry.is_dir(follow_symlinks=False)
        except OSError:
            continue
        if is_dir:
            collect(entry.path, child_rel, found)
        elif is_container(entry.name) and os.path.isfile(entry.path):
            found.append((child_rel, entry.path))


def encode(text):
    return text.encode("utf-8", errors="surrogateescape")


def main(argv):
    args = argv[1:]
    hashes = bool(args) and args[0] == "--hashes"
    if hashes:
        args = args[1:]
    if len(args) != 1 or args[0] in ("-h", "--help"):
        print(__doc__.strip(), file=sys.stderr if len(args) != 1 else sys.stdout)
        return 2 if len(args) != 1 else 0
    root = args[0]
    if not os.path.isdir(root):
        print(f"utoc-list: not a directory: {root}", file=sys.stderr)
        return 2
    found = []
    collect(root, "", found)
    lines = []
    for rel, path in found:
        try:
            paths = list_container(path)
        except Status as status:
            lines.append(f"{rel}\t!{status.args[0]}")
        except OSError:
            lines.append(f"{rel}\t!unreadable")
        else:
            for path_text, hash_text in paths:
                suffix = f"\t{hash_text}" if hashes else ""
                lines.append(f"{rel}\t{path_text}{suffix}")
    lines.sort(key=encode)
    out = sys.stdout.buffer
    for line in lines:
        out.write(encode(line) + b"\n")
    out.flush()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
