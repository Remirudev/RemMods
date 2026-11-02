#!/usr/bin/env python3
"""Builds test fixtures for RemMods (docs/PLAN.md §9). Standard library only.

Usage: python3 -I scripts/make-fixtures.py smoke <out>
       python3 -I scripts/make-fixtures.py --profiles <out>

smoke  A fake Steam library below <out> with TEKKEN 8 installed (appmanifest, both game exes) and a
       Paks folder holding every scan case of the P1 smoke test, plus the marker file
       `remmods-smoke-fixture` with the expected rows. Prints the Paks path; run the app with
       `remmods --smoke --paks <that path>`.

The Paks root also holds `install-smoke.zip` (one stored `SmokeInstall` folder with a pak) for the P4
install smoke; the marker names it under `install`.

Alpha, Loose and Beta also carry small crafted IoStore tables of contents (`.utoc`, TOC v5) so the
content view has something to list: Alpha and the loose mod ship the same asset, Beta's is disabled.
The base `pakchunk0-Windows.utoc` lists that asset too, so the two enabled ones replace a game file
and conflict (content conflicts count only game files).

--profiles  Writes sample files of the original app's profiles.json into <out> (for manual import
       checks, never into a real app folder): `profiles.json` (a Default profile with
       `EnabledMods: null`, an Exclusive-style list with `Standard:`-prefixed keys, one with
       `LogicMods:` keys and one with legacy keys without a colon) and `profiles-bom.json` (the same
       list with a UTF-8 byte order mark). Prints the folder.

Refuses when <out> exists and is not empty. Writes only below <out> and never replaces a file.
"""

import io
import json
import os
import struct
import sys
import zipfile

PAKS_PARTS = ("steamapps", "common", "TEKKEN 8", "Polaris", "Content", "Paks")
DATA = b"data"
MARKER_NAME = "remmods-smoke-fixture"

APPMANIFEST = (
    '"AppState"\n'
    "{\n"
    '\t"appid"\t\t"1778820"\n'
    '\t"Universe"\t\t"1"\n'
    '\t"name"\t\t"TEKKEN 8"\n'
    '\t"StateFlags"\t\t"4"\n'
    '\t"installdir"\t\t"TEKKEN 8"\n'
    '\t"LastUpdated"\t\t"1789022667"\n'
    '\t"LastPlayed"\t\t"1793490222"\n'
    '\t"SizeOnDisk"\t\t"139210798921"\n'
    '\t"StagingSize"\t\t"0"\n'
    '\t"buildid"\t\t"25221600"\n'
    '\t"LastOwner"\t\t"76561190000000001"\n'
    '\t"DownloadType"\t\t"3"\n'
    '\t"UpdateResult"\t\t"0"\n'
    '\t"BytesToDownload"\t\t"1144798160"\n'
    '\t"BytesDownloaded"\t\t"1144798160"\n'
    '\t"BytesToStage"\t\t"1333880600"\n'
    '\t"BytesStaged"\t\t"1333880600"\n'
    '\t"TargetBuildID"\t\t"0"\n'
    '\t"AutoUpdateBehavior"\t\t"0"\n'
    '\t"AllowOtherDownloadsWhileRunning"\t\t"0"\n'
    '\t"ScheduledAutoUpdate"\t\t"0"\n'
    '\t"InstalledDepots"\n'
    "\t{\n"
    '\t\t"1778821"\n'
    "\t\t{\n"
    '\t\t\t"manifest"\t\t"8720712599120052957"\n'
    '\t\t\t"size"\t\t"139068191529"\n'
    "\t\t}\n"
    '\t\t"3145890"\n'
    "\t\t{\n"
    '\t\t\t"manifest"\t\t"8461657819190922589"\n'
    '\t\t\t"size"\t\t"39846240"\n'
    '\t\t\t"dlcappid"\t\t"3145890"\n'
    "\t\t}\n"
    "\t}\n"
    '\t"SharedDepots"\n'
    "\t{\n"
    '\t\t"228989"\t\t"228980"\n'
    '\t\t"228990"\t\t"228980"\n'
    "\t}\n"
    '\t"UserConfig"\n'
    "\t{\n"
    '\t\t"language"\t\t"english"\n'
    "\t}\n"
    '\t"MountedConfig"\n'
    "\t{\n"
    '\t\t"language"\t\t"english"\n'
    "\t}\n"
    "}\n"
)

