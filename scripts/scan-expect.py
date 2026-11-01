#!/usr/bin/env python3
"""Prints the scan log lines RemMods should write for a Paks folder, computed from a
scripts/paks-snapshot.sh TSV by the scan rules (docs/PLAN.md §4, §7). Standard library only; reads
the TSV and writes stdout, nothing else.

Usage: python3 -I scripts/scan-expect.py <paks-snapshot.tsv> | LC_ALL=C sort > local/scan-expect.txt

Lines (TAB-separated, keys and paths escaped like the app's log):
  scan-row        key, state, livePak/offPak/liveCompanion/offCompanion, size, conflicting keys
  scan-archive    relPath, size
  scan-unmanaged  kind, relPath, liveFiles
  scan-leftover   relPath
Not modelled: modinfo.json, issues and file ids, which the lines do not carry.

Exits 2 instead of guessing when the snapshot holds a symlink (or another special file) or a name
that is not UTF-8.
"""

import re
import sys
import unicodedata

HEADER = b"# paks-snapshot v1"
ROOT_KINDS = (("Standard", "Mods"), ("Legacy", "~mods"), ("Logic", "LogicMods"))
PAYLOAD_EXTENSIONS = ("pak", "ucas", "utoc", "sig")
ARCHIVE_SUFFIXES = (".zip", ".7z", ".rar")
GAME_FILE_SUFFIXES = (".pak", ".ucas", ".utoc", ".sig")
DISABLED_PREFIX = ".disabled_"
TEMP_MARK = b"_tmp_"
HEX_DIGITS = frozenset(b"0123456789abcdefABCDEF")
ASCII_LOWER = str.maketrans("ABCDEFGHIJKLMNOPQRSTUVWXYZ", "abcdefghijklmnopqrstuvwxyz")
BIDI_CONTROLS = frozenset(
    [0x061C, 0x200E, 0x200F, 0x202A, 0x202B, 0x202C, 0x202D, 0x202E, 0x2066, 0x2067, 0x2068, 0x2069]
)
TSV_ESCAPES = {b"\\t": b"\t", b"\\n": b"\n"}


class SnapshotError(Exception):
    """The snapshot cannot be modelled without guessing (exit status 2)."""


class Directory:
    def __init__(self, name):
        self.name = name
        self.dirs = {}
        self.files = {}


class File:
    def __init__(self, name, size):
        self.name = name
        self.size = size


# Names and keys --------------------------------------------------------------------------------


def lower_ascii(text):
    return text.translate(ASCII_LOWER)


def fold_key(text):
    if text.isascii():
        return text.lower()
    return unicodedata.normalize("NFC", unicodedata.normalize("NFD", text).casefold())


def key_text(name):
    # Names are valid UTF-8 here (anything else exits 2), so only "\" needs escaping.
    return name.replace("\\", "\\\\")


def order_key(text):
    return (fold_key(text), text.encode("utf-8"))


def display_escaped(text):
    return "".join(
        f"\\u{{{ord(c):04X}}}"
        if unicodedata.category(c) == "Cc" or ord(c) in BIDI_CONTROLS
        else c
        for c in text
    )


def rel_path(*names):
    return "/".join(display_escaped(key_text(name)) for name in names)


def payload_kind(name):
    """(extension, live) for a payload file name, else None."""
    low = lower_ascii(name)
    for extension in PAYLOAD_EXTENSIONS:
        if low.endswith("." + extension):
            return extension, True
        if low.endswith("." + extension + "-x"):
            return extension, False
    return None


def payload_stem(name):
    extension, live = payload_kind(name)
    return name[: len(name) - len(extension) - 1 - (0 if live else 2)]


def is_archive(name):
    return lower_ascii(name).endswith(ARCHIVE_SUFFIXES)


def is_game_file(name):
    low = lower_ascii(name)
    if low in ("global.ucas", "global.utoc"):
        return True
    return low.startswith("pakchunk") and "-windows" in low and low.endswith(GAME_FILE_SUFFIXES)


def is_install_temp_name(name):
    raw = name.encode("utf-8")
    mark = raw.rfind(TEMP_MARK)
    if not raw.startswith(b"_") or mark < 0:
        return False
    digits = raw[mark + len(TEMP_MARK) :]
    return len(digits) == 32 and all(byte in HEX_DIGITS for byte in digits)


def has_disabled_prefix_ascii(name):
    # bytes.lower() changes ASCII letters only.
    return name.encode("utf-8")[: len(DISABLED_PREFIX)].lower() == DISABLED_PREFIX.encode("ascii")


def root_kind(name):
    low = lower_ascii(name)
    for prefix, folder in ROOT_KINDS:
        if low == lower_ascii(folder):
            return prefix
    return None


# Snapshot parsing ------------------------------------------------------------------------------


