---
name: release
description: Use when the user types /release to publish a new Jobert's Vault version to GitHub Releases and every installed copy.
disable-model-invocation: true
argument-hint: "[patch | minor | major | X.Y.Z]"
---

# Release Jobert's Vault

A release is a PR that changes only the version in `package.json`. Merging it to `main` runs `.github/workflows/release.yml`, which runs every CI check, builds and signs the installer, then publishes `vX.Y.Z` and creates its tag. Every installed copy with automatic updates on installs it at its next start. Never create or push a tag yourself. `RELEASING.md` describes the same steps for doing them by hand.

Run every command with Bash. Shell variables don't carry over from one Bash call to the next, so write the values you note (the last tag, PR number, merge commit) into later commands literally. At the first check that fails, stop and report it.

## 1. Start from a clean, current, green main
- `git status --porcelain` prints nothing. If it prints anything, stop and list what's uncommitted.
- `git switch main && git pull --ff-only`
- `gh run list --workflow ci.yml --branch main --limit 1` shows `completed` and `success`. If CI on `main` is red or still running, stop and say so.
- `git describe --tags --abbrev=0` prints the newest tag; note it as LAST. If there's no tag yet, this is the first release: everything on `main` ships, and `git log --oneline` lists it. Otherwise `git log <LAST>..HEAD --oneline` lists what ships. If the list is empty, stop: there's nothing to release.

## 2. Pick the version
- If `$ARGUMENTS` is a version like `0.2.0`, use it. If it's `patch`, `minor` or `major`, work out X.Y.Z from the version in `package.json`.
- With no argument, suggest one. Any `feat` commit since <LAST> means minor. Only `fix`, `docs`, `chore` or `ci` commits mean patch. Never suggest major; use it only when the user asks.
- X.Y.Z must be higher than the current version. While `package.json` says `0.0.0`, the only allowed version is `0.1.0` (ROADMAP 1.11), whatever the argument or the commits say: stop on anything else, including `patch`.
- Show the current version, the new version and the commit list. Wait for the user to confirm the number.
- If `git ls-remote --tags origin vX.Y.Z` prints a line, stop: that version is already released.

## 3. Open the release PR
    git switch -c release/vX.Y.Z
    npm version X.Y.Z --no-git-tag-version
    git diff --stat
    git commit -am "chore: release vX.Y.Z"
    git push -u origin release/vX.Y.Z
    gh pr create --title "chore: release vX.Y.Z" --body "<body>"
- `git diff --stat` must list only `package.json` and `package-lock.json`.
- The commit has no Claude attribution line.
- The body is `Releases **vX.Y.Z**: everything merged since <LAST>.` (or `since the start` for the first release), then one bullet per commit title from step 1.
- Note the PR number.

## 4. Wait for CI
- Run `gh pr checks <PR> --watch` in the background. It takes about 5 minutes.
- If it reports no checks yet, wait a few seconds and run it again. Make sure the run you're watching is for `git rev-parse HEAD`.
- If a check fails, show `gh run view <run-id> --log-failed`, then stop. Don't merge.

## 5. Ask before merging, every time
Ask: "CI is green. Merging publishes vX.Y.Z, and every installed Jobert's Vault with automatic updates on installs it at its next start. Merge now?"

Merge only on a yes given to this question, even if the user said "go ahead" earlier:

    gh pr merge <PR> --squash --delete-branch
    git switch main && git pull --ff-only

## 6. Watch the release run
- `gh pr view <PR> --json mergeCommit --jq .mergeCommit.oid` prints the merge commit; note it as SHA.
- `gh run list --workflow release.yml --commit <SHA>` gives the run ID. The run can take a few seconds to appear.
- Run `gh run watch <id> --exit-status` in the background. A release takes 20–30 minutes, because it runs every CI check before it builds.
- If the run fails, the release stays a hidden draft and nobody gets it. Show `gh run view <id> --log-failed`, then stop. The fix goes through a normal fix PR. Once that merges, `gh workflow run release.yml` finishes the release.

## 7. Confirm it's live
- `gh release view vX.Y.Z --json isDraft,assets`: not a draft, and the assets are `latest.json`, `JobertsVault_X.Y.Z_x64-setup.exe`, its `.sig` and `JobertsVault-setup.exe`.
- `curl -sL https://github.com/JobertQDomingo/JobertsVault/releases/latest/download/latest.json` shows `"version": "X.Y.Z"`, and its `url` ends in `JobertsVault_X.Y.Z_x64-setup.exe`. GitHub may serve the old file for a minute.
- `git fetch --tags`, then `git tag` lists `vX.Y.Z`.
- Report the release URL and what shipped. Say that installed copies update at their next start, and that copies with automatic updates off get it from **Settings → Check for updates**.
