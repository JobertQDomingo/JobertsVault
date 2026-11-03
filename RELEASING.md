# Shipping Jobert's Vault

How a change gets from your editor to every installed copy of Jobert's Vault. Read it top to bottom the first few times, then use the headings as a checklist. [ARCHITECTURE §11.1](docs/ARCHITECTURE.md) explains the design.

> **In progress.** This covers every change. The release loop, what to do when something goes wrong, the signing key and the rehearsal are added in ROADMAP 1.10.

## The idea

- `main` is always ready to ship. Changes reach it only through a pull request (PR) with green CI. The **Protect Main** ruleset enforces this.
- `package.json` holds the app's only version number. `src-tauri/tauri.conf.json` reads it (`"version": "../package.json"`).
- **Merging a change releases nothing.** Merging a **version bump** releases that version: GitHub builds and signs it, then publishes it, and every installed copy installs it at its next start.
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