def unescape_tsv(raw):
    return re.sub(rb"\\[tn]", lambda match: TSV_ESCAPES[match.group(0)], raw)


def decode_name(raw, line_number):
    try:
        return raw.decode("utf-8")
    except UnicodeDecodeError as error:
        raise SnapshotError(f"line {line_number}: name is not UTF-8: {raw!r}") from error


def parse_entry(line, line_number):
    """(components, is_dir, size) for one TSV row."""
    columns = line.split(b"\t")
    if len(columns) != 4:
        raise SnapshotError(f"line {line_number}: expected 4 columns, found {len(columns)}")
    raw_path, size, _mtime, kind = columns
    if kind.startswith(b"link:"):
        raise SnapshotError(
            f"line {line_number}: symlink {raw_path!r}: the snapshot does not follow links, "
            "so their effect on the scan cannot be modelled"
        )
    if kind.startswith(b"type:"):
        raise SnapshotError(f"line {line_number}: special file {raw_path!r} ({kind.decode()})")
    is_dir = raw_path.endswith(b"/")
    if is_dir != (kind == b"dir"):
        raise SnapshotError(f"line {line_number}: folder marker and kind column disagree")
    path = decode_name(unescape_tsv(raw_path[:-1] if is_dir else raw_path), line_number)
    components = path.split("/")
    if any(part in ("", ".", "..") for part in components):
        raise SnapshotError(f"line {line_number}: unexpected path {path!r}")
    if is_dir:
        return components, True, 0
    if not size.isdigit():
        raise SnapshotError(f"line {line_number}: size {size!r} is not a number")
    return components, False, int(size)


def parse_snapshot(data):
    """The Paks folder as a Directory tree."""
    lines = data.split(b"\n")
    if lines and lines[-1] == b"":
        lines.pop()
    if len(lines) < 3 or lines[0] != HEADER or not lines[1].startswith(b"# root: "):
        raise SnapshotError("not a paks-snapshot.sh v1 file (header missing)")
    if not lines[2].startswith(b"# columns: "):
        raise SnapshotError("not a paks-snapshot.sh v1 file (columns line missing)")
    paks = Directory("")
    for line_number, line in enumerate(lines[3:], start=4):
        components, is_dir, size = parse_entry(line, line_number)
        parent = paks
        for part in components[:-1]:
            parent = parent.dirs.setdefault(part, Directory(part))
        name = components[-1]
        if is_dir:
            parent.dirs.setdefault(name, Directory(name))
        else:
            parent.files[name] = File(name, size)
    return paks


# Scan model ------------------------------------------------------------------------------------


class Counts:
    def __init__(self):
        self.live_pak = 0
        self.off_pak = 0
        self.live_companion = 0
        self.off_companion = 0

    def add(self, extension, live):
        if extension == "pak":
            if live:
                self.live_pak += 1
            else:
                self.off_pak += 1
        elif live:
            self.live_companion += 1
        else:
            self.off_companion += 1

    def live_files(self):
        return self.live_pak + self.live_companion

    def text(self):
        return f"{self.live_pak}/{self.off_pak}/{self.live_companion}/{self.off_companion}"


class Row:
    def __init__(self, key, order, counts, size, live_paks, prefixed):
        self.key = key
        self.order = order
        self.counts = counts
        self.size = size
        self.live_paks = live_paks
        self.state = state_for(counts, prefixed)
        self.conflicts = []


def state_for(counts, logic_prefixed):
    # A prefixed Logic folder is off whatever it holds; live paks in it give an issue, not a state.
    if logic_prefixed:
        return "disabled"
    if counts.live_pak > 0:
        return "partial" if counts.off_pak > 0 else "enabled"
    return "disabled" if counts.off_pak > 0 else "noPakFiles"


def add_file(counts, live_paks, file):
    kind = payload_kind(file.name)
    if kind is None:
        return
    counts.add(*kind)
    if kind == ("pak", True):
        live_paks.add(fold_key(file.name))


def walk(folder):
    """(Counts, size, fold keys of live .pak names) of every file below folder, at any depth."""
    counts, size, live_paks = Counts(), 0, set()
    stack = [folder]
    while stack:
        current = stack.pop()
        for file in current.files.values():
            size += file.size
            add_file(counts, live_paks, file)
        stack.extend(current.dirs.values())
    return counts, size, live_paks


def folder_row(prefix, root, folder):
    counts, size, live_paks = walk(folder)
    folder_key = key_text(folder.name)
    prefixed = prefix == "Logic" and has_disabled_prefix_ascii(folder.name)
    logical = folder_key[len(DISABLED_PREFIX) :] if prefixed else folder_key
    order = (prefix, order_key(logical), key_text(root.name).encode(), folder_key.encode())
    return Row(f"{prefix}:{logical}", order, counts, size, live_paks, prefixed)


