# In-game smoke test (Linux)

Runs at the end of every phase (docs/PLAN.md §9, §13) and whenever a change affects what lands in
`Paks`. It is done together: Claude backs up, prepares and switches the files and gives step-by-step
instructions; you start the game, play in **offline Practice mode** and report what you see.

Paths below are relative to the Paks folder,
`~/.local/share/Steam/steamapps/common/TEKKEN 8/Polaris/Content/Paks`.

## Before the game starts

1. TEKKEN 8 is closed and Steam is not updating it.
2. Claude copies `Mods` (and `~mods` / `LogicMods` if present) to `~/RemMods-backups/<date>/`.
   Backups never go inside `Paks`: the game would load the `.pak` files in them.
3. Claude takes the "before" snapshot:
   `scripts/paks-snapshot.sh > local/paks-<phase>-before.tsv`
4. Claude prepares the test mods for this phase and lists which ones must be visible and which must
   be absent, with the character and outfit to check each one.

## In the game (you)

5. Start TEKKEN 8: from RemMods once it can launch the game (P1 and later), from Steam before that.
6. Open Practice mode, offline. Pick the characters and outfits from Claude's list.
7. Report for each mod: visible or not, and anything odd (missing textures, crashes, freezes).
8. Quit the game normally.

## After the game

9. Claude takes the "after" snapshot and compares:
   `scripts/paks-snapshot.sh > local/paks-<phase>-after.tsv`
   `diff -u local/paks-<phase>-before.tsv local/paks-<phase>-after.tsv`
   Only the changes the test intended may show up.
10. Claude restores the backup if the test changed anything that should not stay.

## Checks

- [ ] Enabled mods are visible in-game; disabled ones are absent.
- [ ] After quitting, the snapshot diff shows only the intended changes.
- [ ] From P1: RemMods shows "running" within 4 s of the game starting, and stops showing it after quitting.

## Per phase

- **P0**: baseline. Record the current snapshot and that the game starts and Practice works with the
  mods already installed. RemMods does not touch `Paks` yet, so before and after must be identical.
- **P1**: see "P1 steps" below.
- **P2**: disabled mods vanish in-game and return when re-enabled. Verify-early items (§11): are paks
  loaded loose in `Mods`, at depth 3, inside `Mods/.disabled_Test/`, from both `Mods` and `mods`;
  does `-x` stop them; toggle and trash a mod during a match; which mod wins with a shared pak
  basename, and with the same asset in different paks; does `~mods` beat `Mods`.
- **P3**: two profiles applied back and forth in-game.
- **P4**: one zip, one rar and one 7z mod, dropped onto RemMods, load in-game.
- **P5**: the same check with Steam started from the AppImage build; Steam gets a clean environment
  and working directory, and your gamescope launch options still work.

## P1 steps (read-only phase)

RemMods is the debug build, so it logs at Debug level. Every command works in fish. Nothing in `Paks`
may change in this phase. Captures are reviewed and redacted by hand before any becomes a fixture
(the repo is public).

1. Backup (above) and before snapshot: `scripts/paks-snapshot.sh > local/paks-p1-before.tsv`.
2. Start RemMods: it finds the game (source Steam, build 25221600).
3. The scan matches the disk:
   `python3 -I scripts/scan-expect.py local/paks-p1-before.tsv | LC_ALL=C sort > local/scan-expect.txt`,
   then the lines of the last published snapshot:
   `awk 'match($0, /scan-(snapshot|row|archive|unmanaged|leftover)\t/) { line = substr($0, RSTART); if (line ~ /^scan-snapshot/) n = 0; else out[n++] = line } END { for (j = 0; j < n; j++) print out[j] }' ~/.local/share/com.remirudev.remmods/logs/remmods.log | LC_ALL=C sort > local/scan-actual.txt`,
   then `diff -u local/scan-expect.txt local/scan-actual.txt`: no output. Runs records "identical".
4. Reveal one mod (Dolphin opens with it selected) and "Open Paks folder" (Dolphin opens Paks).
5. Just before clicking Launch, start
   `for i in (seq 60); python3 -I scripts/proc-capture.py p1-launch-$i > local/proc-p1-launch-$i.json; sleep 0.5; end`
   and click Launch in RemMods while it runs. Stopwatch: "running" within 4 s of the game window.
6. In offline Practice: `python3 -I scripts/proc-capture.py p1-running > local/proc-p1-running.json`.
   The rows RemMods shows as Enabled, including the loose `SamuraiHair_P` and `SamuraiOutfit_P`, are
   the ones visible in-game (PLAN §9 per-phase check).
7. Just before quitting, the same loop with `p1-quit-$i`. Quit: RemMods shows "not running" about 3 s
   after the game is gone (SPEC §4.12).
8. After snapshot identical to the before snapshot.
9. Update notice: with RemMods closed, set `game.lastSeenBuild.buildId` to `"1"` in
   `~/.config/com.remirudev.remmods/settings.json`. Start RemMods: the notice shows 1 to 25221600.
   Click "Got it", restart: no notice.
10. Original-app notice: `cp /usr/bin/sleep /tmp/ModManager.exe; /tmp/ModManager.exe 60`. The notice
    appears within about 2 s and clears after the process ends. Then `rm /tmp/ModManager.exe`.
11. KDE Wayland: the custom title bar drags, resizes and maximises; "Change folder…" opens the Plasma
    portal dialog attached to the window. Taskbar icon via `StartupWMClass` and the safe-graphics
    action: deferred to P5 (Appendix C); write it as such in Runs.
12. Runs records: the argv[0] drive letter, the shipping exe's comm, the NUL padding, the reaper's
    parent, the scan diff result and the stopwatch time.

## Runs

| Date | Phase | RemMods commit | Result | Notes |
|------|-------|----------------|--------|-------|
| 2026-11-01 | P0 | c6ef614 | pass | Baseline, game build 25221600, started from Steam. Paks: 264 files (112 in `Mods`), 19 folders. Kunimitsu's modded outfits visible in offline Practice. Before and after snapshots identical. `Mods` backed up to `~/RemMods-backups/2026-11-01/`. |
| 2026-11-01 | P1 | c5592bf | pass | Game build 25221600, launched from RemMods (`steam://rungameid/1778820` via xdg-open, gamescope launch options). In-app state showed RUNNING while playing and NOT RUNNING after quitting (owner; no stopwatch). Kunimitsu's modded outfits visible in offline Practice. Scan = disk: `scan-expect` vs the logged snapshot identical (27 lines: 17 rows incl. loose `SamuraiHair_P`/`SamuraiOutfit_P`, 10 archives, 0 unmanaged, 0 leftovers). Paks snapshot identical to P0. LIVE capture (09:55): argv[0] drive `S:`, shipping exe comm `GameThread`, reaper parent `gamescopereaper`. Launch/quit capture loops not run: `steam-install-live`/`steam-shutdown-live` fixtures still pending. Taskbar icon and safe-graphics action deferred to P5. |
