#!/usr/bin/env bash
# Cross-checks the Rust .utoc reader against the independent Python lister (read-only).
# Usage: scripts/utoc-compare.sh <dir>   (exit 0 = identical listings, 1 = diff, 2 = bad arguments)
# Both listers run with --hashes and print "<rel>\t<display path>\t<content hash or ->" or
# "<rel>\t!<status>" per line (SPEC-P2B §4.7).
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ $# -ne 1 || ! -d "$1" ]]; then
    echo "usage: scripts/utoc-compare.sh <dir>" >&2
    exit 2
fi
dir="$1"

if command -v python3 > /dev/null 2>&1; then PY=python3; else PY=python; fi

work="$(mktemp -d "${TMPDIR:-/tmp}/utoc-compare.XXXXXX")"
trap 'rm -rf "$work"' EXIT

cargo run -q --locked -p remmods-pak --example utoc-list -- --hashes "$dir" > "$work/rust.tsv"
"$PY" -I scripts/utoc-list.py --hashes "$dir" > "$work/python.tsv"

echo "rust: $(wc -l < "$work/rust.tsv") lines, python: $(wc -l < "$work/python.tsv") lines"
diff -u --label rust --label python "$work/rust.tsv" "$work/python.tsv"
echo "identical"
