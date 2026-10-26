# Decision records

Short records of the big decisions behind Jobert's Vault: what was decided, why, and what it costs. They exist so that six months from now nobody, human or AI, has to re-argue a settled question or quietly undo it.

## Index

| # | Decision | Status | Date |
|---|---|---|---|
| [0001](0001-tauri-and-react.md) | Tauri 2 with React, rather than Electron | Accepted | 2026-10-26 |
| [0002](0002-money-in-minor-units.md) | Money stored as integers in minor units (centavos) | Accepted | 2026-10-26 |
| [0003](0003-logic-in-rust.md) | Business logic in Rust; the frontend never runs SQL | Accepted | 2026-10-26 |
| [0004](0004-rusqlite-single-connection.md) | rusqlite with one connection, rather than sqlx with a pool | Accepted | 2026-10-26 |
| [0005](0005-balances-calculated-on-request.md) | Balances calculated when requested, never stored | Accepted | 2026-10-26 |
| [0006](0006-credit-card-accounting.md) | Card purchases are expenses when made; bill payments are transfers | Accepted | 2026-10-26 |
| [0007](0007-glassmorphism-2.md) | Visual style: Glassmorphism 2.0, with the aurora painted by the app | Accepted | 2026-10-26 |
| [0008](0008-fixed-window-size.md) | A fixed 1440 × 900 window that can't be resized | Accepted | 2026-10-26 |
| [0009](0009-offline-only.md) | Offline only: no network access at all | Accepted | 2026-10-26 |

## When to write one

Write a new record when a decision:
- is hard or expensive to reverse (a library, a storage format, a data rule),
- changes one of the rules in CLAUDE.md, or
- reverses an earlier record. **Never edit an accepted record to change its decision.** Write a new one, set the old one's status to "Superseded by NNNN", and link both ways.

Small choices (a component's padding, a variable name) don't need one.

## Template

Copy this into `NNNN-short-title.md`, using the next free number.

```markdown
# NNNN. Title in a few words

| | |
|---|---|
| **Status** | Proposed / Accepted / Superseded by NNNN |
| **Date** | YYYY-MM-DD |
| **Related** | PRD, DATA_MODEL, ARCHITECTURE sections; other records |

## Context
What problem or question forced a decision, and which facts matter.

## Decision
What was decided, stated plainly.

## Alternatives considered
Each serious option, and why it lost.

## Consequences
What gets easier, what gets harder, and what we now have to keep doing.
```
