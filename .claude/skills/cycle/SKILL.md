---
name: cycle
description: Use when the owner types /cycle to start the next release cycle - grill them on the changes, update the docs, then write one task prompt per ROADMAP task.
disable-model-invocation: true
argument-hint: "[what you want changed or added]"
---

# Start a release cycle

A cycle is one release. The owner brings the changes. This session works out exactly what they want, writes it into the docs, splits it into ROADMAP tasks, and writes one prompt per task. Each task then runs in its own session with `/task <id>`, and the owner types `/release` at the end.

**This session builds no app code.** It produces a docs PR and a git-ignored prompt file.

**The owner's permission (2026-11-23):** for this cycle's PRs, you may commit, push, open the PR, review, fix and squash-merge on green CI. That includes this docs PR. It ends when the cycle's close-phase PR merges, and it never covers a version bump or `/release`.

## 1. Start

1. Run `git status --porcelain`: it must print nothing. Then `git switch main`, `git pull --ff-only`.
2. **Resume if an earlier `/cycle` session stopped part-way:**
   - **An open docs PR:** `gh pr list --state open --search "head:docs/phase"` lists one. Switch to its branch and carry on from §4 step 5. The grill's answers are in its PRD §14 rows.
   - **A merged docs PR without prompts:** the highest phase in ROADMAP has no ticked box and no `.superpowers/phase<N>/` folder. Carry on from §5 with that N.
3. Otherwise, N = the highest `## Phase <n>` in ROADMAP.md, plus 1. Check the previous cycle is closed:
   - Every box in Phase N−1 is ticked.
   - No `.superpowers/phase*/` folder is left.

   If either fails, stop and tell the owner what's still open.
4. Run `python -I .claude/skills/task/tools/checkdocs.py`. It must print 0. If it doesn't, fix that first in its own `docs/` PR.

## 2. Read before asking

- What the owner wrote with `/cycle`. If they wrote nothing, ask them what they want changed or added.
- ROADMAP's `## Backlog`. Offer any items that fit with the request.
- For each change, the docs it touches, using CLAUDE.md's sources-of-truth table, and the code it touches. Keep reading until you can say how it works today and what the change would reach.

## 3. Grill

Invoke the `grill-me` skill on the owner's changes.

- Ask **one question at a time** with AskUserQuestion, with the recommended option first. Say what you found that the question rests on.
- Never ask what the docs or the code already answer.
- Ask about anything that would change what gets built:
  - how it treats money
  - the domain rules in CLAUDE.md
  - an empty state, an archived record, a future-dated row
  - the check-in, the forecast, a migration
- Stop when every change has a what, a why, its edge cases, and what it must not do.
- Then play the whole cycle back in a few lines (the phase name and each change) and get a yes.

## 4. The docs PR

Branch: `docs/phase<N>-<short-name>`. PR title: `docs: plan Phase <N>, <name>`.

1. **PRD:**
   - a new row per requirement, with a new ID in its area's series (`TXN-9`, …) and phase `P<N>`
   - `**P<N>**` added to §7's priority list, and a §11 release-plan row for Phase N
   - §14 decision log rows, dated today, for each answer from the grill
2. **Every connected doc**, using the stale-docs table in `.claude/skills/task/SKILL.md` §5:
   - DATA_MODEL first for any schema change, including the new migration's SQL word for word
   - then ARCHITECTURE, UI_FLOWS, DESIGN_SYSTEM, CLAUDE.md's domain rules, and an ADR for a big or hard-to-reverse decision
3. **ROADMAP:**
   - a new `## Phase N · <name>` section, placed before `## Backlog`, and its Overview row (`0 / <count>`)
   - Current focus set to its first task
   - Backlog items now in the phase taken out of the Backlog
4. **Tasks, granular:**
   - Each task is one PR, worth a reviewer's gate, and doable in one session.
   - List them top to bottom, so later tasks build on earlier ones.
   - Each task names its IDs (`[TXN-9]`).
   - End with two standard boxes:
     - review every doc against the shipped app, and update any that drifted
     - the release box: `Release vX.Y.Z with /release`, then use it in the installed app
   - The release box gets two prompts:
     - **get ready for the release:** checks only, no PR unless something needs fixing, and it ticks nothing. The owner types `/release`.
     - **close the phase:** ticks the release box, updates Current focus and README's status line, and deletes the cycle folder.
5. **Review:**
   - `checkdocs` prints 0.
   - Do the stale-docs check (task skill §5).
   - Run a builder check: start a separate reviewer with the Agent tool (`subagent_type: "general-purpose"`, `model: "opus"`, `effort: "high"`, read-only). Ask it to read the new sections as someone about to build each task, and to list every place it would have to guess, or where two sections contradict each other. If the tool refuses that model or effort, stop and tell the owner.
   - Fix the gaps. A finding that changes what the owner decided goes back to the owner.
6. Push. When CI is green on the head commit, run `gh pr merge <n> --squash --delete-branch`. `gh pr view <n> --json state` must say `MERGED`. Then `git switch main && git pull`.

## 5. The prompts

Write `.superpowers/phase<N>/PROMPTS.md` (the folder is git-ignored), after the docs PR merges, so the section numbers are final:

```markdown
# Phase <N> task prompts: <name>

How to run: start a new session in C:\Projects\JobertsVault on Claude Opus 5.5, type `/task T01`, and close it once its PR is merged and its LOG entry is written. Then start the next one.
Tasks that need you: <ids, and what each asks>.

## T01 · <title>
- **Task:** <ROADMAP line, word for word> [IDs]
- **Read first:** <doc §s and files, exact>
- **What matters:** <the traps, the decisions from the grill, what it must not do, what's for a later task>
- **Done when:** <what's tested, what the real-app test shows, screenshots if visible>
- **Branch:** `feat/<name>` · **PR title:** `feat: <reader-facing line> (IDs)`
```

- One block per ROADMAP box, numbered T01 upward in ROADMAP order. The release box gets two: get ready, then close the phase.
- A prompt carries only what's particular to its task. The loop, the permission and the review rules are in `/task`, so don't repeat them.
- A task that tests on a copy of the owner's real data must say exactly how, and must never touch the live database.

Also create `.superpowers/phase<N>/LOG.md` holding only `# Phase <N> log`.

## 6. Hand over

Tell the owner in a few lines:

- the phase and its name, and the docs PR that merged
- how many tasks there are, and which ones need them
- the next step: run `/task T01` in a new session

Ideas that come up mid-cycle go on the ROADMAP Backlog in the next PR that touches ROADMAP, unless the owner says they're urgent.
