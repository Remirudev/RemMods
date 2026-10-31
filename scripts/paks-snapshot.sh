#!/usr/bin/env bash
# Read-only snapshot of TEKKEN 8's Paks folder for the in-game smoke test (docs/PLAN.md §9).
# Usage: scripts/paks-snapshot.sh [paks_dir | game_dir] > local/paks-<label>.tsv
#        diff -u local/paks-before.tsv local/paks-after.tsv
# Default: ~/.local/share/Steam/steamapps/common/TEKKEN 8/Polaris/Content/Paks
# Output: '#' header lines, then one tab-separated line per entry, sorted bytewise by path:
#   <path relative to Paks>  <size in bytes>  <mtime, epoch seconds>  <sha256>
# Folders end in "/" with size "-" and "dir" instead of a hash; files over 64 MiB show "big"
# (not hashed); symlinks show "link:<target>" and are not followed. Nothing is written.
set -euo pipefail

readonly HASH_LIMIT_BYTES=$((64 * 1024 * 1024))
readonly PAKS_SUBDIR="Polaris/Content/Paks"
readonly DEFAULT_PAKS_DIR="$HOME/.local/share/Steam/steamapps/common/TEKKEN 8/$PAKS_SUBDIR"

usage() {
    sed -n '2,9s/^# \{0,1\}//p' "${BASH_SOURCE[0]}"
}

resolve_paks_dir() {
    local dir="$1"
    if [[ -d "$dir/$PAKS_SUBDIR" ]]; then
        dir="$dir/$PAKS_SUBDIR"
    fi
    if [[ ! -d $dir ]]; then
        echo "paks-snapshot: not a folder: $dir" >&2
        echo "paks-snapshot: pass the Paks folder or the TEKKEN 8 game folder" >&2
        exit 1
    fi
    (cd -- "$dir" && pwd -P)
}

# Reads find's records (type, size, mtime, link target, path; \037-separated, NUL-terminated) and
# prints one row per entry. Hashing reads each small file once; nothing is opened for writing.
# Tabs and newlines inside names are printed as \t and \n so every entry stays on one line.
format_rows() {
    local type size mtime link path shown row hash
    local files=0 hashed=0 dirs=0 links=0
    while IFS=$'\037' read -r -d '' type size mtime link path; do
        mtime="${mtime%%.*}"
        shown="${path//$'\t'/\\t}"
        shown="${shown//$'\n'/\\n}"
        case "$type" in
            f)
                files=$((files + 1))
                if ((size <= HASH_LIMIT_BYTES)); then
                    hash="$(sha256sum < "$path")"
                    hash="${hash%% *}"
                    hashed=$((hashed + 1))
                else
                    hash=big
                fi
                row="$shown"$'\t'"$size"$'\t'"$mtime"$'\t'"$hash"
                ;;
            d)
                dirs=$((dirs + 1))
                row="$shown/"$'\t-\t'"$mtime"$'\tdir'
                ;;
            l)
                links=$((links + 1))
                link="${link//$'\t'/\\t}"
                row="$shown"$'\t-\t'"$mtime"$'\t'"link:${link//$'\n'/\\n}"
                ;;
            *)
                row="$shown"$'\t-\t'"$mtime"$'\t'"type:$type"
                ;;
        esac
        printf '%s\n' "$row"
    done
    echo "paks-snapshot: $files files ($hashed hashed, $((files - hashed)) big), $dirs folders, $links symlinks" >&2
}

main() {
    case "${1:-}" in
        -h | --help)
            usage
            exit 0
            ;;
    esac
    if (($# > 1)); then
        usage >&2
        exit 2
    fi

    local root
    root="$(resolve_paks_dir "${1:-$DEFAULT_PAKS_DIR}")"
    cd -- "$root"

    printf '# paks-snapshot v1\n'
    printf '# root: %s\n' "$root"
    printf '# columns: path, size (bytes), mtime (epoch s), sha256 | big (> 64 MiB) | dir | link:<target>\n'
    find . -mindepth 1 -printf '%y\037%s\037%T@\037%l\037%P\0' | format_rows | LC_ALL=C sort
}

main "$@"
