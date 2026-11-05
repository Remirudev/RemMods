# RemMods

A mod manager for TEKKEN 8 on Linux and Windows. It turns pak mods (`Mods`, `~mods`) and LogicMods on
and off, installs them from archives, keeps profiles of which mods are active, and repairs what other
tools leave behind. Built with Tauri 2, Rust and React. It follows the usual TEKKEN 8 mod-folder
conventions (`Mods`, `~mods`, `LogicMods`, `.disabled_` folders, `-x` files, `modinfo.json`), so
existing mod folders keep working.

RemMods only ever writes inside the three mod folders, its own app folders and the Trash. Steam's
files are read-only to it. Removing a mod moves it to the Trash; deleting it permanently needs a second
confirmation.

## Status

**Phase 1 (read-only)**: RemMods finds the game (Steam detection or a picked folder), scans `Mods`,
`~mods` and `LogicMods`, shows every mod with its state and conflicts, shows whether the game is
running, and launches it through Steam.

**Phase 2 (in progress)**: enable and disable mods (P2). A disabled mod is renamed in place, never
overwritten, and the change is journaled so an interrupted toggle is rolled back on the next start.
Installing comes in a later phase. The full plan, including the phases and their
exit criteria (§13), is in [docs/PLAN.md](docs/PLAN.md).

**Phase 2b (in progress)**: shows what each mod contains (from its `.utoc` index) and flags mods that
ship the same assets. It only reads mod files.

