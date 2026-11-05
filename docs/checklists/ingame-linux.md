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
- **P3**: two profiles applied back and forth in-game; see "P3 steps" below.
- **P4**: one zip, one rar and one 7z mod, dropped onto RemMods, load in-game.
- **P5**: the same check with Steam started from the AppImage build; Steam gets a clean environment
  and working directory, and your gamescope launch options still work. See "P5 steps" below.
- **P3b**: two profiles back and forth again, one of them applied from the Mods header's quick-apply
  menu; bulk Move to Trash with a restore, bulk Delete permanently. See "P3b steps" below.

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
10. Other-manager notice: `cp /usr/bin/sleep /tmp/ModManager.exe; /tmp/ModManager.exe 60`. The notice
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
| 2026-11-01 | P2 | e5f296b | pass (basic) | Owner toggled a mod off in RemMods with the game closed: it was gone in offline Practice while the other enabled mods stayed; enabled again, it was back. Not run yet: the live toggle during a match and the PLAN §11 engine questions (loose/depth 3/dot folder/`mods` case variant, which mod wins, `~mods` vs `Mods`); proposed to move to the P2b in-game check. |

## P2 steps (toggle phase)

The first phase that renames mod files. Done with the owner: Claude prepares and switches the files,
you play offline Practice mode and report. Every command works in fish. Each step ends with a
"Result:" line to fill in. Test mods are copies of an installed visible mod (a costume, for example),
renamed.

1. Game closed. Before snapshot: `scripts/paks-snapshot.sh > local/paks-p2-before.tsv`. Back up
   `Mods`, `~mods` and `LogicMods` with `tar` into `local/backup-p2-<date>.tar` (read-only source).
   Result:
2. Prepare the test mods:
   - loose `Mods/P2Loose.pak`, `.ucas`, `.utoc`
   - depth 3: `Mods/P2Deep/a/b/P2Deep.pak` with its `.ucas` and `.utoc`
   - dot folder: `Mods/.disabled_P2Dot/…`
   - case variant: `mods/P2Case/…` beside `Mods`
   - a LogicMods mod renamed to `.disabled_X` whose paks are live

   Result:
3. Launch from RemMods and note which test mods are visible in-game (PLAN §11: loose, depth 3,
   dot folder in `Mods`, both `Mods` and `mods`, Logic dot folder). Quit the game.
   Result:
4. In RemMods, disable each test mod that was visible (its files become `-x`), then launch: each one
   must be gone. Enable them again: each must be back.
   Result:
5. During a match, toggle a mod off and on ("Apply for next launch?" appears). Note crashes or
   missing assets; this decides "ask" or "block" while the game runs (PLAN §4).
   Result:
6. Two mods with the same pak basename, and two mods with the same asset in different paks: which
   one wins? One in `~mods` and one in `Mods`: which wins? (load order, PLAN §11 and §12)
   Result:
7. Quit the game, restore from the backup, then
   `scripts/paks-snapshot.sh > local/paks-p2-after.tsv` and
   `diff -u local/paks-p2-before.tsv local/paks-p2-after.tsv`: no output after the restore.
   Result:
8. Claude records the answers in PLAN §11 and §7 after you agree.
   Result:

## P2b steps (content view)

The first phase that reads what is inside the mods (`.utoc` indexes). Same frame as P2: Claude
prepares and moves files, you play offline Practice mode and report. Every command works in fish.
Each step ends with a "Result:" line to fill in. Test mods are copies of an installed visible mod,
renamed; the copies are removed afterwards.

1. Game closed. Before snapshot: `scripts/paks-snapshot.sh > local/paks-p2b-before.tsv`. Back up
   `Mods`, `~mods` and `LogicMods` with `tar` into `local/backup-p2b-<date>.tar` (read-only source).
   Result:
2. Exit check A (file lists match an independent listing):
   `scripts/utoc-compare.sh "$HOME/.local/share/Steam/steamapps/common/TEKKEN 8/Polaris/Content/Paks/Mods"`
   shows no diff (expected 31 containers, 2790 file lines on the owner's install of 2026-11-01).
   Then open three mods in the RemMods drawer and compare each with
   `python3 -I scripts/utoc-list.py "<Paks>/Mods" | grep '<mod folder>'`.
   Result:
3. Exit check B (same asset, same files): copy one installed costume mod into `Mods/P2bA/` and
   `Mods/P2bB/`, renaming the pak trio to `P2bA_P.*` and `P2bB_P.*`. Identical copies are not a
   content conflict: neither row shows "Replaces same files", the details drawer counts the files
   as shared identical copies, and there is no pak-name conflict. Remove the copies.
   Result:
4. Exit check C (which one wins, same asset in different paks): two mods that ship the same asset
   with a visible difference (for example `Qipao finemod`'s `Qipao_P` and
   `20_(Option_Black_Color_Dress_2_P`), in separate folders `Mods/P2bW1` and `Mods/P2bW2`. Note which
   one is visible in the game. Swap the folder names and note again. Then put one in `~mods` and the
   other in `Mods` and note again. This settles the winner wording (SPEC-P2B Appendix A2).
   Result:
5. Same pak basename in two folders (P2 question): which one wins?
   Result:
6. P2 engine questions (PLAN §11), moved here from P2: a loose `Mods/P2Loose.*`; depth 3
   `Mods/P2Deep/a/b/`; a dot folder `Mods/.disabled_P2Dot/`; a lowercase `mods/P2Case/` beside
   `Mods`; a LogicMods folder renamed to `.disabled_X` whose paks are live. Which of them are visible?
   Result:
7. Live toggle: during a match toggle a mod off and on in RemMods ("Apply for next launch?"). Note
   crashes or missing assets; this decides "ask" or "block" (PLAN §4).
   Result:
8. Hints: with a mod RemMods labels "Alisa" (`mnt`) and one labelled "Kunimitsu" (`ker`), check that
   the costume appears on that character. Note any mismatch.
   Result:
9. Quit the game, restore from the backup, then
   `scripts/paks-snapshot.sh > local/paks-p2b-after.tsv` and
   `diff -u local/paks-p2b-before.tsv local/paks-p2b-after.tsv`: no output after the restore.
   Result:
10. Claude records the answers (load order, winner wording, engine answers, hint confirmations) in
    PLAN §11, §7 and §12 after you agree. The "Runs" table gets a P2b line.
    Result:

## P3 steps (manage phase)

The first phase that renames, trashes and deletes mod files and applies profiles. Backup first,
together: Claude prepares and runs the commands, you play offline Practice mode and report. Every
command works in fish. Each step ends with a "Result:" line to fill in. Never run a step without the
backup of step 1.

1. Game closed. Before snapshot: `scripts/paks-snapshot.sh > local/paks-p3-before.tsv`. Back up
   `Paks/Mods`, `Paks/~mods`, `Paks/LogicMods` and `~/.local/share/com.remirudev.remmods/` with
   `cp -a --reflink=auto` into `local/backup-p3/` (the sources stay read-only), and note the backup
   size (`du -sh local/backup-p3`). Confirm the system Trash is on the same filesystem as Paks:
   `stat -c %d "<Paks>" ~/.local/share/Trash` prints the same number twice (otherwise Trash uses a
   `.Trash-<uid>` folder at the mount root; note which).
   Result:
2. In RemMods: save profile **A** (additive) with mod set 1 enabled; enable mod set 2 instead and save
   profile **B** (exclusive). Note both sets.
   Result:
3. Apply A (check the changes listed in the diff dialog), launch, confirm set 1 is visible and set 2
   is not. Quit. Apply B, launch, confirm set 2 is visible and set 1 is gone. Quit. Apply A again,
   launch, confirm. After each apply an automatic snapshot appears in the profile list (the newest 10
   are kept). This is the P3 exit check "two profiles applied back and forth in-game".
   Result:
4. Rename one enabled mod (for example `NeonSkin` to `NeonSkin2`): profile A's entry follows (its
   diff shows no change), launch, the mod still loads.
   Result:
