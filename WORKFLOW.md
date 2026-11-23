# Building the next update

How you take an idea for Jobert's Vault from "I want this" to an update installed on your PC, using the `/cycle` and `/task` commands. [RELEASING.md](RELEASING.md) covers the last step, the release itself. The commands' full instructions are in `.claude/skills/cycle/SKILL.md` and `.claude/skills/task/SKILL.md`.

## The idea

- **One cycle is one release.** A cycle can hold several changes: new features, updates to existing ones, and fixes.
- **Each cycle is the next phase.** After Phase 5 (`v1.0.0`) comes Phase 6. Its requirements are tagged `P6` in the PRD, and it gets its own section in ROADMAP.md.
- **The docs come first.** No code is written until the PRD and every connected doc say what to build.
- **Each task is one session and one PR.** Every session starts fresh, so the prompt file and the LOG carry everything between sessions.
- **Two different models.** One builds each task and another reviews it. A separate Opus 5.5 reviewer checks every PR, so the one reviewing never wrote the change.

```
you: /cycle <what you want>
  → grill (you answer questions)  → docs PR merged  → task prompts written
you: /task T01, T02, …            (one new session each; each one merges its own PR)
you: /release                     → update installs → /task <close> marks the phase done
```

## What you do, and what the agent does

| You | The agent |
|---|---|
| Say what you want changed or added | Reads the docs and code first, then asks only what they don't answer |
| Answer the grill's questions | Writes the PRD rows, the connected docs and the ROADMAP tasks, and merges the docs PR |
| Start each task in a new session | Builds, tests, tests in the real app, gets the review, fixes what it finds, and merges |
| Answer when a session stops to ask | Records your answer in the doc's decision log |
| Type `/release`, then use the update | Never releases, never bumps the version, never touches your real data |

**Permission:** you've allowed the agent to commit, push, open PRs, review, fix and merge every PR in a cycle (2026-11-23). It ends when the cycle's close-phase PR merges. Version bumps and `/release` are always yours.

## Step 1: start the cycle

1. **Before you start**, write down what you want, roughly. Have a look at the **Backlog** at the end of [ROADMAP.md](ROADMAP.md) too: it lists what's still open, and `/cycle` will offer anything that fits.
2. **Open a new Claude Code session** in `C:\Projects\JobertsVault`, on **Claude Opus 5.5**.
3. **Type `/cycle`** followed by your changes, e.g. `/cycle let me split a transaction across two categories, and fix the wording on the card focus`.
4. **Answer the grill.** It asks one question at a time, with its recommendation first. Pick that one, pick another, or type your own answer. It stops when every change has a what, a why and its edge cases. Then it plays the whole cycle back to you in a few lines. Say yes, or correct it.
5. **It opens the docs PR.**
   - The PR has the new PRD requirements, the connected docs and a new ROADMAP phase with small tasks.
   - A separate reviewer reads the new sections as the builder would, and lists where it would have to guess.
   - Anything that would change what you decided comes back to you.
   - Then the PR merges.
6. **It writes the task prompts** to `.superpowers/phase<N>/PROMPTS.md`. That folder is git-ignored, so it stays on your PC. It ends by telling you how many tasks there are, which ones need you, and the first command to run.

If the session stops part-way, run `/cycle` again. It picks up an open docs PR, or a merged one whose prompts were never written.

## Step 2: run the tasks

For each task, in order:

1. **Open a new session** on Claude Opus 5.5. Use a new one every time: a long session gets slower and less careful.
2. **Type `/task T01`** (then `T02`, `T03`, …).
3. **Let it work.** It branches, plans, builds with tests, runs every check, tests the running app if anything on screen changed, opens the PR, gets the review, fixes what it finds, waits for CI and merges.
4. **Answer if it asks.** It stops only when a doc and the code disagree, a new package is needed, a requirement is unclear, or something is outside its task.
5. **When it says the PR is merged and the LOG entry is written,** close the session.

Things you may want to look at, but don't have to:

- **The PR:** what changed, how it was tested, and screenshots in both themes for anything visible.
- **The PR comment** that lists each review finding, and whether it was fixed or ruled out.
- **The LOG** (`.superpowers/phase<N>/LOG.md`): one short entry per task.

`/task` refuses to run a task out of order, or one from an old cycle. If it refuses, it tells you which task is next.

## Step 3: release and close

1. **The "get ready" task** checks that everything is ready to ship. It makes no PR unless something needs fixing.
2. **Type `/release`** (see [RELEASING.md](RELEASING.md)). It suggests the version: a cycle with a new feature means a minor version (`1.1.0`), and fixes only mean a patch.
3. **Let the update install,** then use it for real. Check your data is all there and try what changed.
4. **Run the close task** (`/task` with the last id). It ticks the release box, updates the status lines and deletes the cycle's folder. That ends the cycle and the merge permission.

## When something goes wrong

| What happened | What to do |
|---|---|
| A session stopped before merging | Run the same `/task` in a new session. The LOG entry says where it stopped, and it finishes that PR first. |
| You disagree with a decision in a PR | Say so in that session, or in a new one. A changed decision goes into the doc's decision log first. |
| You think of something new mid-cycle | Tell any session to add it to ROADMAP's Backlog. It goes into the next cycle, unless you say it's urgent. |
| An urgent bug in the installed app | Open a session and describe it. It's fixed in its own `fix/` PR with a failing test first. Then `/release patch`. Outside a cycle, you merge it yourself unless you say otherwise. |
| CI is red on `main` | The session stops and tells you. Nothing merges until it's green. |
| `/cycle` says the last cycle isn't closed | Finish the open task, or run the close task. |

## Cheat sheet

```
/cycle <what you want>     start an update: grill → docs PR → task prompts
/task T01                  run one task, in a new Opus 5.5 session each time
/release                   publish it (you type this, always)
/task <last id>            after the update installs: close the phase
```