**Phase 3 (in progress)**: manage mods. Rename a mod (the folder, its files and the profiles that name
it follow), move it to the Trash (freedesktop.org Trash, restorable from the file manager), delete it
permanently after a second confirmation, edit its metadata (`modinfo.json`), and fix leftovers of
interrupted operations in the Repair tab. Groups remember which mods are on: pick their mods or save
the current state, switch them on (additive or exclusive, with an automatic snapshot first) or off,
import the `profiles.json` of another mod manager (only read, never written), and export or import
RemMods groups. On Windows the Trash is the Recycle Bin
(CI-tested only, see [Windows](#windows)).

**Phase 3b (in progress)**: select several mods and choose "Move to Trash" (Windows: "Move to Recycle
Bin") or "Delete permanently…" for all of them at once; when not every mod went through, a list shows
the result for each one. Trash is the freedesktop.org Trash on Linux and the Recycle Bin on Windows
(CI-tested only). On Windows, when another program (an Explorer
window, an antivirus scan, the search indexer) holds a mod folder, RemMods tries again for up to 30 s
and shows a toast with Cancel. Cancelling stops there: the mod in progress is left as it was unless its
move had already finished, and in a bulk action mods already moved or deleted stay that way and the
rest are skipped. Saved sets of mods are groups, listed in a sidebar beside the mod list: one switch
turns a group on or off, its name shows only its mods, and every switch can be undone from its toast.

**Phase 4 (in progress)**: install mods. Drop zip, 7z or rar archives onto the window or pick them (or
a folder) with the file picker. The preview shows the detected root, the mod name, the filename
encoding, variants to choose from, automatic fixes and anything that blocks the install. Files are
unpacked into a hidden staging folder first and moved into place in one step, so a cancelled or failed
install leaves nothing behind; a mod that is already installed can be replaced (the old one goes to
the Trash). Limits: no password-protected archives and no multi-volume archives (install those from an
extracted folder).

**Phase 5 (in progress)**: release packaging. CI builds the AppImage, deb, rpm and Windows installer,
installs each one and runs its self-test; the AUR package `remmods-bin` is built from the deb. The
AppImage and the Windows installer can update themselves from Settings → Updates once a later release
exists (signed updates, checked only when you click). On Linux, Settings → Graphics and a "Start in safe graphics mode" launcher
action help when the window stays blank. **No release has been published yet**: no release-blocking
Windows gap is open in [docs/windows-gaps.md](docs/windows-gaps.md) (WG-2 and WG-5 were closed with
Phase 3b, CI-tested only); v1.0.0 waits for the final test round.

## Platforms

| Platform | Support |
|----------|---------|
| Linux (first) | glibc 2.39 or newer: Ubuntu 24.04+, Linux Mint 22+, Debian 13+, Fedora 40+, SteamOS 3.7+, Arch and CachyOS |
| Windows | Windows 10 22H2 and Windows 11, x64, with WebView2 Evergreen: **beta, CI-tested only** until it has been tested on a real PC |

Not planned for v1: macOS, Flatpak and Snap packages, Steam Deck Game Mode.

## Install

Releases are published on the [releases page](https://github.com/Remirudev/RemMods/releases) (the
first one will be v1.0.0). Pick one package:

| Package | File or name | For | Updates |
|---------|--------------|-----|---------|
| AppImage | `RemMods_<version>_amd64.AppImage` | any x86_64 Linux with glibc 2.39 or newer, Steam Deck (Desktop Mode) | in the app |
| Debian package | `RemMods_<version>_amd64.deb` (package `rem-mods`) | Ubuntu 24.04+, Linux Mint 22+, Debian 13+ | apt, by hand |
| RPM package | `RemMods-<version>-1.x86_64.rpm` (package `rem-mods`) | Fedora 40+ | dnf, by hand |
| AUR | `remmods-bin` | Arch Linux, CachyOS and other Arch-based distributions | your AUR helper |
| Windows installer | `RemMods_<version>_x64-setup.exe` | Windows 10 22H2 and 11, x64 (beta, see [Windows](#windows)) | in the app |

**Linux needs glibc 2.39 or newer**: Ubuntu 24.04+, Linux Mint 22+, Debian 13+, Fedora 40+, SteamOS
3.7+, Arch and CachyOS. Ubuntu 22.04, Debian 12 and Linux Mint 21 are too old.

The commands below work in bash, zsh and fish. Run them in the folder you downloaded to.

- **AppImage**: make it executable and start it. Keep it in a folder you own, such as
  `~/Applications`: it updates itself in place. If it does not start because FUSE is missing, add
  `--appimage-extract-and-run`.

  ```sh
  mkdir -p ~/Applications
  mv RemMods_*_amd64.AppImage ~/Applications/
  chmod +x ~/Applications/RemMods_*_amd64.AppImage
  ~/Applications/RemMods_*_amd64.AppImage
  ```

  For a menu entry and the taskbar icon, let an AppImage integration tool such as Gear Lever add it.
- **deb**: `sudo apt install ./RemMods_*_amd64.deb`. Remove it with `sudo apt remove rem-mods`.
- **rpm**: `sudo dnf install ./RemMods-*-1.x86_64.rpm`. Remove it with `sudo dnf remove rem-mods`.
- **AUR**: `paru -S remmods-bin` (or `yay -S remmods-bin`). The AUR package repackages the release's
  deb and checks it against its SHA-256.
- **Windows**: run the setup. It installs for the current user only (no administrator prompt) into
  `%LOCALAPPDATA%\RemMods` and downloads WebView2 if it is missing.

**From this repository (CachyOS / Arch, no package):** `./cachyos-install.sh` builds the release
binary when the sources are newer than it and installs it for your user only, without root and
without a package manager:

- the program as `~/.local/bin/remmods` and a marker `~/.local/lib/remmods/install-source`, so
  Settings → Updates shows "Local install (cachyos-install.sh)";
- the icons into `~/.local/share/icons/hicolor/` and the menu entry
  `~/.local/share/applications/remmods.desktop` (taskbar icon and "Start in safe graphics mode"
  included).

```sh
./cachyos-install.sh               # build when stale, then install; run it again to update
./cachyos-install.sh --build       # force a fresh release build first
./cachyos-install.sh --uninstall   # remove the installed files
```

It needs the build tools from [Development](#development). `PREFIX=/other/prefix` installs below
another prefix instead of `~/.local`.

Removing RemMods keeps its settings, profiles and logs (Linux: `~/.config/com.remirudev.remmods/` and
`~/.local/share/com.remirudev.remmods/`; Windows: unless you tick "delete app data" in the
uninstaller).

**Check your download.** Every release has a `SHA256SUMS` file. Download it into the same folder,
then on Linux:

```sh
sha256sum --check --ignore-missing SHA256SUMS
```

Each downloaded file must print `OK`. On Windows (PowerShell), compare the hash with the installer's
line in `SHA256SUMS`:

```powershell
Get-FileHash .\RemMods_*_x64-setup.exe -Algorithm SHA256
```

Every package carries `THIRD-PARTY-NOTICES` (Rust crates) and `THIRD-PARTY-NOTICES-NPM` (npm
packages and fonts): on Linux in `/usr/share/doc/remmods/`, on Windows as `.txt` files in the
install folder. Settings → About shows them too.

## Updates

RemMods never checks for updates on its own: Settings → Updates checks when you click "Check for
updates" (it then reads the release information from GitHub). Only stable releases are offered.
Settings → Updates also shows how this copy was installed.

| Installed as | How it updates |
|--------------|----------------|
| AppImage | "Install and restart" in Settings → Updates. After the confirmation "RemMods will close to install the update." it downloads the new AppImage, checks its signature, replaces the file in place and starts again. The previous version is not kept. |
| Windows installer | The same button. RemMods closes, the installer shows its progress and starts RemMods again. |
| deb, rpm | Settings → Updates says that a new version exists. Download the new package from the releases page and install it like the first one (`sudo apt install ./RemMods_*_amd64.deb`, `sudo dnf install ./RemMods-*-1.x86_64.rpm`). |
| AUR | Update with your AUR helper, for example `paru -Syu`. |
| Local install (`cachyos-install.sh`) | Pull the new sources and run `./cachyos-install.sh` again; it rebuilds and replaces the installed binary. |
| Built from source, CI test builds | No update channel ("This build has no update channel"). |

Updates are signed. RemMods installs one only after its signature matches the key built into the
app, and not while a mod install or an unfinished mod change is pending.

## Steam Deck

Use the AppImage in **Desktop Mode** (SteamOS 3.7 or newer). Download it with the browser, make it
executable (Dolphin: Properties → Permissions → "Is executable") and start it; Gear Lever (from
Discover) adds it to the menu and keeps it in one place, so it can update itself. If it does not
start because FUSE is missing, start it from Konsole with `--appimage-extract-and-run`:

```sh
~/Applications/RemMods_*_amd64.AppImage --appimage-extract-and-run
```

RemMods finds Steam and TEKKEN 8 as on any Linux PC. Game Mode is not supported: switch to Desktop
Mode to manage mods, then go back to play.

## Windows

The Windows build is **beta, CI-tested only**: CI builds it, installs it and runs its self-test, but
it has not been tested on a real PC yet. Moving mods to the Trash puts them into the drive's Recycle
Bin, where you can restore them (CI-tested only; WG-5). When a drive has no Recycle Bin, or it is turned
off or too small (network shares, many USB sticks), RemMods says so and deletes nothing; "Delete
permanently" then removes the mod after a second confirmation. A mod folder that another program holds
(an Explorer window, an antivirus scan) is retried for up to 30 s behind a toast with Cancel (WG-2).

The installer is **not code-signed** yet:

- Microsoft Defender SmartScreen shows "Windows protected your PC". Choose "More info", then
  "Run anyway".
- **Smart App Control** blocks unsigned apps, and it has no per-app exception. With Smart App Control
  on, RemMods cannot be installed.

Each release's installer is submitted to Microsoft for analysis, and code signing is planned after
the first release.

## Graphics problems (Linux)

RemMods draws its window with WebKitGTK. If the window stays blank, flickers or RemMods crashes at
start (some NVIDIA and Wayland setups), use a safer graphics mode:

- **The launcher action "Start in safe graphics mode"** (right-click RemMods in the menu or the
  taskbar) starts RemMods with software rendering for this session. If RemMods is already running,
  the action restarts it in safe graphics mode, which also works when its window is blank.
- **`--safe-graphics`** on the command line does the same, for example
  `~/Applications/RemMods_*_amd64.AppImage --safe-graphics` or `remmods --safe-graphics`.
- **Settings → Graphics** stores the mode for every start: Automatic, "GPU, shared memory" or
  Software. It applies after RemMods restarts. A safe-graphics session offers "Always use software
  rendering".
- **Automatic step**: if a start did not get as far as a working window (a crash, or the window was
  closed before the interface loaded), the next start switches one step down (Automatic → "GPU,
  shared memory" → Software) and shows a "Graphics mode changed" banner with "Keep" and "Switch
  back".

The environment variables RemMods sets for itself:

| Mode | Variables |
|------|-----------|
| Automatic | none |
| GPU, shared memory | `WEBKIT_DMABUF_RENDERER_FORCE_SHM=1` |
| Software | `WEBKIT_DISABLE_DMABUF_RENDERER=1` |
| any mode, NVIDIA driver on Wayland | also `__NV_DISABLE_EXPLICIT_SYNC=1` |

**Your environment wins.** If you set `WEBKIT_DMABUF_RENDERER_FORCE_SHM`,
`WEBKIT_DISABLE_DMABUF_RENDERER` or `WEBKIT_DISABLE_COMPOSITING_MODE` yourself, RemMods sets no
WebKit variable of its own and never steps the mode; a variable you set from the table keeps your
value. Programs RemMods starts (Steam, the file manager) never get RemMods' own variables or the
AppImage's environment. The stored mode is in `~/.config/com.remirudev.remmods/graphics.json`.

## Development

You need:

- **Rust via rustup.** `rust-toolchain.toml` pins Rust 1.99.1 with clippy and rustfmt, and rustup
  installs it on first use (or run `rustup toolchain install`). Distribution `rust` packages ignore that
  file, so make sure `cargo` comes from rustup (`~/.cargo/bin`).
- **Node 26** (`.node-version`) with npm.
- **Linux libraries** for Tauri:
  - Arch / CachyOS: `sudo pacman -S --needed base-devel webkit2gtk-4.1 librsvg openssl`
  - Ubuntu / Debian: `sudo apt install build-essential pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev
    librsvg2-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev libwayland-dev libssl-dev`
- **Windows**: the Microsoft C++ Build Tools ("Desktop development with C++") and WebView2, which
  Windows 10 22H2 and 11 already include. Run the scripts in `scripts/` from Git Bash.

Run the app in development mode (Vite on port 1430 with hot reload):

```sh
npm ci
npm run tauri dev
```

The checks CI runs on every pull request:

```sh
npm run lint && npm run typecheck && npm test && npm run build
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
npm run bindings      # regenerates src/lib/ipc/bindings.ts; commit it when it changes
scripts/check-cfg.sh && scripts/check-native.sh && scripts/check-commands.sh
```

The Playwright flows (Chromium only) drive the UI against a mock backend that `vite --mode e2e`
injects (the `e2e` job in CI, which installs Playwright's own Chromium). Locally, point them at the
system Chromium instead of downloading one:

```sh
REMMODS_E2E_CHROMIUM=/usr/bin/chromium npm run e2e        # bash, zsh
env REMMODS_E2E_CHROMIUM=/usr/bin/chromium npm run e2e    # fish
npm run e2e:serve     # only the mock-backed UI on http://127.0.0.1:1431, to open in a browser
```

A separate `fs-matrix` job runs the rename, rollback and recovery tests on real tmpfs, ext4, btrfs,
vfat and exfat mounts (plus read-only, root-owned, full-disk and tiny-inotify-queue cases). It needs
`sudo` for the mounts (`scripts/fs-matrix-mount.sh`), so it runs in CI only; locally
`cargo test -p remmods-core --test fs_matrix` passes without doing anything.

Build the app and run its self-test (needs a display; exits 0 when every check passes):

```sh
npm run tauri -- build --debug --no-bundle
./target/debug/remmods --smoke
```

The smoke test can also scan a fixture game folder. The script writes one (it refuses a non-empty
target) and prints the Paks path:

```sh
python3 -I scripts/make-fixtures.py smoke /tmp/remmods-fixture
./target/debug/remmods --smoke --paks "/tmp/remmods-fixture/steamapps/common/TEKKEN 8/Polaris/Content/Paks"
```

Hostile and odd zips for the archive reader (traversal names, forged headers, legacy encodings) come
from a second script; `archive_fixtures` checks each against `manifest.json` on both OSes:

```sh
python3 -I scripts/make-archive-fixtures.py /tmp/archive-fixtures
REMMODS_ARCHIVE_FIXTURES=/tmp/archive-fixtures cargo test -p remmods-core --test archive_fixtures
```

`npm run tauri -- build --no-bundle` builds the optimised (LTO) app in `target/release/`.

### Packages and releases

Locally, build and inspect only the deb and rpm (nothing is installed):

```sh
npm run tauri -- build --bundles deb,rpm
python3 -I scripts/inspect-bundle.py --expect deb target/release/bundle/deb/RemMods_*_amd64.deb
python3 -I scripts/inspect-bundle.py --expect rpm target/release/bundle/rpm/RemMods-*.x86_64.rpm
```

The AppImage and the Windows installer are built only in CI (`.github/workflows/artifacts.yml`, which
also installs every package and runs `--smoke`). Never build locally with
`src-tauri/tauri.release.conf.json`: only release builds carry the updater configuration. The AUR
files can be rendered from a local deb and compared with makepkg:

```sh
python3 -I scripts/render-pkgbuild.py --deb target/release/bundle/deb/RemMods_0.1.0_amd64.deb --version 0.1.0 --local --out /tmp/remmods-aur
cd /tmp/remmods-aur && makepkg --printsrcinfo | diff - .SRCINFO
```

`THIRD-PARTY-NOTICES` and `THIRD-PARTY-NOTICES-NPM` are committed and CI checks that they are
current, so regenerate them after every `Cargo.lock` or `package-lock.json` change (it needs
cargo-about 0.9.3: `cargo install --locked --features cli cargo-about@0.9.3`):

```sh
bash scripts/notices.sh            # or: bash scripts/notices.sh --check
```

The release gate and the workflow rules run in CI's `policy` job; locally:
`python3 -I scripts/release-check.py` and `python3 -I scripts/check-workflows.py` (needs PyYAML).
Releases are cut by tagging main (`.github/workflows/release.yml`); the owner's steps, including the
signing key and the AUR account, are in [docs/release-runbook.md](docs/release-runbook.md).
