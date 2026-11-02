#!/usr/bin/env bash
# Pre-seeds the AppImage tools tauri-bundler 2.10.4 would otherwise download without a hash check
# (SPEC-P5 §2.4, §7.2, W11). Runs in CI on Linux before every `tauri build` that makes an AppImage.
#   AppRun-x86_64                         becomes the AppImage's own AppRun (every user runs it)
#   linuxdeploy-07333c6-x86_64.AppImage   the bundler's pinned linuxdeploy build
#   linuxdeploy-plugin-appimage.AppImage  a fixed release instead of the moving `continuous` tag
#   runtime-x86_64                        the type-2 runtime stub every AppImage user runs first;
#                                         appimagetool would fetch it from type2-runtime `continuous`
# Each file is downloaded into a private temp folder, checked against the sha256 below and only then
# moved into ${XDG_CACHE_HOME:-$HOME/.cache}/tauri/ (tools 0755, the runtime 0644: it is only read),
# where the bundler finds it and skips its own download. Any mismatch fails before a file is made
# executable. With GITHUB_ENV set, LDAI_RUNTIME_FILE is exported to the later steps of the job, so
# the AppImage plugin hands the runtime to appimagetool as --runtime-file.
# Usage: scripts/seed-appimage-tools.sh
set -euo pipefail

usage() {
    sed -n '2,14s/^# \{0,1\}//p' "${BASH_SOURCE[0]}"
}

if (($# > 0)); then
    case "$1" in
        -h | --help)
            usage
            exit 0
            ;;
        *)
            echo "seed-appimage-tools: takes no arguments" >&2
            exit 2
            ;;
    esac
fi

readonly RELEASES=https://github.com
# Recorded on 2026-11-02 from files that were downloaded and hashed but never run. AppRun and
# linuxdeploy match the `digest` GitHub reports for the release assets; GitHub reports none for
# the plugin's 1-alpha-20250213-1 asset. The runtime (type2-runtime tag 20251108) matches its
# reported digest.
readonly TOOLS=(
    "AppRun-x86_64|$RELEASES/tauri-apps/binary-releases/releases/download/apprun-old/AppRun-x86_64|f30140a43a0a59e46db21bdefdf749b9e9f2c6946e92afabbacf98b8ae73fb4f"
    "linuxdeploy-07333c6-x86_64.AppImage|$RELEASES/tauri-apps/binary-releases/releases/download/linuxdeploy-07333c6/linuxdeploy-x86_64.AppImage|36a2d7e274d12e1050d0e9ecfe11d339ed54720b2bec464c286d53f8b07f5c62"
    "linuxdeploy-plugin-appimage.AppImage|$RELEASES/linuxdeploy/linuxdeploy-plugin-appimage/releases/download/1-alpha-20250213-1/linuxdeploy-plugin-appimage-x86_64.AppImage|992d502a248e14ab185448ddf6f6e7d25558cb84d4623c354c3af350c25fccb3"
    "runtime-x86_64|$RELEASES/AppImage/type2-runtime/releases/download/20251108/runtime-x86_64|2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d"
)
readonly RUNTIME=runtime-x86_64

cache_dir="${XDG_CACHE_HOME:-$HOME/.cache}/tauri"
staging="$(mktemp -d)"
trap 'rm -rf -- "$staging"' EXIT

for entry in "${TOOLS[@]}"; do
    IFS='|' read -r name url sha256 <<< "$entry"
    echo "seed-appimage-tools: downloading $name"
    curl -fsSL --retry 3 --proto '=https' --tlsv1.2 -o "$staging/$name" "$url"
done

(
    cd "$staging"
    for entry in "${TOOLS[@]}"; do
        IFS='|' read -r name _ sha256 <<< "$entry"
        printf '%s  %s\n' "$sha256" "$name"
    done | sha256sum -c --strict -
)

mkdir -p "$cache_dir"
for entry in "${TOOLS[@]}"; do
    IFS='|' read -r name _ _ <<< "$entry"
    if [[ $name == "$RUNTIME" ]]; then
        chmod 0644 "$staging/$name"
    else
        chmod 0755 "$staging/$name"
    fi
    mv -f -- "$staging/$name" "$cache_dir/$name"
done
if [[ -n ${GITHUB_ENV:-} ]]; then
    printf 'LDAI_RUNTIME_FILE=%s\n' "$cache_dir/$RUNTIME" >> "$GITHUB_ENV"
fi
echo "seed-appimage-tools: seeded ${#TOOLS[@]} verified files into $cache_dir"
