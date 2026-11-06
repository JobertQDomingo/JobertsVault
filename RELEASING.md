# Shipping Jobert's Vault

How a change gets from your editor to every installed copy of Jobert's Vault. Read it top to bottom the first few times, then use the headings as a checklist. [ARCHITECTURE §11.1](docs/ARCHITECTURE.md) explains the design.

## The idea

- `main` is always ready to ship. Changes reach it only through a pull request (PR) with green CI. The **Protect Main** ruleset enforces this.
- `package.json` holds the app's only version number. `src-tauri/tauri.conf.json` reads it (`"version": "../package.json"`).
- **Merging a change releases nothing.** Merging a **version bump** releases that version: GitHub builds and signs it, then publishes it, and every installed copy with automatic updates on installs it at its next start.
- `0.0.0` never releases. It's the version until the first release, `0.1.0` (ROADMAP 1.11).
- Never push a tag by hand. Publishing a release creates its tag.

```
a change:   branch → PR → CI ✓ → squash-merge → main                   (nothing released)
a release:  release/vX.Y.Z → PR → CI ✓ → merge → Release workflow → installed copies update
```

## Loop 1: every change (feature, fix, docs, chore)

```powershell
git switch main
git pull
git switch -c feat/short-name        # or fix/…, docs/…, chore/…, ci/…
# work and commit, as many commits as you like
git push -u origin feat/short-name
gh pr create --web
gh pr checks --watch                 # CI: the check job, about 5 minutes
```

When CI is green, click **Squash and merge**, which also deletes the branch on GitHub. Then run:

```powershell
git switch main
git pull
git branch -D feat/short-name        # capital D: git can't tell a squashed branch was merged
```

- **Your PR title becomes a line in the next release's notes**, for example `feat: add transfer validation (TXN-1)`. Write it for the person reading them, with a type prefix and the requirement IDs.
- **The next release ships everything on `main`**, so only merge what's ready.
- **CI runs every check from [CLAUDE.md](CLAUDE.md)**, plus the bindings check. Run them locally before you push to save a round trip.
- **If CI is red**, open the failing step (PR → Checks, or `gh run view --log-failed`), fix it, commit and push. CI runs again, and a newer push cancels the older run. Never merge on red.
- **If `main` rejects a direct push**, the **Protect Main** ruleset is working. Use a branch and a PR.

## Loop 2: a release

### Pick the version number

| Since the last release | Bump | Example |
|---|---|---|
| Only `fix`, `docs`, `chore`, `ci` | patch | 0.1.0 → 0.1.1 |
| At least one `feat` | minor | 0.1.1 → 0.2.0 |
| A breaking change, or you call it "1.0" | major | 0.x → 1.0.0 |

The first release is `0.1.0`. To see what would ship, run `git log v0.1.0..HEAD --oneline`, replacing `v0.1.0` with the newest tag (`git describe --tags --abbrev=0` prints it). A version can only go up: the Release workflow fails on one lower than the newest release.

### Automate it with `/release`

The repo includes a Claude Code skill, [`.claude/skills/release/SKILL.md`](.claude/skills/release/SKILL.md), that runs the whole release for you and **always asks before it publishes**.

**Run it:**

1. Open Claude Code in this repo, either the terminal (`claude`) or the VS Code panel. Start a new session after pulling a new version of the skill, because skills load when a session starts.
2. Type one of:

   | You type | It releases |
   |---|---|
   | `/release` | It suggests patch or minor from the commits, and you confirm |
   | `/release patch` | The next patch, e.g. 0.1.0 → 0.1.1 |
   | `/release minor` | The next minor, e.g. 0.1.1 → 0.2.0 |
   | `/release 0.1.0` | Exactly that version |

   Typing `/` shows the list of commands. `release` is among them as *(project)*.

**What it does, and where it waits for you:**

| # | It… | You |
|---|---|---|
| 1 | Checks your working tree is clean, `main` is up to date, and CI on `main` is green. Then it lists what ships. | Commit or stash anything it says is uncommitted |
| 2 | Proposes the version number | **Confirm**, or give a different one |
| 3 | Makes `release/vX.Y.Z`, bumps the version, commits, pushes and opens the PR | Approve the commands if Claude Code asks |
| 4 | Waits for CI on the PR | Nothing |
| 5 | **Asks: "Merging publishes vX.Y.Z… Merge now?"** | **Yes** to publish, **no** to stop. The PR stays open, and you can merge it yourself later. |
| 6 | Squash-merges, then watches the Release run (20–30 minutes) | Nothing |
| 7 | Checks the release isn't a draft, all 4 files are there and `latest.json` shows the new version. Then it gives you the link. | Check it in Settings if you like |