5. Case-only rename (`NeonSkin2` to `neonskin2`); check the folder name on disk with
   `ls "<Paks>/Mods"`.
   Result:
6. Move one mod to the Trash: it leaves the table; launch, it is gone in-game. Restore it from
   Dolphin's Trash; RemMods shows it again after the rescan; launch, it is back.
   Result:
7. Edit the metadata of one mod; open its `modinfo.json` and compare it with the expected bytes;
   RemMods' reading rules (PLAN §7) still parse it (`python3 -I scripts/scan-expect.py` if it
   applies).
   Result:
8. Import a copy of another mod manager's profiles (`python3 -I scripts/make-fixtures.py --profiles
   local/profiles-fixture`, then pick `profiles.json`): the imported profiles are Exclusive, and the
   diff of the empty Default lists every enabled mod as Disable. Cancel the apply.
   Result:
9. Repair tab: craft a leftover with `mkdir "<Paks>/Mods/_Test_tmp_5f3ccf7c5cff4a1e9b20d6e8c1a7b394"`
   holding a `-x` file (for example `touch .../Old.pak-x`); it shows in Repair. Move it to the Trash.
   Result:
10. Game closed. `scripts/paks-snapshot.sh > local/paks-p3-after.tsv`, then
    `diff -u local/paks-p3-before.tsv local/paks-p3-after.tsv` shows only the intended changes. Restore
    from `local/backup-p3/` if anything is unexpected, and diff again: no output after the restore.
    Result:

## P4 steps (install phase)

The first phase that unpacks untrusted archives and writes new mod folders. Run **at the very end of
the app, with the owner**: Claude prepares, you drop the files and play. Every command works in fish.
Nothing is written inside `Paks` except by RemMods. Each step ends with a "Result:" line to fill in.

1. Game closed. Before snapshot: `scripts/paks-snapshot.sh > local/paks-p4-before.tsv`. Back up `Mods`,
   `~mods`, `LogicMods` and `~/.local/share/com.remirudev.remmods/` to `~/RemMods-backups/<date>/`
   (`cp -a --reflink=auto`). Copy the 19 archives read-only to `local/archives-p4/`: only the
   extensions that exist, fish errors on an unmatched glob (for example
   `for f in ~/Downloads/*.zip; cp -n $f local/archives-p4/; end` once per extension that is there).
   Result:
2. `python3 -I scripts/make-fixtures.py smoke local/p4-fixture`, then `npm run tauri dev` (needs the
   display; sandbox off, owner-approved). Note the current game folder, then pick the fixture's
   (`local/p4-fixture/steamapps/common/TEKKEN 8/Polaris/Content/Paks` is its Paks folder).
   Result:
3. Drop all 19 archives at once. For each preview check the root, the name, the encoding, the
   variants, the fixes and the issues against `bsdtar -tf <archive>` or `7z l <archive>`, record the
   comparison in `local/p4-archives.md`, then install each one. Afterwards
   `find <fixture Paks>/{Mods,~mods,LogicMods} -type f | sort` matches the expected trees: wrapper
   folders stripped, `.pak` files live, files the mod shipped as `-x` still off. **Exit criterion 1.**
   Result:
4. In the fixture:
   - install one folder (the "Install folder" picker);
   - `python3 -I scripts/make-archive-fixtures.py local/p4-hostile`, then drop `traversal-backslash.zip`
     (blocked, nothing written), `multi.z01` and `encrypted.zip` (each shows its hint);
   - start a large install and cancel it: no `_install_tmp_*` folder is left and Repair is empty;
   - install the same mod twice and choose Replace: the old version is in the Trash.

   Result:
5. Switch back to the real game folder (the path noted in step 2). Game closed: drop one zip, one rar
   and one 7z mod from `local/archives-p4/`, chosen with the owner (not installed yet, or confirm
   Replace). Each lands enabled in the expected root.
   Result:
6. Launch from RemMods, offline Practice: the three mods show (Claude lists the character and outfit
   to check for each). Quit. **Exit criterion 2.**
   Result:
7. Trash the three test installs or keep them (owner's choice). Then
   `scripts/paks-snapshot.sh > local/paks-p4-after.tsv` and
   `diff -u local/paks-p4-before.tsv local/paks-p4-after.tsv`: only the intended changes, else restore
   from the backup and diff again.
   Result:

## P5 steps (release packaging)

The first phase that runs RemMods as a package: the AppImage from the pull request's `artifacts` run
(a CI build without the update channel). Claude never installs packages; the steps marked "(you)" are
yours. Every command works in fish and runs from the repository folder unless it says otherwise.
Nothing in `Paks` may change in this phase. Each step ends with a "Result:" line to fill in. Steps 1
to 9 and 12 are the P5 exit check; steps 10 and 11 follow later releases.

1. Game closed. Before snapshot: `scripts/paks-snapshot.sh > local/paks-p5-before.tsv`. Back up
   `Mods`, `~mods`, `LogicMods`, `~/.config/com.remirudev.remmods/` and
   `~/.local/share/com.remirudev.remmods/` to `~/RemMods-backups/<date>/` (`cp -a --reflink=auto`;
   the graphics steps below change `graphics.json`). Check that your environment sets no WebKit
   variable: `env | string match -r '^(WEBKIT_|__NV_)'` prints nothing.
   Result:
2. (you) Download the artifact `bundles-linux` of the PR's `artifacts` run (Actions → artifacts → the
   run → Artifacts) and put the AppImage into `~/Applications`:

   ```fish
   mkdir -p ~/Applications
   cd ~/Applications
   unzip ~/Downloads/bundles-linux.zip 'RemMods_*_amd64.AppImage'
   chmod +x RemMods_*_amd64.AppImage
   ```

   Result:
3. Start it (`~/Applications/RemMods_*_amd64.AppImage`). Settings → Updates shows "AppImage" and
   "This build has no update channel (development or test build)."; Diagnostics shows "Installed as"
   with the value "AppImage"; About → "Third-party notices" shows two tabs. The log agrees:
   `string match '*install kind*' < ~/.local/share/com.remirudev.remmods/logs/remmods.log | tail -n 1`
   ends with `install kind AppImage, updater configured false`. Close RemMods.
   Result:
4. (you) Desktop integration and taskbar icon: let Gear Lever integrate the AppImage, or install its
   own desktop file and icons for your user:

   ```fish
   cd ~/Applications
   set app (path resolve RemMods_*_amd64.AppImage)
   $app --appimage-extract 'usr/share/applications/*'
   $app --appimage-extract 'usr/share/icons/*'
   mkdir -p ~/.local/share/applications ~/.local/share/icons
   sed "s|^Exec=remmods|Exec=$app|" squashfs-root/usr/share/applications/RemMods.desktop > ~/.local/share/applications/RemMods.desktop
   cp -r squashfs-root/usr/share/icons/hicolor ~/.local/share/icons/
   rm -r squashfs-root
   ```

   Start RemMods from the application menu: the taskbar shows the RemMods icon (not a generic one),
   and a pinned launcher and the running window are one taskbar entry.
   Result:
5. Window class (records the UNVERIFIED item of SPEC-P5 §2.3): run
   `qdbus6 org.kde.KWin /KWin org.kde.KWin.queryWindowInfo` and click the RemMods window (or use the
   window menu → More Actions → Configure Special Window Settings → Detect Window Properties). It
   shows `resourceClass: remmods`. Note the `desktopFile` value too.
   Result:
6. Safe graphics action: close RemMods, right-click its menu entry and choose "Start in safe graphics
   mode". Settings → Graphics shows "This session runs in safe graphics mode (started with
   --safe-graphics)." and the "Always use software rendering" button (do not click it). The log shows
   the mode and the variable RemMods set:
   `string match '*graphics: stored*' < ~/.local/share/com.remirudev.remmods/logs/remmods.log | tail -n 1`
   contains `effective Software from Flag` and `WEBKIT_DISABLE_DMABUF_RENDERER`. (RemMods sets the
   variable in its own process, so `/proc/<pid>/environ`, which shows only the environment a
   process started with, does not list it.) Close RemMods.
   Result:
7. Crash marker: Settings → Graphics is Automatic. Close RemMods, start it from the menu and, within
   5 s, `pkill -KILL -x remmods`. Start it again: the banner "Graphics mode changed" says "RemMods did
   not start correctly last time, so it switched from Automatic to GPU, shared memory.", and Settings →
   Graphics shows "GPU, shared memory". Click "Switch back": Settings → Graphics shows Automatic and
   `string match '*"mode"*' < ~/.config/com.remirudev.remmods/graphics.json` shows `"auto"`. Wait 15 s:
   `path filter ~/.config/com.remirudev.remmods/graphics-pending` prints nothing (the marker is
   cleared after a good start).
   Result:
8. Launcher action while RemMods runs: start RemMods from the menu and note `pgrep -n -x remmods`.
   Choose "Start in safe graphics mode" from the menu while its window is open: RemMods closes and
   comes back with another PID, Settings → Graphics shows the safe graphics note, and the log has
   `restarting in safe graphics mode for a second instance's --safe-graphics`. The new process got the
   host environment, without RemMods' variables:
   `string split0 < /proc/(pgrep -n -x remmods)/environ | string match -r '^(WEBKIT_|__NV_)'` prints
   nothing. Close RemMods.
   Result:
9. Steam from the AppImage: Steam closed (`pgrep -x steam` prints nothing). Start RemMods from the
   menu and click Launch. Once the game runs, each of these prints nothing:

   ```fish
   string split0 < /proc/(pgrep -n -x steam)/environ | string match -r '^(APPDIR|APPIMAGE|OWD|ARGV0|GTK_|GDK_PIXBUF|GIO_MODULE|GSETTINGS_SCHEMA_DIR|GI_TYPELIB_PATH|PYTHONHOME|WEBKIT_|__NV_)'
   string split0 < /proc/(pgrep -n -x steam)/environ | string match '*.mount_*'
   readlink /proc/(pgrep -n -x steam)/cwd | string match '*.mount_*'
   ```

   (The second covers every path list, such as `PATH` and `XDG_DATA_DIRS`. Steam's launcher changes
   into its own folder, so its working directory is not `$HOME`; the `$HOME` working directory of
   RemMods' child processes is tested in CI.) The game starts with your gamescope launch options as
   before. In offline Practice, the enabled mods are visible (PLAN §9 per-phase check). Quit the game.
   Close RemMods and leave Steam running: within a few seconds
   `findmnt -rn -o TARGET | string match '*/.mount_RemMod*'` prints nothing (Steam inherited none of
   the AppImage runtime's file descriptors, so it does not keep the mount alive).
   Result:
