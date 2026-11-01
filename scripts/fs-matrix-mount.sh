#!/usr/bin/env bash
# Mounts the file systems of the `fs-matrix` CI job (docs/PLAN.md §9, SPEC-P2 §9.2) and prints the
# environment that crates/remmods-core/tests/fs_matrix.rs reads, one NAME=value per line, so the job
# can append stdout to $GITHUB_ENV. Everything else (notices, progress) goes to stderr.
# CI only: needs root, loop devices and mkfs tools, and changes one sysctl. Never run it on a
# machine you care about; it mounts only below <dir> and touches nothing else.
# Usage: sudo bash scripts/fs-matrix-mount.sh <dir> >> "$GITHUB_ENV"
set -euo pipefail

dir="${1:?usage: fs-matrix-mount.sh <dir>}"
case "$dir" in
  /?*) ;;
  *) echo "fs-matrix-mount: <dir> must be an absolute path other than /" >&2; exit 2 ;;
esac
[[ "$(id -u)" -eq 0 ]] || { echo "fs-matrix-mount: run it with sudo" >&2; exit 2; }
owner_uid="${SUDO_UID:?run it through sudo so SUDO_UID is set}"
owner_gid="${SUDO_GID:-$owner_uid}"

mkdir -p "$dir"
log() { echo "fs-matrix-mount: $*" >&2; }
notice() { echo "::notice::fs-matrix: $*" >&2; }

# make_image <label> <size MiB> <mkfs command…>: sparse image file, formatted, not yet mounted.
make_image() {
  local label="$1" size="$2"
  shift 2
  truncate -s "${size}M" "$dir/$label.img"
  "$@" "$dir/$label.img" >&2
}

# mount_label <label> <mount options>: loop-mounts the image and hands the root to the CI user.
mount_label() {
  local label="$1" options="$2"
  mkdir -p "$dir/$label"
  mount -o "loop${options:+,$options}" "$dir/$label.img" "$dir/$label"
}

own() { chown "$owner_uid:$owner_gid" "$@"; }

# add_mount <label> <size MiB> <required|optional> <mount options> <mkfs command…>
mounts=""
add_mount() {
  local label="$1" size="$2" need="$3" options="$4"
  shift 4
  if make_image "$label" "$size" "$@" && mount_label "$label" "$options"; then
    [[ "$options" == *uid=* ]] || own "$dir/$label"
    mounts="${mounts}${label}=${dir}/${label};"
    log "mounted $label"
  elif [[ "$need" == required ]]; then
    echo "::error::fs-matrix: cannot set up the required file system $label" >&2
    exit 1
  else
    notice "$label skipped: mkfs or mount failed (module missing on this runner?)"
    umount "$dir/$label" 2>/dev/null || true
  fi
}

mkdir -p "$dir/tmpfs"
mount -t tmpfs -o size=64m tmpfs "$dir/tmpfs"
own "$dir/tmpfs"
mounts="tmpfs=${dir}/tmpfs;"

add_mount ext4 64 required "" mkfs.ext4 -q -F
# btrfs refuses images below about 110 MiB; the file is sparse, so size costs nothing.
add_mount btrfs 128 required "" mkfs.btrfs -q -f
modprobe vfat 2>/dev/null || true
add_mount vfat 64 optional "uid=$owner_uid,gid=$owner_gid" mkfs.vfat
modprobe exfat 2>/dev/null || true
add_mount exfat 64 optional "uid=$owner_uid,gid=$owner_gid" mkfs.exfat

echo "REMMODS_FS_MATRIX=${mounts%;}"

# A read-only ext4: the mod exists before the remount.
make_image ro 8 mkfs.ext4 -q -F
mount_label ro ""
mkdir -p "$dir/ro/Mods/Ro" "$dir/ro/~mods" "$dir/ro/LogicMods"
echo pak > "$dir/ro/Mods/Ro/Ro.pak"
own -R "$dir/ro"
mount -o remount,ro "$dir/ro"
echo "REMMODS_FS_MATRIX_RO=$dir/ro"

# A tmpfs whose folders and mod stay root-owned (PLAN §9: root-owned mount root), mode 0755.
mkdir -p "$dir/denied"
mount -t tmpfs -o size=1m,mode=0755 tmpfs "$dir/denied"
mkdir -p "$dir/denied/Mods/R" "$dir/denied/~mods" "$dir/denied/LogicMods"
echo pak > "$dir/denied/Mods/R/R.pak"
chmod 0755 "$dir/denied" "$dir/denied/Mods" "$dir/denied/Mods/R" "$dir/denied/~mods" "$dir/denied/LogicMods"
chmod 0644 "$dir/denied/Mods/R/R.pak"
echo "REMMODS_FS_MATRIX_DENIED=$dir/denied"

# A full data dir: the folders exist first, then the rest of the file system is filled up. -m 0 drops
# ext4's reserved blocks, so the CI user hits ENOSPC too, not only root.
make_image full 4 mkfs.ext4 -q -F -m 0 -O ^has_journal
mount_label full ""
mkdir -p "$dir/full/data/sessions"
: > "$dir/full/data/remmods.lock"
own -R "$dir/full/data"
own "$dir/full"
dd if=/dev/zero of="$dir/full/filler" bs=64k 2>/dev/null || true
sync
echo "REMMODS_FS_MATRIX_FULL=$dir/full"

# The inotify queue overflow test needs a tiny queue; without the sysctl it stays off.
if sysctl -q -w fs.inotify.max_queued_events=16 >&2; then
  echo "REMMODS_FS_MATRIX_SMALL_QUEUE=1"
else
  notice "inotify queue not shrunk: the overflow test is skipped"
fi