**Safety:**
- It only runs when you type `/release`. Claude never starts a release on its own (`disable-model-invocation: true`).
- It never merges without a yes to step 5, even if you said "go ahead" earlier.
- It never creates tags.
- If anything fails, it stops and shows you the error. A failed release stays a hidden draft, so no installed copy sees it.

**Changing how it works:** edit `.claude/skills/release/SKILL.md`. It's plain instructions, not code. Ship the edit like any change (Loop 1), then start a new Claude Code session.

### By hand

The same steps, if you'd rather type them yourself. An assistant never does this: it uses `/release`, which asks first ([CLAUDE.md](CLAUDE.md) rule 10).

1. Check `main` is green with `gh run list --workflow ci.yml --branch main --limit 1`.
2. Branch, bump and open the PR:
   ```powershell
   git switch main
   git pull
   git switch -c release/v0.1.1
   npm version 0.1.1 --no-git-tag-version   # never drop the flag: tags are the workflow's job
   git diff --stat                           # only package.json and package-lock.json
   git commit -am "chore: release v0.1.1"
   git push -u origin release/v0.1.1
   gh pr create --web
   ```
   Title: `chore: release v0.1.1`. Body: `Releases **v0.1.1**`, followed by the PRs that ship.
3. Run `gh pr checks --watch`. When it's green, click **Squash and merge**. **This is the moment it goes to every installed copy.**
4. Watch it with `gh run list --workflow release.yml --limit 1`, then `gh run watch`. A release takes 20–30 minutes. You can also follow it at **Actions → Release**.
5. Check it's live:
   ```powershell
   gh release view v0.1.1
   curl.exe -sL https://github.com/JobertQDomingo/JobertsVault/releases/latest/download/latest.json
   ```
   It shouldn't be a draft, it should have 4 files, and `latest.json` should show the new version. GitHub may serve the old file for a minute.
6. Clean up with `git switch main`, `git pull`, then `git branch -D release/v0.1.1`.

### What the Release workflow does

