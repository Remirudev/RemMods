# Release runbook (owner)

The owner's steps around a release: the updater signing key, the AUR account, the GitHub settings and
cutting a version. The pipeline itself is `.github/workflows/release.yml` (docs/PLAN.md §10); it
runs when a `v*` tag is pushed. This file names secrets, never their values. Every command works in
fish.

Do the steps in this order. Steps 1 to 3 happen once, shortly before the first release; nothing in
them is needed earlier.

| Name | Kind (environment `release`) | Content | Used by job |
|------|------------------------------|---------|-------------|
| `TAURI_SIGNING_PRIVATE_KEY` | secret | the content of the updater private key file | `sign` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | secret | its password | `sign` |
| `AUR_SSH_PRIVATE_KEY` | secret | the content of the AUR SSH private key | `aur-push` |
| `AUR_KNOWN_HOSTS` | variable | `known_hosts` lines of `aur.archlinux.org` | `aur-push` |
| `AUR_COMMIT_EMAIL` | variable | the e-mail address of the AUR commits (public) | `aur-push` |

The jobs `sign`, `publish` and `aur-push` run in the environment `release` and wait for your approval.
Every other job runs without secrets.

## 1. Updater signing key

**Where**: run these steps in a terminal outside Claude Code: not the app's Terminal panel and not
while an agent session runs, because agents can read that panel and files in your home folder. The
key never touches the repository, the home folder or the clipboard.

1. Plug in an offline backup medium (a USB stick that is not synced anywhere) and point `key` at a
   file on it; replace `BACKUP` with the medium's name under `/run/media/$USER/`.
2. In the repository folder, make sure `CI` is not set: with `CI` set, the Tauri CLI silently creates
   a key **without a password**.

   ```fish
   set key /run/media/$USER/BACKUP/remmods-updater.key
   set -e CI
   npm run tauri -- signer generate -w $key
   ```

   Do not pass `-p`: type a strong password at the prompt. The command writes
   `remmods-updater.key` (private) and `remmods-updater.key.pub` (public) onto the medium.
3. Write the password down offline and make a second offline backup of the key file and the password
   (another medium, kept elsewhere). Losing either means no existing install can update itself again
   (step 7).
4. Never commit, `cat`, print or paste the private key file. Its only copy outside the backups will be
   the GitHub secret (step 3).
5. The public key goes into the app: replace `REMMODS-UPDATER-PUBKEY-NOT-SET` in
   `src-tauri/tauri.release.conf.json` (`plugins.updater.pubkey`) with the single line of
   `remmods-updater.key.pub`, exactly as it is, in a pull request. `python3 -I
   scripts/release-check.py` checks the structure; the release gate (`--strict`) refuses the
   placeholder.

If you prefer to keep the key on disk instead of a medium, use a folder that Claude Code's sandbox
denies reading, and add that rule before you create the key.

## 2. AUR account and key

1. Create an account on aur.archlinux.org (or use yours) for the package `remmods-bin`.
2. Create a key for CI only, without a passphrase (CI cannot type one):

   ```fish
   ssh-keygen -t ed25519 -N '' -C 'remmods-bin AUR (GitHub Actions)' -f ~/.ssh/aur_remmods
   ```

3. Add the content of `~/.ssh/aur_remmods.pub` to the account (My Account → SSH Public Key).
4. Fetch AUR's host keys and compare their fingerprints with the ones listed on the aur.archlinux.org
   home page ("SSH Public Key Fingerprints"); all must match:

   ```fish
   ssh-keyscan aur.archlinux.org > aur_known_hosts
   ssh-keygen -lf aur_known_hosts
   ```

   `aur_known_hosts` contains only public host keys; it becomes the `AUR_KNOWN_HOSTS` variable.

## 3. GitHub settings

All in the repository's Settings (as the owner):

1. **Environment** (Settings → Environments → New environment): name `release`.
   - Required reviewers: you.
   - "Prevent self-review": **off** (you push the tag yourself, so it would block every release).
   - Deployment branches and tags: "Selected branches and tags", add the **tag** rule `v*`.
2. **Secrets and variables of the environment** (from the repository folder; `gh` asks for nothing
   but the password):

   ```fish
   set key /run/media/$USER/BACKUP/remmods-updater.key
   gh secret set TAURI_SIGNING_PRIVATE_KEY --env release < $key
   gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --env release
   gh secret set AUR_SSH_PRIVATE_KEY --env release < ~/.ssh/aur_remmods
   gh variable set AUR_KNOWN_HOSTS --env release < aur_known_hosts
   gh variable set AUR_COMMIT_EMAIL --env release --body 'you@example.org'
   ```

   The first command stores the key file's content (not its path). The second prompts for the
   password. `AUR_COMMIT_EMAIL` appears in the public AUR git history, so choose the address
   accordingly. The `aur-push` job refuses to run when either variable is empty.
3. **Tag ruleset** (Settings → Rules → Rulesets → New tag ruleset): target tags matching `v*`;
   restrict creations, updates and deletions; bypass list: only you (Repository admin). Then only you
   can start a release.
4. **Immutable releases** (Settings → General → Releases): turn on release immutability, so a
   published release's assets and tag cannot be changed afterwards.
