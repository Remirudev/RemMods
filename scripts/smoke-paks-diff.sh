#!/usr/bin/env bash
# Compares two paks-snapshot listings of the smoke fixture, taken before and after
# `remmods --smoke --paks` (CI only). The install round trip (check installFixture) adds exactly
# Mods/SmokeInstall/ from install-smoke.zip; every other entry must end as it began. Folder mtimes
# are ignored: the toggle round trip renames inside Mods and LogicMods, which bumps them.
# Usage: scripts/smoke-paks-diff.sh <before.tsv> <after.tsv>
set -euo pipefail

usage() {
    sed -n '2,6s/^# \{0,1\}//p' "${BASH_SOURCE[0]}"
}

if (($# != 2)); then
    usage >&2
    exit 2
fi
readonly BEFORE="$1"
readonly AFTER="$2"
readonly INSTALLED=$'Mods/SmokeInstall/\nMods/SmokeInstall/readme.txt\nMods/SmokeInstall/smoke.pak'

installed_entries() {
    awk -F'\t' 'index($1, "Mods/SmokeInstall/") == 1 { print $1 }' "$1"
}

without_install() {
    awk -F'\t' -v OFS='\t' 'index($1, "Mods/SmokeInstall/") != 1 { if ($4 == "dir") $3 = "-"; print }' "$1"
}

if [[ -n "$(installed_entries "$BEFORE")" ]]; then
    echo "smoke-paks-diff: $BEFORE already lists Mods/SmokeInstall/" >&2
    exit 1
fi
installed="$(installed_entries "$AFTER")"
if [[ $installed != "$INSTALLED" ]]; then
    echo "smoke-paks-diff: the install round trip left these entries instead of SmokeInstall's three:" >&2
    printf '%s\n' "${installed:-<none>}" >&2
    exit 1
fi
diff -u <(without_install "$BEFORE") <(without_install "$AFTER")