SMOKE_ARCHIVE = "install-smoke.zip"
SMOKE_INSTALL_KEY = "Standard:SmokeInstall"


def make_install_smoke_zip():
    """One stored mod folder for the P4 smoke install (fixed timestamps: the same bytes every run)."""
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w", zipfile.ZIP_STORED) as archive:
        for name, content in (("SmokeInstall/smoke.pak", DATA), ("SmokeInstall/readme.txt", b"smoke")):
            info = zipfile.ZipInfo(name, date_time=(2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_STORED
            archive.writestr(info, content)
    return buffer.getvalue()


ALPHA_MODINFO = (
    b'{"Version":"2.0","Category":"Character","Description":"Smoke fixture","Author":"RemMods"}'
)

CONTENT_MOUNT = "../../../Polaris/Content/"
INVALID = 0xFFFFFFFF
TOC_MAGIC = b"-==--==--==--==-"
METHOD_NAME_LENGTH = 32


def fstring(text):
    raw = text.encode("latin-1") + b"\x00"
    return struct.pack("<i", len(raw)) + raw


def directory_tree(paths):
    """Directories in pre-order (root first) as [name, [child indexes], [(file name, chunk)]]."""
    root = {"dirs": {}, "files": []}
    for chunk, path in enumerate(paths):
        *folders, name = path.split("/")
        node = root
        for folder in folders:
            node = node["dirs"].setdefault(folder, {"dirs": {}, "files": []})
        node["files"].append((name, chunk))
    flat = []

    def visit(name, node):
        index = len(flat)
        entry = [name, [], node["files"]]
        flat.append(entry)
        for child_name in sorted(node["dirs"]):
            entry[1].append(visit(child_name, node["dirs"][child_name]))
        return index

    visit(None, root)
    return flat


def directory_index(mount, paths):
    dirs = directory_tree(paths)
    strings = []

    def string_id(text):
        if text not in strings:
            strings.append(text)
        return strings.index(text)

    sibling = [INVALID] * len(dirs)
    for _name, children, _files in dirs:
        for left, right in zip(children, children[1:]):
            sibling[left] = right
    file_entries = []
    first_file = []
    for _name, _children, files in dirs:
        first_file.append(len(file_entries) if files else INVALID)
        for position, (name, chunk) in enumerate(files):
            following = len(file_entries) + 1 if position + 1 < len(files) else INVALID
            file_entries.append((string_id(name), following, chunk))
    out = bytearray(fstring(mount))
    out += struct.pack("<i", len(dirs))
    for index, (name, children, _files) in enumerate(dirs):
        name_id = INVALID if name is None else string_id(name)
        first_child = children[0] if children else INVALID
        out += struct.pack("<IIII", name_id, first_child, sibling[index], first_file[index])
    out += struct.pack("<i", len(file_entries))
    for entry in file_entries:
        out += struct.pack("<III", *entry)
    out += struct.pack("<i", len(strings))
    for text in strings:
        out += fstring(text)
    return bytes(out)


def make_toc(mount, paths):
    """A TOC v5 (flags 0x09, one Oodle method, unsigned) listing `paths` below `mount`."""
    index = directory_index(mount, paths)
    files = len(paths)
    entries = files + 1  # the container header chunk
    seeds = (files + 1) // 2
    out = bytearray(TOC_MAGIC)
    out += bytes([5, 0, 0, 0])
    out += struct.pack("<9I", 144, entries, files, 12, 1, METHOD_NAME_LENGTH, 0x10000, len(index), 1)
    out += struct.pack("<Q", 0x0123456789ABCDEF)
    out += bytes(16)
    out += bytes([0x09, 0, 0, 0])
    out += struct.pack("<I", seeds)
    out += struct.pack("<Q", 0xFFFFFFFFFFFFFFFF)
    out += struct.pack("<I", 0)
    out += bytes(144 - len(out))
    out += bytes(entries * 22 + seeds * 4 + files * 12)
    out += b"Oodle".ljust(METHOD_NAME_LENGTH, b"\x00")
    out += index
    out += bytes(entries * 33)
    return bytes(out)


ALPHA_TOC = make_toc(CONTENT_MOUNT, ["Smoke/Shared.uasset", "Smoke/Alpha.uasset"])
LOOSE_TOC = make_toc(CONTENT_MOUNT + "Smoke/", ["Shared.uasset", "Shared.ubulk"])
BETA_TOC = make_toc(CONTENT_MOUNT, ["Smoke/Shared.uasset"])
# The base game's own index: Smoke/Shared is a game file, so Alpha and Loose replace it.
BASE_TOC = make_toc("../../../", ["Polaris/Content/Smoke/Shared.uasset"])

# The scanner's expectations for the Paks tree below, read by `remmods --smoke --paks` (check
# scanFixture) and mirrored by a Rust test in remmods-core, so script and scanner cannot drift.
MARKER = """{
  "version": 1,
  "fixture": "remmods-smoke",
  "rows": [
    { "key": "Standard:Alpha", "state": "enabled", "conflicts": 0, "contentConflicts": 1 },
    { "key": "Standard:Beta", "state": "disabled", "conflicts": 0, "contentConflicts": 0 },
    { "key": "Standard:Gamma", "state": "partial", "conflicts": 0, "contentConflicts": 0 },
    { "key": "Standard:Delta", "state": "enabled", "conflicts": 1, "contentConflicts": 0 },
    { "key": "Standard:Epsilon", "state": "enabled", "conflicts": 1, "contentConflicts": 0 },
    { "key": "Standard:Empty", "state": "noPakFiles", "conflicts": 0, "contentConflicts": 0 },
    { "key": "Loose:Mods/Loose", "state": "enabled", "conflicts": 0, "contentConflicts": 1 },
    { "key": "Legacy:Legacy", "state": "enabled", "conflicts": 0, "contentConflicts": 0 },
    { "key": "Logic:Tool", "state": "enabled", "conflicts": 0, "contentConflicts": 0 },
    { "key": "Logic:Off", "state": "disabled", "conflicts": 0, "contentConflicts": 0 }
  ],
  "archives": ["Mods/Pack.zip", "install-smoke.zip"],
  "install": { "archive": "install-smoke.zip", "key": "Standard:SmokeInstall" },
  "unmanaged": [
    { "relPath": "Stray.pak", "kind": "paksFile" },
    { "relPath": "Mods/_Half_tmp_0123456789abcdef0123456789abcdef", "kind": "tempFolder" }
  ],
  "leftovers": ["Mods/_Old_tmp_ABCDEF0123456789ABCDEF0123456789"],
  "buildId": "25221600"
}
"""

# (path below <out> with "/" separators, content). Folders are created as needed.
SMOKE_FILES = (
    ("steamapps/appmanifest_1778820.acf", APPMANIFEST.encode("utf-8")),
    ("steamapps/common/TEKKEN 8/TEKKEN 8.exe", b""),
    ("steamapps/common/TEKKEN 8/Polaris/Binaries/Win64/Polaris-Win64-Shipping.exe", b""),
)

# (path below Paks with "/" separators, content).
SMOKE_PAKS_FILES = (
    (MARKER_NAME, MARKER.encode("utf-8")),
    ("pakchunk0-Windows.pak", b""),
    ("pakchunk0-Windows.ucas", b""),
    ("pakchunk0-Windows.utoc", BASE_TOC),
    ("global.ucas", b""),
    ("global.utoc", b""),
    ("Stray.pak", DATA),
    (SMOKE_ARCHIVE, make_install_smoke_zip()),
    ("Mods/Alpha/Alpha.pak", DATA),
    ("Mods/Alpha/Alpha.ucas", DATA),
    ("Mods/Alpha/Alpha.utoc", ALPHA_TOC),
    ("Mods/Alpha/modinfo.json", ALPHA_MODINFO),
    ("Mods/Beta/Beta.pak-x", DATA),
    ("Mods/Beta/Beta.ucas-x", DATA),
    ("Mods/Beta/Beta.utoc-x", BETA_TOC),
    ("Mods/Gamma/Content/Paks/Gamma.pak", DATA),
    ("Mods/Gamma/Content/Paks/Gamma_alt.pak-x", DATA),
    ("Mods/Delta/Shared.pak", DATA),
    ("Mods/Epsilon/Sub/shared.PAK", DATA),
    ("Mods/Empty/readme.txt", DATA),
    ("Mods/Loose.pak", DATA),
    ("Mods/Loose.ucas", DATA),
    ("Mods/Loose.utoc", LOOSE_TOC),
    ("Mods/Pack.zip", DATA),
    ("Mods/_Half_tmp_0123456789abcdef0123456789abcdef/Half.pak", DATA),
    ("Mods/_Old_tmp_ABCDEF0123456789ABCDEF0123456789/Old.pak-x", DATA),
    ("~mods/Legacy/Legacy.pak", DATA),
    ("LogicMods/Tool/Tool.pak", DATA),
    ("LogicMods/.disabled_Off/Off.pak-x", DATA),
)


class FixtureError(Exception):
    pass


def below(base, rel):
    """Joins a "/"-separated relative path to base; refuses anything that would leave base."""
    parts = rel.split("/")
    if any(part in ("", ".", "..") for part in parts):
        raise FixtureError(f"bad fixture path: {rel!r}")
    return os.path.join(base, *parts)


def write_new_file(path, content):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    # "xb": never replaces a file, and binary mode keeps LF line endings on Windows too.
    with open(path, "xb") as file:
        file.write(content)


def prepare_out_dir(out):
    if os.path.lexists(out):
        if not os.path.isdir(out) or os.path.islink(out):
            raise FixtureError(f"not a folder: {out}")
        if os.listdir(out):
            raise FixtureError(f"refusing to write into a folder that is not empty: {out}")
    else:
        os.makedirs(out)
    return os.path.abspath(out)


def make_smoke(out):
    json.loads(MARKER)
    base = prepare_out_dir(out)
    paks = os.path.join(base, *PAKS_PARTS)
    for rel, content in SMOKE_FILES:
        write_new_file(below(base, rel), content)
    for rel, content in SMOKE_PAKS_FILES:
        write_new_file(below(paks, rel), content)
    return paks


TMM_PROFILES = [
    {
        "Id": "00000000-0000-4000-8000-000000000001",
        "Name": "Default",
        "Description": "Default profile",
        "EnabledMods": None,
        "Created": "2026-01-01T10:00:00.0000000+00:00",
        "LastUsed": "2026-01-01T10:00:00.0000000+00:00",
        "IsDefault": True,
    },
    {
        "Id": "00000000-0000-4000-8000-000000000002",
        "Name": "Costumes",
        "Description": "Keys with a folder prefix",
        "EnabledMods": ["Standard:Alpha", "Standard:Beta", "LogicMods:Gamma"],
        "Created": "2026-02-01T18:30:00.0000000+01:00",
        "LastUsed": "2026-03-01T20:15:00.0000000+01:00",
        "IsDefault": False,
    },
    {
        "Id": "00000000-0000-4000-8000-000000000003",
        "Name": "Legacy keys",
        "Description": "",
        "EnabledMods": ["Alpha", "Loose.pak"],
        "Created": "2026-02-10T09:00:00.0000000+00:00",
        "LastUsed": "2026-02-10T09:00:00.0000000+00:00",
        "IsDefault": False,
    },
]


def make_profiles(out):
    base = prepare_out_dir(out)
    # JsonSerializer with WriteIndented: two-space indent, LF here so the files are the same on every OS.
    text = json.dumps(TMM_PROFILES, indent=2, ensure_ascii=False).encode("utf-8") + b"\n"
    write_new_file(below(base, "profiles.json"), text)
    write_new_file(below(base, "profiles-bom.json"), b"\xef\xbb\xbf" + text)
    return base


COMMANDS = {"smoke": make_smoke, "profiles": make_profiles}


def usage(stream):
    print(__doc__.strip(), file=stream)


def main(argv):
    if len(argv) == 3 and argv[1] == "--profiles":
        argv = [argv[0], "profiles", argv[2]]
    if len(argv) == 2 and argv[1] in ("-h", "--help"):
        usage(sys.stdout)
        return 0
    if len(argv) != 3 or argv[1] not in COMMANDS:
        usage(sys.stderr)
        return 2
    try:
        result = COMMANDS[argv[1]](argv[2])
    except (FixtureError, OSError) as error:
        print(f"make-fixtures: {error}", file=sys.stderr)
        return 1
    print(result)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