5. Delete the local private-key copies that are not backups: `rm ~/.ssh/aur_remmods` (CI holds the
   only working copy; make a new key if you ever need to push by hand), and any copy of the updater
   key outside the two offline backups. Then `rm aur_known_hosts`.

## 4. Cut a release (v1.0.0)

**Gate for v1.0.0**: WG-5 (Recycle Bin mover, P3b) and WG-2 (folder-busy toast with Cancel) are
closed, `docs/windows-gaps.md` has no open release-blocking entry, and the end-of-project test round
has passed (docs/checklists/ingame-linux.md). The `verify` job enforces the version, the public key
and the Windows gaps (`scripts/release-check.py --strict`); the test round is your call.

1. **Release PR** (a branch from main):

   ```fish
   npm version 1.0.0 --no-git-tag-version
   ```

   then set `version = "1.0.0"` under `[workspace.package]` in `Cargo.toml`, and:

   ```fish
   cargo update -w
   bash scripts/notices.sh
   python3 -I scripts/release-check.py --tag v1.0.0 --strict
   ```

   `npm version` updates `package.json` and `package-lock.json`; `cargo update -w` updates the
   workspace entries in `Cargo.lock`; `notices.sh` regenerates both notices files (it needs
   cargo-about 0.9.3: `cargo install --locked --features cli cargo-about@0.9.3`). The last command
   must pass before the PR is merged. CI must be green. The `artifacts` workflow runs on a PR only
   when packaging files change; otherwise start it by hand (Actions → artifacts → Run workflow, on the
   PR branch).
2. **Merge** the PR into main.
3. **Tag the merge commit** and push the tag:

   ```fish
   git switch main
   git pull --ff-only
   git log -1 --oneline
   ```

   The last line must be the release PR's merge commit. Then:

   ```fish
   set sha (git log -1 --format=%H)
   git tag v1.0.0 $sha
   git push origin v1.0.0
   ```

   `verify` refuses a tag whose commit is not on main or whose version differs.
4. **Approve `sign`** (Actions → the release run → Review deployments → `release`) once `verify`,
   both `build` legs and the `artifacts` jobs are green: `sign` waits for `artifacts`, which installs
   and smoke-tests the exact bundles, so the updater key only signs tested files. `draft` has created
   the draft release with the notes of `packaging/release-notes.md`.
5. After `sign`, the jobs `upload`, `latest-json` and `sums` run on their own. **Review the draft
   release**: the eight files (AppImage, deb, rpm, setup.exe, each with `.sig`), `latest.json` and
   `SHA256SUMS`, and the notes.
6. **Approve `publish`**: the release becomes public and "latest". From now on self-updating copies
   see it.
7. **Approve `aur-push`**: `aur-check` has already rendered `PKGBUILD` and `.SRCINFO` and run namcap;
   `aur-push` checks the hash against the published deb, renders both files again from the
   checked-out template and that deb, refuses any byte difference to `aur-check`'s files and pushes
   to `remmods-bin`.

If a job fails before `publish`, fix the cause and use "Re-run failed jobs"; `draft` reuses the
existing draft. If the code itself must change, delete the draft release and cut the next patch
version instead of moving the tag.

## 5. After a release

- Publish the first patch release soon after v1.0.0, then run the "First self-update" step of
  docs/checklists/ingame-linux.md (and the passive update of docs/checklists/windows-release.md when
  a Windows PC exists). Until then self-update is untested between real releases (WG-9); do not
  announce it before. `packaging/release-notes.md` therefore calls it new; once that step passed,
  its AppImage and setup.exe rows may say plainly that they update themselves.
- Submit the `RemMods_<version>_x64-setup.exe` of every release to Microsoft's file submission portal
  (as a software developer), so SmartScreen and Defender learn it (WG-10).
- Check the AUR package: `paru -S remmods-bin` and the AUR step of docs/checklists/ingame-linux.md.
- The AppImage bundles WebKitGTK from Ubuntu 24.04. Release a patch version about once a month, and
  after every Ubuntu 24.04 WebKitGTK security update, so AppImage users get the fixed libraries.

## 6. Code signing (later)

SignPath (PLAN §10) signs through `signCommand`, which runs inside `tauri build`. The `build` job
holds no secret by design (`--no-sign`, a separate `sign` job); adding SignPath means revisiting that
split (SPEC-P5 A26) before any secret reaches a build step.

## 7. Key loss or leak

Shipped apps trust only the public key they were built with.

- **Updater key lost** (or its password): existing installs can no longer update themselves. Create
  a new key (step 1), put its public key into `tauri.release.conf.json`, replace the two secrets and
  release. Say in the release notes that AppImage and Windows users must download this version by
  hand once.
- **Updater key leaked**: delete the secrets at once
  (`gh secret delete TAURI_SIGNING_PRIVATE_KEY --env release`, the same for
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`), then rotate as above, with a notice in the release notes. An
  attacker also needs to publish a `latest.json` in this repository's releases, so check the account
  and the tag ruleset too.
- **AUR key leaked**: remove the public key from the AUR account, make a new one (step 2) and replace
  `AUR_SSH_PRIVATE_KEY`.