def loose_rows(prefix, folder, root):
    groups = {}
    for file in root.files.values():
        if payload_kind(file.name) is not None:
            groups.setdefault(fold_key(payload_stem(file.name)), []).append(file)
    rows = []
    for files in groups.values():
        if not any(payload_kind(file.name)[0] != "sig" for file in files):
            continue
        files.sort(key=lambda file: order_key(key_text(file.name)))
        stem = key_text(payload_stem(files[0].name))
        counts, live_paks = Counts(), set()
        for file in files:
            add_file(counts, live_paks, file)
        size = sum(file.size for file in files)
        order = ("Loose", order_key(stem), key_text(root.name).encode(), b"")
        rows.append(Row(f"Loose:{folder}/{stem}", order, counts, size, live_paks, False))
    return rows


class Scan:
    def __init__(self):
        self.rows = []
        self.archives = []
        self.unmanaged = []
        self.leftovers = []

    def scan_root(self, prefix, folder, root):
        for child in root.dirs.values():
            if is_install_temp_name(child.name):
                live = walk(child)[0].live_files()
                if live > 0:
                    self.unmanaged.append(("tempFolder", rel_path(root.name, child.name), live))
                else:
                    self.leftovers.append(rel_path(root.name, child.name))
            else:
                self.rows.append(folder_row(prefix, root, child))
        for file in root.files.values():
            if payload_kind(file.name) is None and is_archive(file.name):
                self.archives.append((rel_path(root.name, file.name), file.size))
        self.rows.extend(loose_rows(prefix, folder, root))

    def scan_top_level(self, paks):
        roots = []
        for child in paks.dirs.values():
            prefix = root_kind(child.name)
            if prefix is not None:
                roots.append((prefix, child))
                continue
            live = walk(child)[0].live_files()
            if live > 0:
                self.unmanaged.append(("folder", rel_path(child.name), live))
        for file in paks.files.values():
            kind = payload_kind(file.name)
            if is_game_file(file.name):
                continue
            if kind is not None and kind[1] and kind[0] != "sig":
                self.unmanaged.append(("paksFile", rel_path(file.name), 1))
            elif kind is None and is_archive(file.name):
                self.archives.append((rel_path(file.name), file.size))
        return roots

    def link_conflicts(self):
        loaded = [row for row in self.rows if row.state != "unknown" and row.counts.live_pak > 0]
        for index, row in enumerate(loaded):
            for other in loaded[index + 1 :]:
                if row.live_paks & other.live_paks:
                    row.conflicts.append(other.key)
                    other.conflicts.append(row.key)


def scan(paks):
    result = Scan()
    roots = result.scan_top_level(paks)
    kind_order = {prefix: index for index, (prefix, _) in enumerate(ROOT_KINDS)}
    folders = dict(ROOT_KINDS)
    roots.sort(
        key=lambda item: (
            kind_order[item[0]],
            item[1].name != folders[item[0]],
            key_text(item[1].name).encode(),
        )
    )
    for prefix, root in roots:
        result.scan_root(prefix, folders[prefix], root)
    result.link_conflicts()
    return result


# Output ----------------------------------------------------------------------------------------


def row_line(row):
    conflicts = ",".join(display_escaped(key) for key in sorted(row.conflicts)) or "-"
    return "\t".join(
        ["scan-row", display_escaped(row.key), row.state, row.counts.text(), str(row.size), conflicts]
    )


def output_lines(result):
    kind_order = {"Standard": 0, "Legacy": 1, "Logic": 2, "Loose": 3}
    rows = sorted(result.rows, key=lambda row: (kind_order[row.order[0]],) + row.order[1:])
    lines = [row_line(row) for row in rows]
    for path, size in sorted(result.archives, key=lambda item: order_key(item[0])):
        lines.append(f"scan-archive\t{display_escaped(path)}\t{size}")
    for kind, path, live in sorted(result.unmanaged, key=lambda item: order_key(item[1])):
        lines.append(f"scan-unmanaged\t{kind}\t{display_escaped(path)}\t{live}")
    for path in sorted(result.leftovers, key=order_key):
        lines.append(f"scan-leftover\t{display_escaped(path)}")
    return lines


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
        with open(argv[1], "rb") as file:
            data = file.read()
    except OSError as error:
        print(f"scan-expect: {argv[1]}: {error.strerror}", file=sys.stderr)
        return 1
    try:
        result = scan(parse_snapshot(data))
    except SnapshotError as error:
        print(f"scan-expect: {error}", file=sys.stderr)
        return 2
    sys.stdout.write("".join(line + "\n" for line in output_lines(result)))
    print(
        f"scan-expect: {len(result.rows)} rows, {len(result.archives)} archives, "
        f"{len(result.unmanaged)} unmanaged, {len(result.leftovers)} leftovers",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