1. **`version`** (seconds, on Linux): reads `package.json`. It stops if that version's tag already exists, which is why ordinary merges release nothing. It also stops at `0.0.0`, and when run by hand from a branch other than `main`. It fails if the version is lower than the newest release.
2. **`checks`**: runs `ci.yml` itself, so a release passes exactly the checks every PR does.
3. **`release`** (on Windows):
   - Removes any draft left by a failed run.
   - Builds and signs the installer into a **hidden draft**, named `JobertsVault_X.Y.Z_x64-setup.exe` (plain ASCII, because the curly apostrophe in `Jobert’s Vault` doesn't survive as an asset name), then uploads `JobertsVault-setup.exe` for a direct download.
   - Checks that all 4 files are there: `latest.json`, the installer, its `.sig` and `JobertsVault-setup.exe`.
   - Writes the notes (the install text plus the PR titles) and publishes. Publishing creates the tag.

Only the `release` job can write to the repository. The write token goes only to `tauri-action` (it builds and uploads) and the `gh` steps, and the signing key only to `tauri-action`. Checkout doesn't keep the token, so `npm ci` can't read it.

### How installed copies get it

- **Automatic** (the default): at its next start, the app checks for 3 seconds at most. "Checking for updates…" appears only if the check takes longer than 150 ms. Then "Updating Jobert's Vault to X…" appears, and the app reopens on X with all its data.
- **Automatic off** (Settings → General): the app never connects by itself. **Check for updates** says "You're up to date." or "X is out." with an **Update now** button.
- **Offline, or GitHub unreachable:** the app starts normally and shows no error.
- **A refused or failed update** says why, adds "Nothing was changed.", and the app carries on as before.
- **If the database can't open**, the error screen also has **Check for updates**, so a newer version can fix it.
- **A migration in the update** runs at the next start, after the database is copied to `%APPDATA%\com.jobertsvault.app\backups\pre-migration\` (BAK-5).

## When something goes wrong

| Situation | What to do |
|---|---|
| CI is red on a PR | Open the failing step (PR → Checks, or `gh run view --log-failed`), fix it, commit and push. CI runs again. Never merge on red. |
| The Release run is red | Nobody got anything: it's a hidden draft. Fix it in a normal PR, merge, then run `gh workflow run release.yml` to finish the release. |
| "X is missing, so vX.Y.Z stays a draft" | An asset has the wrong name, often from a changed `assetNamePattern` or product name. Fix the workflow in a PR, merge, then `gh workflow run release.yml`. |
| "X is lower than the newest release" | The version bump went down. Open a new release PR with a higher version. |
| A published version has a bug | Fix it in a PR, then release a patch. Never delete or move a published release or tag: installed copies only move forward. |
| An update brought a migration that broke the data | The copy from before the migration is in `%APPDATA%\com.jobertsvault.app\backups\pre-migration\`. Close the app, delete `jobertsvault.db-wal` and `jobertsvault.db-shm` if they're there (left over, they would be replayed onto the copy), then copy the backup over `jobertsvault.db` by hand (Restore arrives in Phase 2). Then release a fix, because the broken version would migrate it again. |
| You merged changes but forgot the bump | Nothing was released. Open a release PR. |
| "Node.js 20 is deprecated" warnings in Actions | Harmless. Move the named action (e.g. `actions/checkout@v7`) to its newest major version in a `ci/` PR. `tauri-action` is pinned to a commit: move it on purpose, after reading its changes. |
| `main` rejects a direct push | The **Protect Main** ruleset is working. Use a branch and a PR. |
| The signing key or its password is lost | See below. |

## The signing key

- **Private key:** `~/.tauri/jobertsvault.key`, protected by a password and backed up off this PC. GitHub holds copies in the secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. `gh secret list` shows their names; GitHub never shows the values.
- **Public key:** `plugins.updater.pubkey` in `src-tauri/tauri.conf.json`. It's safe to share.
- **What the signature protects:** installed copies refuse an update whose signature doesn't match, or one signed for a different version than `latest.json` announces (`requireSignedVersion`).
- **Never** run `tauri signer generate -f` on that path. It would replace the key and break updates for every installed copy.
- **Never** commit it, paste it into a chat or print it in a log. `*.key` is in `.gitignore`.
- **Local builds:** with `createUpdaterArtifacts` on, a local `npm run tauri build` needs both variables set. `npm run tauri dev` doesn't. The rehearsal below uses its own throwaway key instead.
- **If the key or its password is lost:** no installed copy can accept a new update. Make a new key, put its `.pub` in `tauri.conf.json`, replace both secrets and release. Every installed copy must then be reinstalled by hand once, from `JobertsVault-setup.exe`. The data stays, because it's in `%APPDATA%`.
- **If the key leaks:** the same steps, straight away. Someone with the key and access to this repository could ship a malicious update ([ADR 0010](docs/decisions/0010-automatic-updates.md)).

## Rehearse an update on your PC

Do this whenever you change the updater: `src-tauri/src/updater.rs`, the update commands, the startup flow, the `plugins.updater` config or the plugin version. It builds two signed versions of a separate app, "Jobert’s Vault Rehearsal", that updates from a server on your PC. Nothing is published.

- **It can't touch your real data.** It has its own identifier, so its data lives in `%APPDATA%\com.jobertsvault.app.rehearsal`.
- **It uses its own throwaway key**, never `jobertsvault.key`. The rehearsal config replaces the public key with the throwaway one, so the same code checks the same kind of signature.
- **It builds from a separate worktree** at the commit you're testing, so a running `npm run tauri dev` never sees the rehearsal's migration. Commit your change first.
- **The second version brings a migration** (an empty `rehearsal_marker` table), to test BAK-5 during an update.

### 1. Set up

Run this from the repo folder. `$R` is the rehearsal folder, `$W` the worktree.

```powershell
$R = "$HOME\jv-rehearsal"; $W = "C:\Projects\jv-rehearsal-src"
New-Item -ItemType Directory -Force $R | Out-Null
npx tauri signer generate --ci -p rehearsal -w "$R\rehearsal.key"   # throwaway; its password is "rehearsal"
git worktree add --detach $W HEAD   # the commit you're testing
cmd /c mklink /J "$W\node_modules" "$PWD\node_modules"
```

Write `$R\0.9.0.json` as below, with the contents of `$R\rehearsal.key.pub` as `pubkey`, and a copy named `0.9.1.json` with `"version": "0.9.1"`.

```json
{
  "version": "0.9.0",
  "productName": "Jobert’s Vault Rehearsal",
  "mainBinaryName": "jobertsvault-rehearsal",
  "identifier": "com.jobertsvault.app.rehearsal",
  "plugins": {
    "updater": {
      "pubkey": "<contents of rehearsal.key.pub>",
      "endpoints": ["http://localhost:8765/latest.json"],
      "dangerousInsecureTransportProtocol": true
    }
  }
}
```

### 2. Build both versions, signed with the throwaway key

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content "$R\rehearsal.key" -Raw).Trim()
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "rehearsal"   # PowerShell deletes a variable set to ""
$out = "$W\src-tauri\target\release\bundle\nsis"
Push-Location $W
npx tauri build --config "$R\0.9.0.json"
# 0.9.1 brings a migration: add M::up("CREATE TABLE rehearsal_marker (id INTEGER PRIMARY KEY) STRICT;")
# after the 0001 entry in the vec! in src-tauri\src\db\migrations.rs, then:
npx tauri build --config "$R\0.9.1.json"
Pop-Location
foreach ($v in "0.9.0", "0.9.1") {
  Copy-Item -LiteralPath "$out\Jobert’s Vault Rehearsal_${v}_x64-setup.exe" "$R\setup-$v.exe"
  Copy-Item -LiteralPath "$out\Jobert’s Vault Rehearsal_${v}_x64-setup.exe.sig" "$R\setup-$v.exe.sig"
}
Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY, Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD
```

### 3. Write `latest.json` (three versions of it)

Keep `ascii`: PowerShell 5.1's `utf8` adds a hidden mark the updater can't read.

```powershell
function Write-Latest($version, $file, $sigOf) {
  $sig = (Get-Content "$R\$sigOf.sig" -Raw).Trim()
  @{ version = $version; platforms = @{ "windows-x86_64" = @{ signature = $sig; url = "http://localhost:8765/$file" } } } |
    ConvertTo-Json -Depth 4 | Set-Content "$R\latest.json" -Encoding ascii
}
Write-Latest "0.9.1" "setup-0.9.1.exe" "setup-0.9.1.exe"   # good
# Write-Latest "0.9.1" "setup-0.9.1.exe" "setup-0.9.0.exe" # D1: swapped installer
# Write-Latest "0.9.1" "setup-0.9.0.exe" "setup-0.9.0.exe" # D2: relabelled old release
```

### 4. Serve it

It needs Python 3 (the Microsoft Store `python` alias isn't enough). Run it in its own window, and start and stop it as the tests say. The folder is `$R`:

```powershell
python -m http.server 8765 --bind 127.0.0.1 --directory "$HOME\jv-rehearsal"
```

### 5. Test in this order

| Test | Do | Expect |
|---|---|---|
| A. Offline | Server off. Install `setup-0.9.0.exe` and open it. Finish the welcome form, add a transaction, then turn **Automatic updates** off. | It starts normally, with no error. |
| D1. Swapped installer | `latest.json` as D1. Server on. Settings → **Check for updates** → **Update now**. | "0.9.1 is out.", then refused with a signature error ending "Nothing was changed." It stays on 0.9.0. |
| D2. Relabelled old release | `latest.json` as D2. **Check for updates** → **Update now**. | Refused: "The update was signed for version 0.9.0 but the update endpoint announced version 0.9.1…" It stays on 0.9.0. |
| B. Update now, with a migration | `latest.json` good. **Check for updates** → **Update now**. | It closes, installs and reopens on 0.9.1. Settings shows 0.9.1. The transaction is still there, and `%APPDATA%\com.jobertsvault.app.rehearsal\backups\pre-migration\` has a `…-v1-….db` copy. |
| F. Error screen | Install `setup-0.9.0.exe` again by hand (server on). | It can't open the newer database, and the error screen shows. **Check for updates** → "0.9.1 is out." → **Update now** reopens it on 0.9.1, with the data intact. |
| C. Automatic | Uninstall, delete `%APPDATA%\com.jobertsvault.app.rehearsal`, server off. Install 0.9.0, finish the welcome form (automatic updates stay on), add a transaction and close. Server on. Open it. | "Updating Jobert's Vault to 0.9.1…", then it reopens on 0.9.1 with the transaction, and a new pre-migration copy. |

### 6. Clean up

- Stop the server.
- Uninstall **Jobert’s Vault Rehearsal** in Windows Settings → Apps.
- Delete `%APPDATA%\com.jobertsvault.app.rehearsal`, `%LOCALAPPDATA%\com.jobertsvault.app.rehearsal` and `$HOME\jv-rehearsal`.
- Unlink the junction first, so nothing can follow it into the real `node_modules`: `cmd /c rmdir C:\Projects\jv-rehearsal-src\node_modules` (never `Remove-Item`, which can recurse into it). Then `git worktree remove --force C:\Projects\jv-rehearsal-src`.

### Results

The latest rehearsal is recorded here.

_Not run yet: ROADMAP 1.10.7._