10. AUR (you, after v1.0.0 is published): remove the user desktop file of step 4 first, it would hide
    the package's (`rm ~/.local/share/applications/RemMods.desktop`), then `paru -S remmods-bin`.
    Start RemMods from the menu: Settings → Updates shows "AUR package (remmods-bin)" and "Installed
    from the AUR: update with your AUR helper, for example paru -Syu."; the taskbar icon and the
    safe-graphics action work as in steps 4 and 6.
    Result:
11. First self-update (after the first patch release, before self-update is announced): the previous
    release's AppImage in `~/Applications` (a release build, so Settings → Updates offers "Check for
    updates"). Check, then "Install and restart"; the confirmation says "RemMods will close to install
    the update." RemMods restarts from the same file (it keeps its old file name) and Settings →
    Updates shows the new version. The relaunched process has none of RemMods' variables (SPEC-P5
    A8): `string split0 < /proc/(pgrep -n -x remmods)/environ | string match -r '^(WEBKIT_|__NV_)'`
    prints nothing. No rollback copy is left in any of the three places the updater may put it (the
    temp folder, the cache folder, the AppImage's own folder):
    `find /tmp $TMPDIR ~/.cache $XDG_CACHE_HOME ~/Applications -maxdepth 1 -name 'tauri_current_app*' 2>/dev/null`
    prints nothing (an unset variable expands to nothing in fish).
    Then once more with an older AppImage started as `./RemMods_<old>_amd64.AppImage
    --appimage-extract-and-run`: "Install and restart" brings RemMods back (extracted again, no FUSE
    needed), and the relaunched process did not get the setting as a variable:
    `string split0 < /proc/(pgrep -n -x remmods)/environ | string match 'APPIMAGE_EXTRACT_AND_RUN*'`
    prints nothing. In that session the desktop file's "Start in safe graphics mode" action only
    focuses the window (the log says why).
    Result:
12. Game closed. `scripts/paks-snapshot.sh > local/paks-p5-after.tsv`, then
    `diff -u local/paks-p5-before.tsv local/paks-p5-after.tsv`: no output. Restore
    `~/.config/com.remirudev.remmods/` from the backup if the graphics steps left a mode you do not
    want. The "Runs" table gets a P5 line.
    Result:

## P3b steps (bulk actions and quick-apply)

The first phase that trashes and deletes several mods in one action and applies profiles from the
Mods header. Backup first, together: Claude prepares and runs the commands, you play offline Practice
mode and report. Every command works in fish. Each step ends with a "Result:" line to fill in. Never
run a step without the backup of step 1.

1. Game closed. Before snapshot: `scripts/paks-snapshot.sh > local/paks-p3b-before.tsv`. Back up
   `Paks/Mods`, `Paks/~mods`, `Paks/LogicMods` and `~/.local/share/com.remirudev.remmods/` with
   `cp -a --reflink=auto` into `local/backup-p3b/` (the sources stay read-only), and note the backup
   size (`du -sh local/backup-p3b`).
   Result:
2. Profiles **A** (additive, mod set 1) and **B** (exclusive, mod set 2) as in the P3 steps (save them
   again if they are gone). Apply A from the **Mods header's "Apply profile" menu** (check the changes
   listed in the diff dialog), launch, confirm set 1 is visible and set 2 is not. Quit. Apply B from the
   same menu, launch, confirm set 2 is visible and set 1 is gone. Quit. Apply A from the Profiles panel,
   launch, confirm. This is the P3b exit check "two profiles back and forth in-game, quick-apply
   included".
   Result:
