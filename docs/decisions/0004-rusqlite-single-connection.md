# 0004. rusqlite with one connection, rather than sqlx with a pool

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-10-26 |
| **Related** | ARCHITECTURE §2 and §4.3, DATA_MODEL §8, [0003](0003-logic-in-rust.md) |

## Context

The Rust backend needs a SQLite driver and migrations. The app has one user, one window and small, fast queries: a few milliseconds even with 50,000 transactions.

## Decision

- **Driver:** **`rusqlite`** with the `bundled` feature, so SQLite is compiled into the app and its version never varies.
- **Connection:** **one connection** stored in `AppState` behind a `Mutex`.
- **Migrations:** **`rusqlite_migration`**, which tracks the schema version in `PRAGMA user_version`.
- **Connection settings:** `foreign_keys = ON`, `journal_mode = WAL`, `busy_timeout = 5000` and `synchronous = NORMAL`.

## Alternatives considered

| Option | Why not |
|---|---|
| **`sqlx` (async, with a pool)** | Async and pooling solve concurrency problems this app doesn't have. Its compile-time query checking needs a database during builds, which complicates setup. |
| **Diesel / SeaORM** | An ORM adds a query language on top of SQL. This app's queries are mostly aggregations (`SUM … GROUP BY`), which are clearest as plain SQL. |
| **A pool of rusqlite connections** | With one user, there's nothing to run in parallel, and one connection makes write conflicts impossible. |

## Consequences

- **Good:** simple, synchronous code that's easy to test against an in-memory database. There are no pool settings to tune, and the SQLite version can't vary.
- **Cost:** no compile-time checking of SQL. **Service tests are the safety net:** every repo query runs in at least one test against the real migrations.
- **Note:** Tauri runs commands off the UI thread, so a busy query never freezes the window.
- **Revisit if:** background work, such as long imports, ever needs to run alongside normal use.
