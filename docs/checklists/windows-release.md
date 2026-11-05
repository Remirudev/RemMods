# Windows release checklist

**Status: later.** There is no Windows PC, VM or tester yet (docs/PLAN.md §15). Until this list has
run once on a real PC or VM, Windows builds are labelled "beta, CI-tested only" (§1, §14).

Already covered by CI on windows-2025: every core test, the `smoke (windows)` launch of the built app
and a silent NSIS install with `--smoke` in the artifacts workflow (in place since P5: the `nsis` job
of `.github/workflows/artifacts.yml` installs per user with `/S`, checks that `--smoke` reports the
install kind `nsis` and uninstalls with `/S`). This list is what CI cannot see.

## Install and first start

- [ ] The NSIS installer (per user, no UAC) installs and starts RemMods. Note the SmartScreen prompt
      ("More info", then "Run anyway").
- [ ] Smart App Control (Windows 11, if the machine has it on): it blocks the unsigned installer with
      no per-app exception, as the README says. Record the exact message (WG-10).
- [ ] The install folder `%LOCALAPPDATA%\RemMods` holds `THIRD-PARTY-NOTICES.txt` and
      `THIRD-PARTY-NOTICES-NPM.txt`, and Settings → About → "Third-party notices" shows both texts.
- [ ] Without WebView2, RemMods shows a native message with the Evergreen download link instead of
      exiting silently.
- [ ] Custom title bar: drag, double-click to maximise, minimise / maximise / close, resize from the
      edges and corners. Drag-to-edge snapping works (UNVERIFIED, §5); Windows 11 shows rounded corners
      and a shadow.
- [ ] After switching to the system title bar and restarting, the native caption shows the Snap
      Layouts flyout.

## Steam and the game

- [ ] Steam is found through the registry. Record the HKCU `SteamPath` value (casing, separators) and
      the HKLM `InstallPath` value (WOW6432Node).
- [ ] Record the Toolhelp32 process names of the running game.
- [ ] Launch through `steam://rungameid/1778820` starts the game.
- [ ] The folder picker (IFileDialog) opens and returns the chosen folder.
- [ ] Reveal selects the mod in Explorer; "Open Paks folder" opens the Paks folder.
- [ ] The game is detected through the registry (HKCU `SteamPath`, HKLM `InstallPath`) with the
      values recorded above.

## Files while the game runs

- [ ] TEKKEN 8 holds its paks without `FILE_SHARE_DELETE`: the lock probe reports `InUse`, and
      toggling while the game runs changes nothing on disk.
- [ ] Enabling a disabled mod while the game runs works.
- [ ] Record the `MoveFileExW` error code for an existing target (183 or 80).
- [ ] While Defender scans a freshly installed mod, toggling it shows the folder-busy toast or
      completes; record which (WG-1 holder names are deferred).

## Shell integration

- [ ] The release build suppresses F5, Ctrl+R, Ctrl+Shift+R, Ctrl+P, Ctrl+F, F3, F7 and the browser
      context menu.
- [ ] Dropping an archive from Explorer starts an install.

## Other tools and updates

- [ ] Another-mod-manager check: while a `ModManager.exe` runs, the banner shows and delete / rename /
      overwrite ask first.
- [ ] Settings → Updates shows "Windows installer". In a release build, "Check for updates" finds a
      newer release and offers "Install and restart"; the confirmation says "RemMods will close to
      install the update. The installer shows its progress and starts RemMods again."
- [ ] Passive update from the previous release (WG-9): install the previous release, then "Install and
      restart". RemMods closes, the installer's progress window appears without questions, and
      RemMods starts again showing the new version in Settings → Updates; settings, profiles and
      mods are unchanged.
- [ ] Uninstalling keeps the app data unless "delete app data" is ticked; with it ticked, both app
      folders are removed.
- [ ] A silent uninstall (`& "$env:LOCALAPPDATA\RemMods\uninstall.exe" /S` in PowerShell) keeps both app
      folders (`%APPDATA%\com.remirudev.remmods`, `%LOCALAPPDATA%\com.remirudev.remmods`).

## Recycle Bin and folder-busy wait (P3b)

CI covers the abort path and a real held folder handle (WG-5, WG-2 in docs/windows-gaps.md); these
are the cases only a real PC shows.

- [ ] Trash moves a mod folder into the Recycle Bin; it shows there under its name and restores to Paks.
- [ ] A loose mod (`.pak` group) moves to the Recycle Bin and restores.
- [ ] Bin turned off for the drive ("Don't move files to the Recycle Bin", or size 0) → "The Recycle
      Bin cannot be used" ("Windows would not move this folder to the Recycle Bin …") and nothing
      deleted; "Delete permanently" then removes it after the second confirmation.
- [ ] A mod on a network share or USB stick without a bin → the same refusal, nothing deleted.
- [ ] A folder named `Foo.` (made in cmd with `mkdir "\\?\<Paks>\Mods\Foo."`) next to `Foo`: Trash
      refuses with "Windows cannot move this name to the Recycle Bin. Rename it in Repair first, or
      delete it permanently.", both folders unchanged.
- [ ] Folder-busy: open a mod's folder in Explorer and disable that Logic mod (or rename a mod): after
      about a second the "Waiting for a folder in use" toast appears with the elapsed seconds; close
      Explorer → the change completes and the toast closes. Repeat and press Cancel (or Enter) →
      "Cancelled. No mod was changed."; the disk is unchanged.
- [ ] A file of a mod held open in another program (for example a hex editor) → Trash fails, the
      folder is complete.
- [ ] Recycle Bin size limit smaller than a loose mod's `.utoc` → the `.pak` is in the bin, the
      companions stay inert in Mods, the row is reported as failed.
- [ ] Bulk trash of three mods ("Move to Recycle Bin (3)") → "Moved 3 mods to the Recycle Bin.", three
      in the Recycle Bin.
- [ ] Only if CI ignored them (`#[ignore]` in `crates/remmods-core/tests/platform_windows_recycle.rs`):
      the W6–W10/W12 cases by hand.

## Skin sets

- [ ] Back up `%LOCALAPPDATA%\TEKKEN 8\Saved\SaveGames` first. Sets appear under Groups, and saving
      and switching behaves as in "Skin sets steps" of `ingame-linux.md`, with the game's own folder
      in place of the Proton prefix.
- [ ] Switching right after quitting the game still works: Windows may hold a file for a moment, and
      RemMods waits for it.

## Runs

| Date | Machine (Windows version, PC or VM) | RemMods version | Result | Notes |
|------|-------------------------------------|-----------------|--------|-------|
