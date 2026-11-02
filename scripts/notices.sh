#!/usr/bin/env bash
# Regenerates (or checks) the committed third-party notices (docs/PLAN.md §10, SPEC-P5 §7.1).
#   rust  THIRD-PARTY-NOTICES      cargo-about 0.9.3 over the workspace (about.toml, about.hbs)
#   npm   THIRD-PARTY-NOTICES-NPM  scripts/npm-notices.py over package-lock.json + node_modules
# --check generates into a temp folder and compares with the committed files; on a difference it
# prints the command that regenerates them. The rust part runs `cargo fetch --locked` first and
# then cargo-about with --frozen (no network, no ClearlyDefined lookups); CRLF becomes LF.
# Usage: scripts/notices.sh [--check] [rust|npm|all]   (default: all)
set -euo pipefail

usage() {
    sed -n '2,8s/^# \{0,1\}//p' "${BASH_SOURCE[0]}"
}

readonly CARGO_ABOUT_VERSION="cargo-about 0.9.3"

check=0
part=all
while (($# > 0)); do
    case "$1" in
        --check) check=1 ;;
        rust | npm | all) part="$1" ;;
        -h | --help)
            usage
            exit 0
            ;;
        *)
            echo "notices: unknown argument: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
    shift
done

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
if command -v python3 > /dev/null 2>&1; then python=python3; else python=python; fi

out_dir="$repo_root"
if ((check)); then
    out_dir="$(mktemp -d)"
    trap 'rm -rf -- "$out_dir"' EXIT
fi

failures=0

generate_rust() {
    local found
    found="$(cargo about --version 2> /dev/null || true)"
    if [[ $found != "$CARGO_ABOUT_VERSION" ]]; then
        echo "notices: needs $CARGO_ABOUT_VERSION, found '${found:-none}'" >&2
        echo "  install: cargo install --locked --features cli cargo-about@0.9.3" >&2
        exit 1
    fi
    cargo fetch --locked
    cargo about generate --workspace --frozen --fail -c about.toml -o "$out_dir/THIRD-PARTY-NOTICES.crlf" about.hbs
    # Some crates ship CRLF licence texts; git stores LF (.gitattributes), so the check compares LF.
    sed 's/\r$//' "$out_dir/THIRD-PARTY-NOTICES.crlf" > "$out_dir/THIRD-PARTY-NOTICES"
    rm -f -- "$out_dir/THIRD-PARTY-NOTICES.crlf"
}

generate_npm() {
    "$python" -I scripts/npm-notices.py --out "$out_dir/THIRD-PARTY-NOTICES-NPM"
}

compare() {
    local name="$1" regenerate="$2"
    if cmp -s "$out_dir/$name" "$repo_root/$name"; then
        echo "notices: $name is current"
    else
        echo "notices: $name is stale; regenerate with: $regenerate" >&2
        failures=$((failures + 1))
    fi
}

if [[ $part == rust || $part == all ]]; then
    generate_rust
    if ((check)); then compare THIRD-PARTY-NOTICES "bash scripts/notices.sh rust"; fi
fi
if [[ $part == npm || $part == all ]]; then
    generate_npm
    if ((check)); then compare THIRD-PARTY-NOTICES-NPM "bash scripts/notices.sh npm"; fi
fi

if ((failures > 0)); then
    exit 1
fi
if ((!check)); then
    echo "notices: regenerated ($part); commit the changed files"
fi
