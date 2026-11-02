# RemMods

Mod manager for TEKKEN 8: Tauri 2 shell, Rust core, React 19 + Mantine 9 UI. Linux first (CachyOS,
KDE Plasma 6 Wayland is the dev machine), Windows working at all times. Port of Unfug's
TekkenModManager; folders made by either app keep working in both.

The plan is `docs/PLAN.md` (v3.2, the owner's decisions in §15; phases and exit criteria in §13), the review
trail `docs/REVIEW-LOG.md`. Read the plan section for the area you touch first. Where code and plan
disagree, ask before changing either.

## Stack

| Layer | Choice |
|---|---|
| Rust | 1.99.1 pinned in `rust-toolchain.toml`, edition 2024, resolver 3; exact `=` pins in `[workspace.dependencies]` |
| Shell | Tauri 2.12.5; plugins single-instance, log, window-state, opener, updater 2.14.1 (Rust only, release builds only). No fs, shell or process plugins |
| IPC | tauri-specta + specta `=2.0.0-rc.26`, specta-typescript 0.0.13; bumped together by hand |
| UI | React 19.3, Mantine 9.7 (Quiet Canvas theme), TanStack Query, zustand (UI state only), i18next EN + DE |
| Tooling | Node 26, Vite 8, TypeScript 6.0 (not 7: no tsserver for WebStorm), Biome 2.6, Vitest 5 + jsdom |
| CI | GitHub Actions on ubuntu-24.04 and windows-2025; cargo-deny 0.20.2, cargo-about 0.9.3; Dependabot weekly |
| Packages | AppImage, deb + rpm (package `rem-mods`), AUR `remmods-bin` (repackaged deb), NSIS per user; built only in CI |

## Layout

```
crates/remmods-core/   all logic, synchronous, no Tauri or specta
  src/platform/        the only place for cfg and unsafe; one API, identical signatures on both OSes
                       (host_env.rs: cfg-free env plan for host_command, CLI paths against $OWD)
  src/safe_fs/         the only place that mutates the file system: scope, rename, batch, journal, gate,
                       watch (names.rs: Windows name rules; app_files.rs: removing one app-folder file)
  src/game/            Steam detection, game folder layout, status, process tracker, launch
  src/mods/            Paks scan, mod rows, conflicts, ModRef tokens, details
  src/content/         per-container .utoc cache, row content summaries, content conflicts, base index
  src/archive/         zip/7z/rar/folder readers (one backend file per crate), names, limits, capped sink
  src/install/         inspect (root, name, variants, issues) and start (stage, commit, job), no fs mutator
  src/app_update/      install-kind allowlist, AUR marker, version rules, update preflight + gate seal; no network
  src/graphics/        Linux graphics fallback: modes, env rules, graphics.json, crash marker; cfg-free
  tests/fixtures/      proc/ (fake /proc captures) and steam/ (VDF/ACF files); byte-exact, outside Biome
crates/remmods-pak/    pure-Rust .utoc reader (header + directory index; no decompression, no Oodle); no core dep
crates/remmods-ipc/    DTOs + AppError (serde + specta, no Tauri) and their tests
src-tauri/             thin shell (package remmods, lib remmods_lib); no Rust tests
  src/commands/        one file per domain; async, spawn_blocking, Result<T, AppError>
  src/os/              the only other cfg place (navigation, webview checks; graphics.rs sets the env)
  src/watcher.rs       Paks watcher thread: debounced rescans, auto-refresh status
  src/install_sources.rs, src/drop.rs   dropped/picked sources registered by id (no paths over IPC)
  src/bin/             export-bindings (feature "bindings") writes src/lib/ipc/bindings.ts
  linux/remmods.desktop  desktop template for deb/rpm/AppImage (StartupWMClass, safe-graphics action)
  tauri.{linux,windows}.conf.json  bundle targets, depends, notices files; tauri.release.conf.json:
                       updater config + updater artifacts, used only by release.yml
src/                   React app: app/ (shell, title bar, rail, smoke), features/<x>/, shared/, lib/, locales/
scripts/               check-cfg/native/commands.sh, fs-matrix-mount.sh and smoke-paks-diff.sh (CI only), paks-snapshot.sh, make-fixtures/make-archive-fixtures/scan-expect/proc-capture/utoc-list.py, utoc-compare.sh;
                       release: release-check/check-workflows/check-latest-json/make-latest-json/render-pkgbuild/inspect-bundle/npm-notices.py, notices.sh, seed-appimage-tools.sh (CI only)
packaging/             aur/PKGBUILD.in (remmods-bin), release-notes.md (draft release text, @VERSION@)
about.toml, about.hbs  cargo-about config + template → THIRD-PARTY-NOTICES; npm-notices.py → THIRD-PARTY-NOTICES-NPM
.github/workflows/     ci.yml (merge gate), release.yml (v* tags), artifacts.yml (installs every package, --smoke)
docs/                  PLAN.md, REVIEW-LOG.md, windows-gaps.md, release-runbook.md (owner), checklists/
```

A new feature is a core module + `commands/<feature>.rs` + DTOs in `remmods-ipc` + `src/features/<feature>/`
+ `locales/{en,de}/<feature>.ts` + tests that run on both OSes (PLAN §3).

## Code rules

- Files: ≤500 lines as the target, 1000 hard max. Small focused functions with descriptive names.
- One React component per file. Types go in a `types.ts` per module, not inline in components.
- No cross-feature imports: `src/features/<x>` never imports another feature; share through
  `src/shared/` or `src/lib/`.
- At most 3 directory levels under `src/`. Descriptive file names (no `utils.ts`, `helpers.ts`).
  Stores ≤200 lines.
- Comments: none that restate code. Short "why" comments only for platform quirks and safety invariants.
- UI: Mantine only (no Tailwind, no shadcn). No native `<select>`, checkbox or `<dialog>`, no
  `confirm()`/`alert()`. No `backdrop-filter`/blur (WebKitGTK GPU issue). Icons from
  `@tabler/icons-react` (`size={20} stroke={1.8}` in the rail). Token prefixes: `--qc-` for Quiet
  Canvas, `--rmods-` for app tokens.
- WebKitGTK: `-webkit-user-select: none` on app chrome, every pane in a ScrollArea, no
  `scrollbar-color`, fonts bundled via @fontsource.
- Rust: one thiserror enum per module; no `unwrap()`/`expect()` in non-test code unless provably
  infallible (clippy's `unwrap_used`/`expect_used` enforce it: add an `#[allow]` and say why in a
  short comment). Test helpers outside `#[test]` functions need `#![allow(clippy::expect_used)]`.
- IPC: DTOs are camelCase with no `PathBuf`/`OsString` (display strings only) and no `u64`/`usize`/`i64`
  (use `f64` or `u32`); every `f64` field carries `#[specta(type = specta_typescript::Number)]`;
  struct variants inside `#[serde(tag = "kind")]` enums use single-word field names only. `AppError` is tagged by `kind` and carries no English; the UI translates it.

## Platform and file-system rules

- `cfg(windows | unix | target_os …)` only in `crates/remmods-core/src/platform/**`,
  `src-tauri/src/os/**`, `crates/*/tests/platform_*.rs`, `build.rs` files and Cargo target-specific
  dependency tables. `scripts/check-cfg.sh` enforces it in CI.
- `#![deny(unsafe_code)]` in core; only `platform` may use `unsafe`. In the shell the only `unsafe` is
  `env::set_var` in `src-tauri/src/os/graphics.rs`, before any thread exists (`// SAFETY:` line).
- The graphics env vars (`WEBKIT_*`, `__NV_DISABLE_EXPLICIT_SYNC`) are set only in
  `src-tauri/src/os/graphics.rs`; the rules (modes, user env wins, crash marker) live in core
  `graphics/`. Windows reports the fallback as `unsupported` (WG-8).
- Every program RemMods starts on Linux (launch, reveal fallback, `open_logs`, the AppImage relaunch)
  goes through `platform::host_command`: it drops AppImage/AppRun values and RemMods' own graphics vars
  and starts in `$HOME`. On Linux never a bare `Command::new` or an opener call that spawns a
  program (`open_path`, `open_url`); Windows keeps the opener.
- Every `std::fs` call on a path inside the game folder goes through `platform::fs_path`; which form
  a returned path carries (plain or verbatim) is fixed in PLAN/SPEC §4.2.
- Classify OS errors only through `platform::classify`: raw codes collide (Windows 5/32/33 are
  EIO/EPIPE/EDOM on Linux).
- Every file-system mutation goes through `remmods_core::safe_fs`; clippy's `disallowed-methods`
  (`clippy.toml`) bans the std mutators everywhere else: `std::fs` writes, copies, renames,
  removals and links, opening for writing (`OpenOptions::write/append/create/create_new/truncate`,
  `File::create*`), `File::set_len/set_permissions/set_times/set_modified`, `DirBuilder::create`
  and the OS symlink functions, plus the rustix and windows-sys rename, link, unlink and mkdir calls.
  A new mutator API gets added there. Mod paths change only through `OpGate::write` and
  `WriteGuard::rename_batch`; fault injection in tests is a runtime `FaultPlan` in `BatchOptions`,
  not a cargo feature. JSON is written with
  `write_atomic` under the `AppLock`; a corrupt file is backed up before it is replaced.
- Writes only inside the three mod roots, RemMods' app folders and the Trash. Steam files are
  read-only. Removal is a rename; permanent delete needs a second confirmation. RemMods never writes
  the original's `profiles.json`. One exception: a profile export writes the single `.json` file the
  user picked in the native save dialog (`WriteGuard::write_export`; approved 2026-11-01, SPEC-P3 A3).
  Second exception, self-update (approved 2026-11-02, SPEC-P5 A9): only inside `update_install`,
  after the install-kind allowlist (AppImage with a valid `$APPIMAGE`, or NSIS) and with the OpGate
  sealed, the updater plugin rewrites the file at `$APPIMAGE` (it moves the previous AppImage into a
  private temp folder on the same device and deletes it once the new file is written, so the old
  version is not kept) or, on Windows, writes the installer to `%TEMP%` and NSIS installs per user.
  The update confirmation is the only confirmation. deb, rpm and AUR installs never self-update; the
  updater never runs `dpkg`, `rpm` or `pkexec`.
- The Trash is reached only through `platform::trash_location` + `safe_fs` `WriteGuard::trash`; tests
  pass a `TrashEnv` inside their TempDir and never use `TrashEnv::from_env()` (only the shell does).
  Permanent delete needs a token from `mods_delete_prepare` (the second confirmation);
  `remove_aside` and `discard_staging` (one shared inert-temp-folder check) are the only recursive
  deletes under a mod root.
- Archives are read only through `remmods_core::archive` (one backend file per crate, never library
  extract helpers); extraction writes only through `safe_fs::StagingDir`/`StagedFile` into a journaled
  `_install_tmp_<32hex>` staging folder, capped at each entry's declared size; installs commit through
  `WriteGuard::rename_batch_replacing`.
- Pure-Rust crates preferred. `deny.toml` bans C/asm crates and features; `scripts/check-native.sh`
  checks both targets' dependency trees and Linux build-script outputs.

## Releases and packaging

- The updater is used from Rust only: no `@tauri-apps/plugin-updater`, no `updater:*` permission. The
  plugin is registered only when the config has `plugins.updater` and never under `--smoke`; no test
  and no `--smoke` run touches the network.
- `src-tauri/tauri.release.conf.json` (updater config, `createUpdaterArtifacts`) is used only by
  `release.yml`, which builds with `--no-sign` and signs in a separate `sign` job. Never build with it
  locally. The pubkey stays `REMMODS-UPDATER-PUBKEY-NOT-SET` until the owner sets it
  (docs/release-runbook.md).
- AppImage and NSIS bundles are built only in CI; locally only `--bundles deb,rpm`. Never download and
  run linuxdeploy, appimagetool, AppRun or makensis (`scripts/seed-appimage-tools.sh` is CI-only).
  Never install a package on this machine (no `dpkg -i`, `rpm -i`, `pacman -U`, `makepkg -i/-s`).
- A `@tauri-apps/cli` bump (Dependabot included) fails W11 until the new tauri-bundler's
  `prepare_tools` (AppImage tool names, URLs, tools folder) is checked against
  `scripts/seed-appimage-tools.sh` and its `VERIFIED_CLI` is updated (by the owner or the main
  session; the post-build `--verify-cache` step catches a bundler that downloaded its own tools).
- Regenerate `THIRD-PARTY-NOTICES` and `THIRD-PARTY-NOTICES-NPM` after every `Cargo.lock` or
  `package-lock.json` change (`bash scripts/notices.sh`, Dependabot PRs included): CI's `policy` and
  `linux` jobs fail on stale notices. `LICENSE` is shipped and embedded as-is.
- Workflows name secrets only (environment `release`: `TAURI_SIGNING_PRIVATE_KEY`,
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, `AUR_SSH_PRIVATE_KEY`; variables `AUR_KNOWN_HOSTS`,
  `AUR_COMMIT_EMAIL`). Claude never creates, prints or stores keys or passwords, and never pushes
  tags, publishes releases or pushes to the AUR. `scripts/check-workflows.py` enforces the workflow
  rules (pinned actions, no `${{ }}` in `run:`, secrets only in the `release` environment).
- A release needs zero open release-blocking entries in `docs/windows-gaps.md`; release.yml's `verify`
  job refuses the tag otherwise (`release-check.py --strict`).

## Windows at all times

- Everything must compile and pass tests on windows-2025, though only Linux runs here. Before
  pushing Rust changes, lint the whole workspace for Windows like the `windows` CI job does (it
  checks, never links, so it works here):
  `cargo clippy --locked --target x86_64-pc-windows-msvc --workspace --all-targets -- -D warnings`.
  tauri-winres prints "GNU compiler is not supported for this target"; that warning is expected.
- Build paths with `Path`/`PathBuf`, never by joining strings with `/`. No Linux-only shell in npm
  scripts. Tests that use temp dirs canonicalise them (Windows runners use 8.3 names). LF line endings
  everywhere (`.gitattributes`).
- Same behaviour on both OSes: Windows name rules, case-insensitive collision checks, never-overwrite
  renames; IPC bindings and UI strings byte-identical.
- A Linux-first feature reports `unsupported` on Windows and gets a `docs/windows-gaps.md` entry; the
  PR template's "Windows impact" line says which case applies.

## Commands

```sh
npm run tauri dev                 # app with Vite on :1430 (needs a display)
npm run lint                      # biome ci .   (npm run check fixes, npm run format formats)
npm run typecheck                 # tsc -b
npm test                          # vitest run (projects: unit = node, dom = jsdom)
npm run build                     # tsc -b && vite build
npm run bindings                  # regenerate src/lib/ipc/bindings.ts (commit it)
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test -p remmods-core --test fs_matrix   # no-op locally; the real run is the fs-matrix CI job (needs sudo)
scripts/check-cfg.sh && scripts/check-native.sh && scripts/check-commands.sh
npm run tauri -- build --debug --no-bundle && ./target/debug/remmods --smoke
npm run tauri -- build --bundles deb,rpm          # local packages only (never appimage/nsis, never the release config)
python3 -I scripts/inspect-bundle.py --expect deb target/release/bundle/deb/RemMods_*_amd64.deb   # or --expect rpm <rpm>
python3 -I scripts/render-pkgbuild.py --deb <deb> --version 0.1.0 --local --out <empty dir>   # AUR files; .SRCINFO must equal makepkg --printsrcinfo
bash scripts/notices.sh [--check] [rust|npm|all]  # THIRD-PARTY-NOTICES{,-NPM}; needs cargo install --locked --features cli cargo-about@0.9.3
python3 -I scripts/release-check.py               # release gate, CI mode (release.yml: --tag vX.Y.Z --strict)
python3 -I scripts/check-workflows.py             # workflow rules W1-W11 (needs PyYAML); every release .py script except npm-notices.py has --self-test (npm-notices.py --check runs in CI's linux job)
python3 -I scripts/make-fixtures.py smoke <empty dir>   # fixture game folder; then: remmods --smoke --paks <printed Paks path>
python3 -I scripts/scan-expect.py <paks.tsv>            # expected scan-* log lines from a paks-snapshot
python3 -I scripts/make-archive-fixtures.py <empty dir> # crafted zips + manifest.json; REMMODS_ARCHIVE_FIXTURES=<dir> cargo test -p remmods-core --test archive_fixtures
scripts/utoc-compare.sh <dir>                           # Rust vs Python .utoc listing (read-only)
python3 -I scripts/proc-capture.py <label> > local/proc-<label>.json   # redact by hand before committing
scripts/paks-snapshot.sh > local/paks-<label>.tsv   # read-only listing of the game's Paks folder
```

CI (`.github/workflows/ci.yml`) has the required checks `linux`, `windows`, `smoke (linux)`,
`smoke (windows)` and `policy`. `artifacts.yml` (not required) runs on packaging-path PRs, weekly and
from `release.yml`. Actions are pinned by full commit SHA with a `# vX.Y.Z` comment;
look SHAs up with `git ls-remote`.

## Claude Code sandbox

- `export CARGO_HOME="$TMPDIR/cargo-home"` before cargo: `~/.cargo` is read-only in the sandbox, but
  the rustup proxies in `~/.cargo/bin` and the toolchains in `~/.rustup` work.
- `export npm_config_cache="$TMPDIR/npm-cache"` before npm or npx: `~/.npm` is read-only too.
- Network goes through a proxy; pass `allowed_domains`: `index.crates.io`, `static.crates.io`,
  `crates.io` (crates), `registry.npmjs.org` (npm), `github.com`, `api.github.com`,
  `codeload.github.com`, `raw.githubusercontent.com`, `objects.githubusercontent.com` (GitHub).
- The sandbox refuses `npm install` / `npm ci` even with `allowed_domains` (`npm view` passes).
  The main session runs it with the sandbox off, which the user approves; subagents never bypass
  the sandbox and report it instead.
- Anything that opens a window (`npm run tauri dev`, `remmods --smoke`) needs the real display, so
  it also runs only with the sandbox off, under the same rule.
- No sudo. Commit, push or switch branches only when the user asks.
- The user's shell is fish; commands meant for them must work in fish.
- The game's Paks folder is only touched during the in-game checklist (`docs/checklists/ingame-linux.md`),
  after a backup, together with the user.
