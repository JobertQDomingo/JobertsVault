# 0005. Balances calculated when requested, never stored

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-10-26 |
| **Related** | DATA_MODEL §5.1–5.2, ARCHITECTURE §4.3 and §12 |

## Context

Account balances, the net balance and period totals change whenever a transaction is added, edited or deleted, including old ones. They can be stored and kept up to date, or recalculated each time a screen asks for them.

## Decision

**Never store balances or totals.** Calculate them on every request with SQL `SUM`, using the indexes on `date`, `account_id`, `to_account_id` and `category_id`. Only an account's `opening_balance` is stored.

## Alternatives considered

| Option | Why not |
|---|---|
| **A `balance` column updated on every write** | Every create, edit, delete, merge and restore must update it correctly, and one missed path leaves the balance wrong. Editing a transaction from last year makes this especially error-prone. |
| **A running balance stored on each transaction** | Inserting or editing an older transaction means rewriting every later row. |
| **Cached monthly snapshots** | Worth it only if plain sums get slow. That isn't the case at the planned scale. |

## Consequences

- **Good:** a balance can never be stale or out of sync. Restores, edits and merges need no special code.
- **Good:** "balance as of a date" is the same query with a different date (future-dated transactions are excluded from today's balance this way).
- **Cost:** each dashboard load runs a few aggregate queries. With the indexes, that's well under the 300 ms target at 50,000 transactions. The Phase 1 release task checks this.
- **Revisit if:** the performance check fails. Then add cached monthly snapshots as a new decision record, keeping SQL sums as the source of truth.
