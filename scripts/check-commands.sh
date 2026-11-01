#!/usr/bin/env bash
# IPC command-list guard (docs/PLAN.md §5). Every app command is named in three places that must agree:
#   src-tauri/build.rs                  COMMANDS (tauri-build makes an allow-<command> permission each)
#   src-tauri/src/specta.rs             collect_commands![…] (invoke handler and TypeScript bindings)
#   src-tauri/capabilities/main.json    the app's "allow-<command>" permissions (plugin ones ignored)
# Usage: scripts/check-commands.sh   (exit 0 = the three sets are equal, 1 = differences listed)
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

readonly BUILD_RS=src-tauri/build.rs
readonly SPECTA_RS=src-tauri/src/specta.rs
readonly CAPABILITY=src-tauri/capabilities/main.json

usage() {
    sed -n '2,7s/^# \{0,1\}//p' "${BASH_SOURCE[0]}"
}

# Rust sources: CRs and line comments are dropped first, so a commented-out entry does not count.
strip_rust_comments() {
    sed -e 's/\r$//' -e 's#//.*$##' "$1"
}

# The string literals of `const COMMANDS: &[&str] = &[ … ];`.
commands_from_build_rs() {
    strip_rust_comments "$BUILD_RS" | awk '
        !in_list && /const[[:space:]]+COMMANDS[[:space:]]*:/ { in_list = 1; sub(/^.*=/, "") }
        in_list {
            line = $0
            while (match(line, /"[^"]*"/)) {
                print substr(line, RSTART + 1, RLENGTH - 2)
                line = substr(line, RSTART + RLENGTH)
            }
            if ($0 ~ /\][[:space:]]*;/) exit
        }
    '
}

# The paths inside `collect_commands![ … ]`, each reduced to its last segment (`shell::app_ready` →
# `app_ready`); a turbofish such as `::<tauri::Wry>` is dropped first.
commands_from_specta_rs() {
    strip_rust_comments "$SPECTA_RS" | awk '
        !in_list && /collect_commands![[:space:]]*\[/ { in_list = 1; sub(/^.*collect_commands![[:space:]]*\[/, "") }
        in_list {
            done = sub(/\].*$/, "")
            body = body " " $0
            if (done) exit
        }
        END {
            n = split(body, items, ",")
            for (i = 1; i <= n; i++) {
                item = items[i]
                gsub(/[[:space:]]+/, "", item)
                sub(/::<.*$/, "", item)
                sub(/^.*::/, "", item)
                if (item != "") print item
            }
        }
    '
}

# Permission strings that start with "allow-" (app commands); plugin permissions such as
# "core:window:allow-close" or "log:default" carry a "<plugin>:" prefix and are skipped.
commands_from_capability() {
    sed 's/\r$//' "$CAPABILITY" |
        grep -oE '"allow-[a-z0-9-]+"' |
        sed -e 's/^"allow-//' -e 's/"$//' -e 's/-/_/g' || true
}

sorted_set() {
    LC_ALL=C sort -u
}

# Prints "  <name> (only in <a>, missing from <b>)" lines for both directions.
report_difference() {
    local a_label="$1" a_set="$2" b_label="$3" b_set="$4"
    LC_ALL=C comm -23 <(printf '%s\n' "$a_set") <(printf '%s\n' "$b_set") |
        sed "s#^#  in $a_label, missing from $b_label: #"
    LC_ALL=C comm -13 <(printf '%s\n' "$a_set") <(printf '%s\n' "$b_set") |
        sed "s#^#  in $b_label, missing from $a_label: #"
}

main() {
    case "${1:-}" in
        -h | --help)
            usage
            exit 0
            ;;
        "") ;;
        *)
            usage >&2
            exit 2
            ;;
    esac

    local file
    for file in "$BUILD_RS" "$SPECTA_RS" "$CAPABILITY"; do
        if [[ ! -f $file ]]; then
            echo "check-commands: missing $file" >&2
            exit 1
        fi
    done

    local build specta capability
    build="$(commands_from_build_rs | sorted_set)"
    specta="$(commands_from_specta_rs | sorted_set)"
    capability="$(commands_from_capability | sorted_set)"

    local empty=0 label
    for label in build specta capability; do
        if [[ -z ${!label} ]]; then
            echo "check-commands: found no commands in the $label list; has its format changed?" >&2
            empty=1
        fi
    done
    if ((empty)); then
        exit 1
    fi

    local differences
    differences="$(
        report_difference "$BUILD_RS" "$build" "$SPECTA_RS" "$specta"
        report_difference "$BUILD_RS" "$build" "$CAPABILITY" "$capability"
        report_difference "$SPECTA_RS" "$specta" "$CAPABILITY" "$capability"
    )"
    if [[ -n $differences ]]; then
        echo "check-commands: the command lists differ:" >&2
        printf '%s\n' "$differences" | LC_ALL=C sort -u >&2
        exit 1
    fi

    echo "check-commands: OK ($(wc -l <<< "$build") commands in build.rs, specta.rs and main.json)"
}

main "$@"
