# 0002. Money stored as integers in minor units

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-10-26 |
| **Related** | DATA_MODEL §1 and §6, ARCHITECTURE §4.2, §4.6 and §5.3, CLAUDE.md rule 1 |

## Context

Floating-point numbers can't represent most decimal amounts exactly: `0.1 + 0.2` is `0.30000000000000004`. Over thousands of transactions, these tiny errors make totals and balances disagree with real bank statements. Being exact is the app's core job.

## Decision

Store and calculate every amount as a **whole number of minor units**, which for the peso means **centavos**. ₱1,234.50 is stored as `123450`.
- **Rust** uses a `Money(i64)` type with checked arithmetic.
- **SQLite** uses `INTEGER`.
- **TypeScript** keeps the same integer in a `number`.
- Text is converted to and from minor units **only** at the edges: `src/lib/money.ts` for the UI and the CSV export in Rust.

## Alternatives considered

| Option | Why not |
|---|---|
| **`f64` / JavaScript numbers in pesos** | Rounding errors. This is the problem being solved. |
| **Decimal strings in SQLite** | Exact, but SQL can't `SUM` them correctly, so every total would be calculated outside the database. |
| **A decimal library (`rust_decimal`, `decimal.js`)** | Exact, but adds two libraries and conversion code on both sides for no benefit over integers, since the app has one currency with fixed decimals. |

## Consequences

- **Good:** sums are exact, SQL `SUM` is fast and correct, and a balance always matches the statement.
- **Rule:** never use `f64`, `parseFloat` or floating-point maths on money anywhere. Reviews must catch this.
- **Rule:** `currency_decimals` can't change once transactions exist, because the stored integers would change meaning.
- **Limit:** amounts are capped at 999,999,999,999 centavos. Both `i64` and JavaScript's exact-integer range (2⁵³) are far above that.
- **Display:** percentages and chart scaling can use floating point, because they're only for display. They're never stored or added back into money.
