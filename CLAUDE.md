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
| Shell | Tauri 2.12.5; plugins single-instance, log, window-state, opener. No fs, shell or process plugins |
| IPC | tauri-specta + specta `=2.0.0-rc.26`, specta-typescript 0.0.13; bumped together by hand |
| UI | React 19.3, Mantine 9.7 (Quiet Canvas theme), TanStack Query, zustand (UI state only), i18next EN + DE |
| Tooling | Node 26, Vite 8, TypeScript 6.0 (not 7: no tsserver for WebStorm), Biome 2.6, Vitest 5 + jsdom |
| CI | GitHub Actions on ubuntu-24.04 and windows-2025; cargo-deny 0.20.2; Dependabot weekly |

## Layout

```
crates/remmods-core/   all logic, synchronous, no Tauri or specta
  src/platform/        the only place for cfg and unsafe; one API, identical signatures on both OSes
  src/safe_fs/         the only place that mutates the file system: scope, rename, batch, journal, gate,
                       watch (names.rs: Windows name rules)
  src/game/            Steam detection, game folder layout, status, process tracker, launch
  src/mods/            Paks scan, mod rows, conflicts, ModRef tokens, details
  src/content/         per-container .utoc cache, row content summaries, content conflicts, base index
  tests/fixtures/      proc/ (fake /proc captures) and steam/ (VDF/ACF files); byte-exact, outside Biome
crates/remmods-pak/    pure-Rust .utoc reader (header + directory index; no decompression, no Oodle); no core dep
crates/remmods-ipc/    DTOs + AppError (serde + specta, no Tauri) and their tests
src-tauri/             thin shell (package remmods, lib remmods_lib); no Rust tests
  src/commands/        one file per domain; async, spawn_blocking, Result<T, AppError>
  src/os/              the only other cfg place (navigation, webview checks)
  src/watcher.rs       Paks watcher thread: debounced rescans, auto-refresh status
  src/bin/             export-bindings (feature "bindings") writes src/lib/ipc/bindings.ts
src/                   React app: app/ (shell, title bar, rail, smoke), features/<x>/, shared/, lib/, locales/
scripts/               check-cfg/native/commands.sh, fs-matrix-mount.sh (CI only), paks-snapshot.sh, make-fixtures/scan-expect/proc-capture/utoc-list.py, utoc-compare.sh
docs/                  PLAN.md, REVIEW-LOG.md, windows-gaps.md, checklists/
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
- `#![deny(unsafe_code)]` in core; only `platform` may use `unsafe`.
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
  user picked in the native save dialog (`WriteGuard::write_export`; pending owner approval, SPEC-P3 A3).
- The Trash is reached only through `platform::trash_location` + `safe_fs` `WriteGuard::trash`; tests
  pass a `TrashEnv` inside their TempDir and never use `TrashEnv::from_env()` (only the shell does).
  Permanent delete needs a token from `mods_delete_prepare` (the second confirmation);
  `remove_aside` is the only recursive delete under a mod root.
- Pure-Rust crates preferred. `deny.toml` bans C/asm crates and features; `scripts/check-native.sh`
  checks both targets' dependency trees and Linux build-script outputs.

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
python3 -I scripts/make-fixtures.py smoke <empty dir>   # fixture game folder; then: remmods --smoke --paks <printed Paks path>
python3 -I scripts/scan-expect.py <paks.tsv>            # expected scan-* log lines from a paks-snapshot
scripts/utoc-compare.sh <dir>                           # Rust vs Python .utoc listing (read-only)
python3 -I scripts/proc-capture.py <label> > local/proc-<label>.json   # redact by hand before committing
scripts/paks-snapshot.sh > local/paks-<label>.tsv   # read-only listing of the game's Paks folder
```

CI (`.github/workflows/ci.yml`) has the required checks `linux`, `windows`, `smoke (linux)`,
`smoke (windows)` and `policy`. Actions are pinned by full commit SHA with a `# vX.Y.Z` comment;
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
