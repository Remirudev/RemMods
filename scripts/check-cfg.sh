#!/usr/bin/env bash
# cfg placement guard (docs/PLAN.md §1, §3). OS-specific cfg predicates (windows, unix, target_os,
# target_family, target_env, target_vendor) may appear only in:
#   crates/remmods-core/src/platform/**   src-tauri/src/os/**   crates/*/tests/platform_*.rs   package build.rs
# (plus Cargo target-specific dependency tables, which are not .rs files).
# Usage: scripts/check-cfg.sh   (exit 0 = clean, 1 = violations listed)
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

readonly ALLOWED_HINT='crates/remmods-core/src/platform/**, src-tauri/src/os/**, crates/*/tests/platform_*.rs, package build.rs'

# Reports a line when the parentheses of a cfg(…), cfg!(…) or cfg_attr(…) name a platform predicate,
# also on the continuation lines of a predicate that spans several lines. Only the text inside those
# parentheses counts, so `windows_subsystem = "windows"` and quoted feature names pass. String
# literals are blanked first, so a `//` inside quotes does not hide the rest of the line, and then
# line comments are ignored.
read -r -d '' SCANNER <<'AWK' || true
function strip_strings(s) {
    gsub(/"([^"\\]|\\.)*"/, "\"\"", s)
    return s
}
function strip_comment(s,    i) {
    i = index(s, "//")
    return i ? substr(s, 1, i - 1) : s
}
function names_platform(s) {
    return s ~ /(^|[^"[:alnum:]_-])(windows|unix|target_(os|family|env|vendor))([^"[:alnum:]_-]|$)/
}
# Returns the text from `start` up to the parenthesis that closes depth `d`. Sets open_depth (0 when
# it closed on this line) and end_pos (index of the closing parenthesis, or past the end).
function take_predicate(s, start, d,    i, c, n) {
    n = length(s)
    for (i = start; i <= n; i++) {
        c = substr(s, i, 1)
        if (c == "(") {
            d++
        } else if (c == ")" && --d == 0) {
            open_depth = 0
            end_pos = i
            return substr(s, start, i - start)
        }
    }
    open_depth = d
    end_pos = n + 1
    return substr(s, start)
}
function report() {
    printf "  %s:%d: %s\n", FILENAME, FNR, $0
}
FNR == 1 { depth = 0 }
{
    line = strip_comment(strip_strings($0))
    hit = 0
    pos = 1
    if (depth > 0) {
        if (names_platform(take_predicate(line, 1, depth))) hit = 1
        depth = open_depth
        pos = end_pos + 1
    }
    while (depth == 0 && pos <= length(line)) {
        if (!match(substr(line, pos), /(^|[^[:alnum:]_])cfg(_attr)?!?[[:space:]]*\(/)) break
        if (names_platform(take_predicate(line, pos + RSTART + RLENGTH - 1, 1))) hit = 1
        depth = open_depth
        pos = end_pos + 1
    }
    if (hit) report()
}
AWK

# Anchored regexes: a `case` glob's `*` also matches `/`, which let any module named build.rs or any
# nested tests/platform_*.rs through. Build scripts count only at a package root.
readonly ALLOWED_PATTERNS=(
    '^crates/remmods-core/src/platform(\.rs|/.+)$'
    '^src-tauri/src/os(\.rs|/.+)$'
    '^crates/[^/]+/tests/platform_[^/]+\.rs$'
    '^(src-tauri/|crates/[^/]+/)?build\.rs$'
)

is_allowed() {
    local pattern
    for pattern in "${ALLOWED_PATTERNS[@]}"; do
        [[ $1 =~ $pattern ]] && return 0
    done
    return 1
}

list_rust_files() {
    if git rev-parse --is-inside-work-tree > /dev/null 2>&1; then
        # Tracked plus new, not-ignored files, so uncommitted work is checked too.
        git ls-files -z --cached --others --exclude-standard -- '*.rs'
    else
        find . \( -name target -o -name node_modules -o -name .git \) -prune -o -type f -name '*.rs' -print0 |
            sed -z 's#^\./##'
    fi
}

checked=()
skipped=0
while IFS= read -r -d '' file; do
    [[ -f $file ]] || continue
    if is_allowed "$file"; then
        skipped=$((skipped + 1))
    else
        checked+=("$file")
    fi
done < <(list_rust_files)

violations=""
if ((${#checked[@]} > 0)); then
    violations="$(printf '%s\0' "${checked[@]}" | xargs -0 -r awk "$SCANNER")"
fi

if [[ -n $violations ]]; then
    echo "check-cfg: OS-specific cfg outside the platform layer:" >&2
    printf '%s\n' "$violations" >&2
    echo "check-cfg: allowed only in $ALLOWED_HINT (docs/PLAN.md §3)." >&2
    exit 1
fi

echo "check-cfg: OK (${#checked[@]} .rs files checked, $skipped in allowed places)"
