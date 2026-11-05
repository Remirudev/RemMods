#!/usr/bin/env bash
# Installs RemMods for this user into ~/.local (no root, no package manager) with its desktop
# entry and icons. Re-run it to update: it rebuilds whenever a source is newer than the release
# binary. CachyOS / Arch friendly: it builds with --no-bundle, so no AppImage, deb or rpm tooling
# runs. Settings, profiles, logs and the game's mod folders are never touched.
#
#   ./cachyos-install.sh               build when stale, then install
#   ./cachyos-install.sh --build       force a fresh release build first
#   ./cachyos-install.sh --uninstall   remove what this script installed
#   PREFIX=/some/prefix ./cachyos-install.sh   install below another prefix (default ~/.local)
set -euo pipefail

usage() { sed -n '2,10s/^# \{0,1\}//p' "${BASH_SOURCE[0]}"; }

readonly APP=remmods
readonly HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly PREFIX="${PREFIX:-$HOME/.local}"
readonly BINDIR="$PREFIX/bin"
readonly MARKERDIR="$PREFIX/lib/$APP"
readonly APPDIR="$PREFIX/share/applications"
readonly ICONROOT="$PREFIX/share/icons/hicolor"
readonly RELBIN="${CARGO_TARGET_DIR:-$HERE/target}/release/$APP"
# Source file in src-tauri/icons → hicolor size folder.
readonly ICONS=("32x32.png:32x32" "128x128.png:128x128" "128x128@2x.png:256x256" "icon.png:512x512")

FORCE_BUILD=0
UNINSTALL=0
for arg in "$@"; do
  case "$arg" in
    --build) FORCE_BUILD=1 ;;
    --uninstall) UNINSTALL=1 ;;
    -h | --help) usage; exit 0 ;;
    *) echo "unknown option: $arg" >&2; usage >&2; exit 2 ;;
  esac
done

if (( EUID == 0 )); then
  echo "error: run this as your own user, not with sudo; it installs into $PREFIX" >&2
  exit 1
fi

refresh_caches() {
  update-desktop-database "$APPDIR" 2>/dev/null || true
  gtk-update-icon-cache -q -t "$ICONROOT" 2>/dev/null || true
  # Plasma reads menu entries and icons through its own cache.
  kbuildsycoca6 >/dev/null 2>&1 || true
}

uninstall() {
  local removed=0 path icon
  local paths=("$BINDIR/$APP" "$MARKERDIR/install-source" "$APPDIR/$APP.desktop")
  for icon in "${ICONS[@]}"; do
    paths+=("$ICONROOT/${icon#*:}/apps/$APP.png")
  done
  for path in "${paths[@]}"; do
    if [[ -e "$path" || -L "$path" ]]; then
      rm -f -- "$path"
      echo "removed  $path"
      removed=1
    fi
  done
  rmdir "$MARKERDIR" 2>/dev/null || true
  (( removed )) && refresh_caches
  echo "RemMods is uninstalled from $PREFIX. Settings, profiles, logs and mods stay where they are."
}

if (( UNINSTALL )); then
  uninstall
  exit 0
fi

needs_build() {
  (( FORCE_BUILD )) && return 0
  [[ -x "$RELBIN" ]] || return 0
  [[ -n "$(find "$HERE/src" "$HERE/src-tauri/src" "$HERE/src-tauri/icons" "$HERE/crates" \
    "$HERE/index.html" "$HERE/package.json" "$HERE/package-lock.json" "$HERE/vite.config.ts" \
    "$HERE/Cargo.toml" "$HERE/Cargo.lock" "$HERE/src-tauri/Cargo.toml" \
    "$HERE/src-tauri/tauri.conf.json" "$HERE/src-tauri/tauri.linux.conf.json" \
    -newer "$RELBIN" -print -quit 2>/dev/null)" ]]
}

if needs_build; then
  if [[ ! -d "$HERE/node_modules" ]]; then
    echo "==> Installing npm dependencies…"
    (cd "$HERE" && npm ci)
  fi
  echo "==> Building the release binary (no bundle)…"
  (cd "$HERE" && npm run tauri -- build --no-bundle -- --locked)
fi
[[ -x "$RELBIN" ]] || { echo "error: no release binary at $RELBIN (try: $0 --build)" >&2; exit 1; }

# The desktop template is tauri-bundler's (Handlebars); fill it the way the bundler does.
desktop_entry() {
  local exec="$BINDIR/$APP"
  [[ "$exec" == *" "* ]] && exec="\"$exec\""
  sed -e '/^{{#if comment}}$/d' -e '/^{{\/if}}$/d' \
    -e 's|{{name}}|RemMods|g' -e 's|{{comment}}|Mod manager for TEKKEN 8|g' \
    -e "s|{{exec}}|$exec|g" -e "s|{{icon}}|$APP|g" -e 's|{{categories}}|Utility;|g' \
    "$HERE/src-tauri/linux/$APP.desktop"
}

mkdir -p "$BINDIR" "$MARKERDIR" "$APPDIR"

echo "==> Binary        → $BINDIR/$APP"
# Install beside it and rename over it, so a running RemMods keeps its old file.
install -m755 "$RELBIN" "$BINDIR/.$APP.new"
mv -f "$BINDIR/.$APP.new" "$BINDIR/$APP"

echo "==> Install marker → $MARKERDIR/install-source"
printf 'install-source=local\n' > "$MARKERDIR/install-source"

for icon in "${ICONS[@]}"; do
  size="${icon#*:}"
  echo "==> Icon $size → $ICONROOT/$size/apps/$APP.png"
  install -Dm644 "$HERE/src-tauri/icons/${icon%%:*}" "$ICONROOT/$size/apps/$APP.png"
done

echo "==> Desktop entry → $APPDIR/$APP.desktop"
desktop_entry > "$APPDIR/.$APP.desktop.new"
mv -f "$APPDIR/.$APP.desktop.new" "$APPDIR/$APP.desktop"
if command -v desktop-file-validate >/dev/null 2>&1; then
  desktop-file-validate "$APPDIR/$APP.desktop" || echo "!!  desktop-file-validate reported the issues above"
fi

refresh_caches

echo
echo "Done. Start RemMods from the application menu, or run: $BINDIR/$APP"
case ":$PATH:" in
  *":$BINDIR:"*) ;;
  *) echo "Note: $BINDIR is not on your PATH, so a plain \"$APP\" will not be found in a terminal." ;;
esac
echo "  update    : run $0 again"
echo "  uninstall : $0 --uninstall"
