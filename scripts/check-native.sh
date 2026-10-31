#!/usr/bin/env bash
# Native-code guard (docs/PLAN.md §1, §10):
#   1. the dependency trees of both release targets contain no crate from deny.toml's [bans] deny list;
#   2. the Windows tree contains no openssl-sys (Windows uses SChannel);
#   3. [Linux host] no build script asked rustc to link a static library
#      (checked in target/*/build/*/output when a build has run; skipped otherwise);
#   4. optional: --glibc <binary> fails if the binary needs a GLIBC_ symbol newer than 2.39
#      (ubuntu-24.04, the supported floor; override with GLIBC_MAX=x.y).
# Usage: scripts/check-native.sh [--glibc <binary>]
set -euo pipefail

usage() {
    sed -n '2,10s/^# \{0,1\}//p' "${BASH_SOURCE[0]}"
}

readonly TARGETS=(x86_64-unknown-linux-gnu x86_64-pc-windows-msvc)
readonly WINDOWS_TARGET=x86_64-pc-windows-msvc
readonly WINDOWS_ONLY_BANNED=(openssl-sys)

glibc_binary=""
while (($# > 0)); do
    case "$1" in
        --glibc)
            glibc_binary="${2:?--glibc needs a binary path}"
            shift 2
            ;;
        -h | --help)
            usage
            exit 0
            ;;
        *)
            echo "check-native: unknown argument: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ -n $glibc_binary && $glibc_binary != /* ]]; then
    glibc_binary="$PWD/$glibc_binary"
fi
cd "$repo_root"

failures=0
fail() {
    echo "check-native: FAIL: $*" >&2
    failures=$((failures + 1))
}

# The `{ crate = "…" }` (or plain "…") entries of the deny array inside [bans].
banned_from_deny_toml() {
    awk '
        /^[[:space:]]*\[/ { in_bans = ($0 ~ /^[[:space:]]*\[bans\][[:space:]]*(#.*)?$/); in_deny = 0; next }
        in_bans && /^[[:space:]]*deny[[:space:]]*=[[:space:]]*\[/ { in_deny = 1; next }
        in_deny && /^[[:space:]]*\]/ { in_deny = 0; next }
        in_deny {
            if (match($0, /crate[[:space:]]*=[[:space:]]*"[^"]+"/)) {
                name = substr($0, RSTART, RLENGTH)
                sub(/^crate[[:space:]]*=[[:space:]]*"/, "", name)
            } else if (match($0, /^[[:space:]]*"[^"]+"/)) {
                name = substr($0, RSTART, RLENGTH)
                sub(/^[[:space:]]*"/, "", name)
            } else {
                next
            }
            sub(/".*/, "", name)
            sub(/[@:].*/, "", name)
            print name
        }
    ' deny.toml
}

cargo_tree() {
    cargo tree --locked --workspace --all-features --edges normal,build,dev "$@"
}

crate_names() {
    cargo_tree --target "$1" --prefix none --format '{p}' | awk '{ print $1 }' | LC_ALL=C sort -u
}

check_dependency_trees() {
    local banned_list target names crate count
    banned_list="$(banned_from_deny_toml)"
    if [[ -z $banned_list ]]; then
        fail "could not read the [bans] deny list from deny.toml"
        return
    fi
    local -a banned
    mapfile -t banned <<< "$banned_list"

    for target in "${TARGETS[@]}"; do
        names="$(crate_names "$target")"
        local -a forbidden=("${banned[@]}")
        if [[ $target == "$WINDOWS_TARGET" ]]; then
            forbidden+=("${WINDOWS_ONLY_BANNED[@]}")
        fi
        count=0
        for crate in "${forbidden[@]}"; do
            if grep -qxF -- "$crate" <<< "$names"; then
                fail "$crate is in the $target dependency tree; it is pulled in by:"
                cargo_tree --target "$target" --invert "$crate" >&2 || true
                count=$((count + 1))
            fi
        done
        if ((count == 0)); then
            echo "check-native: $target: $(wc -l <<< "$names") crates, none of the ${#forbidden[@]} banned ones"
        fi
    done
}

check_static_links() {
    if [[ "$(uname -s)" != Linux ]]; then
        echo "check-native: static-link check runs on Linux hosts only; skipped"
        return
    fi
    local target_dir="${CARGO_TARGET_DIR:-target}"
    local -a outputs
    shopt -s nullglob
    outputs=("$target_dir"/*/build/*/output)
    shopt -u nullglob
    if ((${#outputs[@]} == 0)); then
        echo "check-native: no build-script outputs in $target_dir/*/build (nothing built yet); static-link check skipped"
        return
    fi
    local hits
    hits="$(printf '%s\0' "${outputs[@]}" |
        xargs -0 grep -H -E 'rustc-link-lib=static|rustc-flags=.*-l[[:space:]]*static=' || true)"
    if [[ -n $hits ]]; then
        fail "build scripts link static libraries (prefer pure-Rust crates, docs/PLAN.md §1):"
        printf '%s\n' "$hits" >&2
    else
        echo "check-native: ${#outputs[@]} build-script outputs, no static links"
    fi
}

check_glibc_floor() {
    local binary="$1" max="${GLIBC_MAX:-2.39}" highest newest
    if [[ ! -f $binary ]]; then
        fail "--glibc: no such file: $binary"
        return
    fi
    highest="$(objdump -T "$binary" | { grep -oE 'GLIBC_[0-9]+(\.[0-9]+)+' || true; } |
        sed 's/^GLIBC_//' | sort -uV | tail -n 1)"
    if [[ -z $highest ]]; then
        echo "check-native: $binary references no versioned glibc symbols"
        return
    fi
    newest="$(printf '%s\n%s\n' "$highest" "$max" | sort -V | tail -n 1)"
    if [[ $newest != "$max" ]]; then
        fail "$binary needs GLIBC_$highest, newer than the supported floor $max"
    else
        echo "check-native: $binary needs at most GLIBC_$highest (floor $max)"
    fi
}

check_dependency_trees
check_static_links
if [[ -n $glibc_binary ]]; then
    check_glibc_floor "$glibc_binary"
fi

if ((failures > 0)); then
    echo "check-native: $failures problem(s) found" >&2
    exit 1
fi
echo "check-native: OK"
