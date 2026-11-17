# 0013. Levels are milestones worked out from the data, not points

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-11-17 |
| **Related** | PRD §7.18 (LVL-1 to LVL-8), DATA_MODEL §4.9 and §5.20–5.23, UI_FLOWS §8.8, ADR 0005 |

## Context

The owner wants to see their finances the way a game shows a character, "stats or financial status in visual form … just like how I can do better at a game that has levels", and to be shown how to improve.

Most game layers hand out points for activity: a badge for opening the app, XP for logging. Points like that only go up. A month of overspending would still earn a level, so the number would stop meaning anything about the money.

## Decision

- **Six stats, each 0 to 100, are worked out from the money data for today:** Cashflow, Bills, Card, Debt, Savings and Habit. They go down as well as up.
- **The level is a place on a fixed ladder of 10 milestones.** Each is a set of real conditions, such as nothing overdue, or savings that cover a month of spending. The level is the highest one whose conditions, and those of every level below it, hold today.
- **Both are calculated when asked for, never stored as the truth**, the same way balances are ([ADR 0005](0005-balances-calculated-on-request.md)). So the level is live: it can drop, and the app says why.
- **Nothing raises a stat or a level except the money data and finishing check-ins on time.** There are no points, and no reward for a click.
- **A finished check-in stores a snapshot** of the level and stats, only to show change: the hexagon's outline, the trend lines, the level-up moment. A snapshot is never recalculated.
- **There's no single overall score.** The level plays that part, and the six stats say where the weakness is.
- **The ladder puts a savings level before debt-free and another after it** (two weeks of spending, then one month), so an emergency doesn't force a new loan while the loans are being cleared.

## Alternatives considered

| Option | Why not |
|---|---|
| **XP and levels** | XP never drops, so a bad month only slows the climb and never shows as a fall. It rewards using the app more than improving the money. |
| **One health score, 0 to 100** | Honest, but there's nothing to climb and no named next step. |
| **Weekly quests and badges** | They give a target for the week, which the focus now does, but no picture of overall standing. |
| **A level that's kept once reached** | Kinder, but it can show something that's no longer true, such as "Bill Keeper" with a payment overdue. |
| **A level that changes only at a check-in** | Stable between reviews, but the sidebar would show a stale level, and it would need the level stored as state rather than calculated. |
| **Stats stored and updated by each save** | They could go stale or disagree with the balances, the reason ADR 0005 was written. |

## Consequences

- **Good:** the level always means something about the money. The next level's unmet conditions are a ready-made to-do list, which is where the next best move comes from.
- **Trade-off:** a level can drop the day a payment goes overdue. That's the point, but it must be said plainly, with the reason, and without animation.
- **Trade-off:** the formulas are judgement calls. They live in DATA_MODEL §5.20–5.21, with scales chosen to fit the owner's situation when they were written, so they'll need revisiting as it changes. They aren't settings: a change is a docs change first.
- **Cost:** the level needs the forecast (level 5), and the sidebar shows it on every screen, so `get_vault_status` is held to the 300 ms target and timed in the performance test (ARCHITECTURE §12).
- **Rule:** nothing may add a stored counter that goes up for using the app. A new stat or condition must be a pure function of the data and `today`, with a domain test at both ends of its scale.
