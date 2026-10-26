# 0003. Business logic in Rust; the frontend never runs SQL

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-10-26 |
| **Related** | ARCHITECTURE §1, §4 and §5, CLAUDE.md rule 2, [0001](0001-tauri-and-react.md), [0004](0004-rusqlite-single-connection.md) |

## Context

Tauri allows two designs:
- The React frontend talks to SQLite directly through `tauri-plugin-sql`, and the logic lives in TypeScript.
- The frontend calls Rust commands, and Rust owns the database and the rules.

Rules like "transfers are never income", how periods are cut and how card cycles work must be correct in exactly one place.

## Decision

**All business logic and all database access live in Rust**, split into layers:
- `commands`: thin wrappers that Tauri calls.
- `services`: rules and validation.
- `domain`: pure calculations.
- `repo`: SQL.

The React frontend only displays what commands return and sends user actions back. It **never runs SQL and never calculates totals or balances**.

## Alternatives considered

| Option | Why not |
|---|---|
| **`tauri-plugin-sql` from the frontend** | Fewer files at first, but money rules would spread across components, SQL would live in the UI, and the frontend would need wide database permissions. |
| **Logic in TypeScript, Rust as a thin data layer** | Puts the important code in the language without integer-safe types like `Money`, and splits testing across two places. |

## Consequences

- **Good:** a single source of truth for every rule.
- **Good:** the core is tested with fast `cargo test` runs. Pure domain functions are tested without a database, and services against an in-memory one.
- **Good:** the frontend stays simple, and security is tighter, because the UI has no file or database access.
- **Cost:** each feature needs a Rust command plus a typed TypeScript wrapper and hook. ts-rs generates the shared types, so they can't drift apart.
- **Rule:** `domain` and `services` never read the clock. `today` is passed in, so tests can fix the date.
