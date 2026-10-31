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
- **P1**: RemMods finds the game folder; the running state flips in-game; the snapshot is unchanged
  (read-only phase). Record the live `/proc` values of the running game (argv[0] form, comm, reaper
  line) for the test fixtures (§11).
- **P2**: disabled mods vanish in-game and return when re-enabled. Verify-early items (§11): are paks
  loaded loose in `Mods`, at depth 3, inside `Mods/.disabled_Test/`, from both `Mods` and `mods`;
  does `-x` stop them; toggle and trash a mod during a match; which mod wins with a shared pak
  basename, and with the same asset in different paks; does `~mods` beat `Mods`.
- **P3**: two profiles applied back and forth in-game.
- **P4**: one zip, one rar and one 7z mod, dropped onto RemMods, load in-game.
- **P5**: the same check with Steam started from the AppImage build; Steam gets a clean environment
  and working directory, and your gamescope launch options still work.

## Runs

| Date | Phase | RemMods commit | Result | Notes |
|------|-------|----------------|--------|-------|
| 2026-11-01 | P0 | c6ef614 | pass | Baseline, game build 25221600, started from Steam. Paks: 264 files (112 in `Mods`), 19 folders. Kunimitsu's modded outfits visible in offline Practice. Before and after snapshots identical. `Mods` backed up to `~/RemMods-backups/2026-11-01/`. |
