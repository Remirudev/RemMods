# Windows release checklist

**Status: later.** There is no Windows PC, VM or tester yet (docs/PLAN.md §15). Until this list has
run once on a real PC or VM, Windows builds are labelled "beta, CI-tested only" (§1, §14).

Already covered by CI on windows-2025: every core test, the `smoke (windows)` launch of the built app,
the `interop-diff` job against the original's code (from P3) and a silent NSIS install with `--smoke`
in the artifacts workflow (in place since P5: the `nsis` job of `.github/workflows/artifacts.yml`
installs per user with `/S`, checks that `--smoke` reports the install kind `nsis` and uninstalls with
`/S`). This list is what CI cannot see.

## Install and first start

- [ ] The NSIS installer (per user, no UAC) installs and starts RemMods. Note the SmartScreen prompt
      ("More info", then "Run anyway").
- [ ] Smart App Control (Windows 11, if the machine has it on): it blocks the unsigned installer with
      no per-app exception, as the README says. Record the exact message (WG-10).
- [ ] The install folder `%LOCALAPPDATA%\RemMods` holds `LICENSE.txt`, `THIRD-PARTY-NOTICES.txt` and
      `THIRD-PARTY-NOTICES-NPM.txt`, and Settings → About → "Licences and third-party notices" shows
      the three texts.
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
- [ ] Record which holder Restart Manager names while Defender scans a freshly installed mod.

## Shell integration

- [ ] The release build suppresses F5, Ctrl+R, Ctrl+Shift+R, Ctrl+P, Ctrl+F, F3, F7 and the browser
      context menu.
- [ ] Dropping an archive from Explorer starts an install.

## Coexistence and updates

- [ ] Coexistence UI check with the original TekkenModManager v1.1.1: the banner shows while
      `ModManager.exe` runs, and delete / rename / overwrite ask first. (The file-level rules are
      covered by `interop-diff`.)
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

## Runs

| Date | Machine (Windows version, PC or VM) | RemMods version | Result | Notes |
|------|-------------------------------------|-----------------|--------|-------|