3. Copy three throwaway mods into `Mods` as `RemModsTest1`, `RemModsTest2` and `RemModsTest3`
   (`cp -a --reflink=auto` of a folder mod from `local/backup-p3b/`), then select the three in RemMods
   and choose "Disable" so the game never loads a copy twice. Select `RemModsTest1` plus two real mods
   and choose "Move to Trash (3)": the confirmation lists the three; after it, the message reads
   "Moved 3 mods to the Trash." (the list "Not everything went as planned" opens only if a row
   failed). Launch: the two real mods are
   gone in-game. Quit. Restore only the two real ones from Dolphin's Trash (`RemModsTest1` stays
   there; empty it by hand after the run); RemMods shows them again after the rescan; launch, they are
   back.
   Result:
4. Select exactly `RemModsTest2` and `RemModsTest3` by name and choose "Delete permanently… (2)",
   then "Continue": the final confirmation "Delete these mods forever?" lists both with their sizes
   and the total (its button stays disabled for a moment); choose "Delete permanently (2)". Both are gone and not in the Trash, and no hidden `_…_tmp_…`
   folder is left (`ls -a "<Paks>/Mods"`).
   Result:
5. Bulk with a matrix refusal: select one loose `.pak` row and one folder row. The bar reads "Delete
   permanently… (1)" and the first confirmation says 1 selected mod is skipped (linked, loose or
   archive). Cancel: nothing changes.
   Result:
