# 0014. A check-in that can't be saved doesn't lock the app

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-11-19 |
| **Related** | [ADR 0012](0012-required-check-in.md) (this record amends one of its rules), PRD CHK-2, UI_FLOWS §8.7 and F17, ARCHITECTURE §5.4 |

## Context

ADR 0012 makes a due check-in take over the window, and says that nothing but a finished check-in may clear the redirect. It already lets the app open normally when the check-in can't be **loaded**. It says nothing about a check-in that loads but can't be **saved**.

If Finish kept failing, because of a bug or a database error, the owner would be left on the check-in with only quick-add. Settings (backup and restore), Accounts and Reports would all be out of reach until a fixed version installed itself, and only if automatic updates are on.

The same review found a second dead end. The offers depend on today, so a check-in loaded before midnight and finished after it is refused with "Pick one of the offered focuses". With the window kept in focus, nothing reloaded it, so it would keep being refused.

## Decision

- **When Finish fails, the error toast offers "Open the app anyway".** It lifts the redirect for this session only. Nothing is saved, and the check-in comes back at the next start, or sooner if the app is reloaded. There's still no skip, snooze or close button while saving works.
- **A refused Finish reloads the check-in at once.** A focus pick that's no longer among the offers is cleared, so the user picks again from the current ones.
- **The check-in is also asked for again at local midnight while the app is open**, as well as on every window focus, so a period that comes due overnight shows without switching windows.

This amends ADR 0012's rule "nothing but a finished check-in may clear the redirect". It now reads: **only a finished check-in, or a failed attempt to save one, may clear the redirect, and the second only until the next start.** The rest of ADR 0012 stands.

## Alternatives considered

| Option | Why not |
|---|---|
| **Keep it strict** | A bug in one save would lock every screen but quick-add, with backup and restore out of reach, until an update fixes it. |
| **An escape hatch in debug builds only** | The installed app is where a lock-out would do harm. |
| **Let Finish save without a focus when the offers changed** | The focus is the check-in's one choice (CHK-7). Reloading the offers keeps it a real choice. |

## Consequences

- **Good:** the required check-in can never stand between the owner and their data or their backups.
- **Trade-off:** there's now one way past a due check-in. It shows only after a save has failed, so it can't become a habit.
- **Rule:** the lifted redirect is held in memory only, never stored, so a restart always brings a due check-in back. Frontend tests check that the escape appears only after a failed Finish, and that a reload brings the check-in back.
