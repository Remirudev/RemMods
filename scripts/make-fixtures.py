#!/usr/bin/env python3
"""Builds test fixtures for RemMods (docs/PLAN.md §9). Standard library only.

Usage: python3 -I scripts/make-fixtures.py smoke <out>

smoke  A fake Steam library below <out> with TEKKEN 8 installed (appmanifest, both game exes) and a
       Paks folder holding every scan case of the P1 smoke test, plus the marker file
       `remmods-smoke-fixture` with the expected rows. Prints the Paks path; run the app with
       `remmods --smoke --paks <that path>`.

Refuses when <out> exists and is not empty. Writes only below <out> and never replaces a file.
"""

import json
import os
import sys

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

ALPHA_MODINFO = (
    b'{"Version":"2.0","Category":"Character","Description":"Smoke fixture","Author":"RemMods"}'
)

# The scanner's expectations for the Paks tree below, read by `remmods --smoke --paks` (check
# scanFixture) and mirrored by a Rust test in remmods-core, so script and scanner cannot drift.
MARKER = """{
  "version": 1,
  "fixture": "remmods-smoke",
  "rows": [
    { "key": "Standard:Alpha", "state": "enabled", "conflicts": 0 },
    { "key": "Standard:Beta", "state": "disabled", "conflicts": 0 },
    { "key": "Standard:Gamma", "state": "partial", "conflicts": 0 },
    { "key": "Standard:Delta", "state": "enabled", "conflicts": 1 },
    { "key": "Standard:Epsilon", "state": "enabled", "conflicts": 1 },
    { "key": "Standard:Empty", "state": "noPakFiles", "conflicts": 0 },
    { "key": "Loose:Mods/Loose", "state": "enabled", "conflicts": 0 },
    { "key": "Legacy:Legacy", "state": "enabled", "conflicts": 0 },
    { "key": "Logic:Tool", "state": "enabled", "conflicts": 0 },
    { "key": "Logic:Off", "state": "disabled", "conflicts": 0 }
  ],
  "archives": ["Mods/Pack.zip"],
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
    ("pakchunk0-Windows.utoc", b""),
    ("global.ucas", b""),
    ("global.utoc", b""),
    ("Stray.pak", DATA),
    ("Mods/Alpha/Alpha.pak", DATA),
    ("Mods/Alpha/Alpha.ucas", DATA),
    ("Mods/Alpha/Alpha.utoc", DATA),
    ("Mods/Alpha/modinfo.json", ALPHA_MODINFO),
    ("Mods/Beta/Beta.pak-x", DATA),
    ("Mods/Beta/Beta.ucas-x", DATA),
    ("Mods/Beta/Beta.utoc-x", DATA),
    ("Mods/Gamma/Content/Paks/Gamma.pak", DATA),
    ("Mods/Gamma/Content/Paks/Gamma_alt.pak-x", DATA),
    ("Mods/Delta/Shared.pak", DATA),
    ("Mods/Epsilon/Sub/shared.PAK", DATA),
    ("Mods/Empty/readme.txt", DATA),
    ("Mods/Loose.pak", DATA),
    ("Mods/Loose.ucas", DATA),
    ("Mods/Loose.utoc", DATA),
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


COMMANDS = {"smoke": make_smoke}


def usage(stream):
    print(__doc__.strip(), file=stream)


def main(argv):
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
