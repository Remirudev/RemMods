# RemMods

A mod manager for TEKKEN 8 on Linux and Windows. It turns pak mods (`Mods`, `~mods`) and LogicMods on
and off, installs them from archives, keeps profiles of which mods are active, and repairs what other
tools leave behind. RemMods is a port of [TekkenModManager](https://github.com/UnfugPsy/TekkenModManager)
by Unfug to Tauri 2, Rust and React. It keeps the original's folder layout, so both apps can manage the
same game folder.

RemMods only ever writes inside the three mod folders, its own app folders and the Trash. Steam's
files are read-only to it. Removing a mod moves it to the Trash; deleting it permanently needs a second
confirmation.

## Status

**Phase 1 (read-only)**: RemMods finds the game (Steam detection or a picked folder), scans `Mods`,
`~mods` and `LogicMods`, shows every mod with its state and conflicts, shows whether the game is
running, and launches it through Steam. It does not change your mod files yet. The full plan,
including the phases and their exit criteria (§13), is in [docs/PLAN.md](docs/PLAN.md).

## Platforms

| Platform | Support |
|----------|---------|
| Linux (first) | glibc 2.39 or newer: Ubuntu 24.04+, Linux Mint 22+, Debian 13+, Fedora 40+, SteamOS 3.7+, Arch and CachyOS |
| Windows | Windows 10 22H2 and Windows 11, x64, with WebView2 Evergreen: **beta, CI-tested only** until it has been tested on a real PC |

Not planned for v1: macOS, Flatpak and Snap packages, Steam Deck Game Mode.

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

`npm run tauri -- build --no-bundle` builds the optimised (LTO) app in `target/release/`. Installers
(AppImage, deb, rpm, NSIS) are built by CI from Phase 5 on.

## Credits and licence

RemMods is MIT-licensed (see [LICENSE](LICENSE)). It is based on
[TekkenModManager](https://github.com/UnfugPsy/TekkenModManager) by **Unfug**, also MIT; its copyright
notice is kept in the licence file. Thanks to Unfug for the original app and its on-disk conventions.