6. Game closed. `scripts/paks-snapshot.sh > local/paks-p3b-after.tsv`, then
   `diff -u local/paks-p3b-before.tsv local/paks-p3b-after.tsv` shows only the intended changes (the
   enable states left by step 2's last apply, folder mtimes); no `RemModsTest*` entry and no `_…_tmp_…`
   folder. Re-apply the starting states, or restore from `local/backup-p3b/`, if anything else
   differs, and diff again. The "Runs" table gets a P3b line.
   Result:

## Groups steps (one dashboard)

Groups replace the Profiles panel: a sidebar left of the mod list with one switch per group. Backup
first, together: Claude prepares and runs the commands, you play offline Practice mode and report.
Every command works in fish. Each step ends with a "Result:" line to fill in. Never run a step
without the backup of step 1.

1. Game closed. Before snapshot: `scripts/paks-snapshot.sh > local/paks-groups-before.tsv`. Back up
   `Paks/Mods`, `Paks/~mods`, `Paks/LogicMods` and `~/.local/share/com.remirudev.remmods/` with
   `cp -a --reflink=auto` into `local/backup-groups/` (the sources stay read-only), and note the
   backup size (`du -sh local/backup-groups`).
   Result:
2. In the sidebar choose "+" (New group): name it **Set 1**, set mod set 1 to On and mod set 2 to Off,
   create it. Make **Set 2** the other way round. Groups that switch on two mods replacing the same
   files show a warning in the dialog and a ⚠ in the sidebar; note any you see.
   Result:
3. Switch **Set 1** on: the change list names the mods it turns on and off; Apply. Launch, confirm set
   1 is visible and set 2 is not. Quit.
   Result:
4. Switch **Set 2** on; Apply. Set 1's switch goes off by itself, Set 2's is on. Launch, confirm set 2
   is visible and set 1 is gone. Quit.
   Result:
5. Switch **Set 2** off: the change list only turns off set 2's mods; Apply. Launch, confirm set 2 is
   gone and nothing else changed. Quit. Then choose **Undo** in the toast (or Automatic snapshots… in
   the groups menu, Restore… on the newest one): set 2 is back; launch, confirm. Quit.
   Result:
6. Click **Set 1**'s name: the mod list shows only its mods and a "Group: Set 1" chip; its × shows
   all mods again.
   Result:
7. Game closed. `scripts/paks-snapshot.sh > local/paks-groups-after.tsv`, then
   `diff -u local/paks-groups-before.tsv local/paks-groups-after.tsv` shows only the intended changes
   (the enable states left by step 5, folder mtimes). Re-apply the starting states, or restore from
   `local/backup-groups/`, if anything else differs, and diff again. The "Runs" table gets a Groups
   line.
   Result:

## Replaces steps (details drawer)

Only reading: the app reads the game and mods, and nothing in Paks changes. No backup needed, and the
game stays closed except to look at its menu. Each step ends with a "Result:" line to fill in.

1. Open RemMods with the game closed. In Settings, "Item pictures and names" shows pictures and names
   after a few seconds; "Check now" changes nothing the second time.
   Result:
2. Pick five mods with several slots (for example two outfits with hair, one accessory mod, one mod
   that adds new items, one hair mod). Click each one and note its Replaces lines for your main
   character: name, slot, left picture, right picture.
   Result:
3. Start the game, open Customization for that character and find the same items. The name matches
   the left and the mod's look matches the right. Any line that does not match goes here.
   Result:
4. Close the game. Open a mod, click another character chip: both pictures of every line switch. Open
   another mod: the same character stays chosen.
   Result:
5. "N more pieces without their own picture" unfolds; the pieces listed are ones the mod hides or
   supports, not its outfit.
   Result:
6. Choose a picture for a mod with "Choose a picture…", then drag another image file onto the window
   while the details are open: the overlay names the mod and the new picture replaces the first.
   Drag an image with no details open: a notice asks which mod it is for, nothing installs. Rename
   that mod: its picture stays. "Remove picture" brings the mod's own look back.
   Result:

## Mod list steps (pictures and grid)

Only looking: nothing in Paks changes and the game stays closed. Each step ends with a "Result:" line.

1. Scroll the table from top to bottom with the mouse wheel, then quickly with the scrollbar. It stays
   smooth in the real window (WebKitGTK); pictures appear within a moment and stay when you scroll
   back.
   Result:
2. Switch to the grid (top right of the list). Scroll the same way. Close and reopen RemMods: the grid
   is still chosen.
   Result:
3. In the grid: click into the list, then use the arrow keys, Home, End, Space (switches the mod) and
   Enter (opens it). Ctrl+click and Shift+click select several, and the action bar works on them.
   Result:
4. Pick a character in the Characters filter: only their mods and mods for everyone stay, and the
   pictures show that character.
   Result:
5. In Settings turn "Hide pictures" on: the list and the details show empty tiles. Turn it off again.
   Result:

## Skin sets steps (saves)

This is the first test that writes in the game's save folder, and it only touches the five skin files.
Backup first, together: Claude prepares and runs the commands, you play offline Practice mode and
report. Every command works in fish. Each step ends with a "Result:" line. Never run a step without the
backup of step 1. RemMods must be the build with skin sets, and Steam Cloud stays as it is.

If the game ever says the character customization data is corrupted: choose **Return to title**, never
"Delete saved data", quit, and stop here. Claude restores the backup. If Steam shows a cloud conflict
window, stop and ask before choosing anything.

1. Game closed. Claude copies the whole `SaveGames` folder to `~/RemMods-backups/<date>-skinsets/` and
   to `/mnt/SSD/RemMods-backups/<date>-skinsets/`, then compares hashes of both copies with the
   originals. Note your Fight Money, one item you own and the number of replays.
   Result:
2. In RemMods the sidebar has "Skin sets" under Groups. Choose **Save my skins** and name the set
   **Set A**. It shows "In the game". Claude compares the set's five files with the game's: identical.
   `global1.sav` has not changed.
   Result:
3. Start the game, change one slot of a character so you can tell it apart (note which), and save it.
   Quit. Choose "+" in Skin sets and name the set **Set B**: B is now in the game.
   Result:
4. Choose **Switch** on Set A. The notice says your skins switched, and A shows "In the game". Claude
   compares: the game's five files equal Set A, every other save file is unchanged.
   Result:
5. Start the game: the slot from step 3 is back as it was, with no warning. Fight Money, the item,
   replays and ghosts are unchanged. Quit. Start it a second time and check again. Quit.
   Result:
6. Switch to Set B, start the game: the changed slot from step 3 is there, the same things are
   unchanged. Quit. Steam shows no conflict window, and Cloud syncs normally.
   Result:
7. Start the game and press **Switch** in RemMods while it runs: it refuses and nothing changes. Quit.
   Result:
8. In the game press F12 on a slot page, quit, open Set B in RemMods and use **Add screenshots…**.
   The picker opens at Steam's screenshot folder, the picture shows in the set, and notes are kept
   after a restart of RemMods. Send Claude one screenshot exactly as Steam saved it: it decides whether
   cutting pages into single slots can be built.
   Result:
9. Start the game, change one slot (note which), quit. Open **Set B** in RemMods and choose **Put back
   in the game**, then confirm. Start the game: the slot is as it was before this step. Quit.
   Result:
10. Choose **Automatic copies** (the clock next to "+"). The newest line is "Skins from before a
    switch", with the time of step 9. Choose **Keep as a set** and name it **Rescued**: it appears in
    the list and B stays in the game. Switch to **Rescued**, start the game: the slot from step 9 is
    back. Quit. Switch to Set B and delete **Rescued**.
    Result:
11. Open a group, link **Set A** in its dialog and save. Switch the group on: mods and skins change, and
    the toast says so. Choose **Undo**: mods and skins go back. Start the game once to confirm. Quit.
    Result:
12. Claude checks that `skin-sets/<id>/backups/` and each `skin-sets/<id>/replaced/<set>/` hold at most
    three copies, and that every save file outside the five still matches the backup, apart from what
    the game wrote itself (`global1.sav`, replays). Switch back to the set you want to keep. The "Runs"
    table gets a Skin sets line.
    Result:
