# 0012. The check-in is required, and still never blocks a save

| | |
|---|---|
| **Status** | Accepted, amended by [0014](0014-check-in-escape-on-failed-finish.md) |
| **Date** | 2026-11-17 |
| **Related** | PRD §7.17 (CHK-1 to CHK-9), DATA_MODEL §4.9 and §5.19, UI_FLOWS §8.7 and F17, ARCHITECTURE §5.4, DESIGN_SYSTEM §2.3 |

## Context

The Reports screen shows everything about the owner's money, but only when they choose to open it. The owner asked for a review that the app pushes at them every week and month, "a required pop up that will actually get my attention".

Until now nothing in the app blocks anything. The credit limit, the card spending levels and a goal that falls behind only warn: the hard spending level asks "Spend anyway?" and still saves (CLAUDE.md). The app's first goal is logging a transaction in under 5 seconds (PRD §2, TXN-5), and a screen that stands between the user and quick-add would break it on exactly the days a review is due.

## Decision

- **A due check-in takes over the window until it's finished.** It has no close button, Esc does nothing, and it can't be postponed. Every route redirects to it.
- **Saving is still never blocked.** + Add (Ctrl+N) works over the check-in, and so do the forms it offers: Pay bill, Pay and Record now. The check-in's figures update after each save.
- **It's a full-window route (`/check-in`), not a dialog.** Quick-add is a dialog, and a dialog over a dialog would put glass inside glass (DESIGN_SYSTEM §2.3). A route is also what the first-launch form already is.
- **Only the newest completed week and month are ever due.** Missed periods are skipped, so the user never returns to a queue of reviews.
- **Nothing is saved until Finish.** Closing the app shows the check-in again at the next start.
- **A check-in that can't be loaded doesn't lock the app.** The guard only redirects when the backend says one is due.
- **The focus nudge follows the old rule.** A save that takes a focus past its target shows a toast, and never asks or blocks.

## Alternatives considered

| Option | Why not |
|---|---|
| **A dashboard banner with "Review now"** | It's what the loan and card reminders do, and it's easy to ignore, which is the problem being solved. |
| **Required, with one "Later" button** | A snooze is the first step to ignoring it. The check-in is short, and logging isn't blocked, so there's little to postpone for. |
| **A full lock, quick-add included** | It breaks 5-second logging on review days, and a purchase not logged at once is often never logged. |
| **A dialog over the dashboard** | Quick-add and the pay forms would be dialogs inside a dialog, against the glass rules, and a dialog reads as something Esc should close. |
| **Stacking every missed period** | After a holiday the app would open on several reviews in a row, which invites clicking through all of them. |

## Consequences

- **Good:** every week and month the app is opened gets looked at, and the first screen of the week shows what's due and what went wrong while it can still be acted on.
- **Trade-off:** this is the one place the app refuses to move on. The rule "warn, never block" now reads: **nothing ever blocks a save, and only the check-in blocks navigation.**
- **Risk:** a required screen can be clicked through without reading. It's kept short, each step leads with one sentence, and the last step is a choice with nothing preselected, so Enter alone can't finish it.
- **Rule:** new forms opened from the check-in must be ordinary dialogs, and nothing but a finished check-in may clear the redirect. A frontend test checks that it can't be dismissed and that Ctrl+N opens over it.
- **Debug builds** show the check-in too, on `jobertsvault-dev.db`.
