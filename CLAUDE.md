# CLAUDE.md

Guidance for AI coding assistants working on **Jobert's Vault**: a single-user Windows desktop app that keeps its data on the user's PC and tracks personal cashflow. It's built with Tauri 2, a React + TypeScript frontend, a Rust backend and SQLite.

> **Project status:** in development. See "Current focus" in [ROADMAP.md](ROADMAP.md) for where to continue.

## Sources of truth

Read the relevant doc **before** changing code in its area. If the code and a doc disagree, stop and ask. Don't silently pick one.

| Doc | Read it when you're… |
|---|---|
| [PRD.md](docs/PRD.md) | Deciding *what* a feature does. Requirements have IDs (`TXN-1`, `CC-3`, …) and a phase (P1–P4). |
| [DATA_MODEL.md](docs/DATA_MODEL.md) | Touching tables, SQL, balances, totals, periods, comparisons, card cycles, recurrence or validation |
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | Adding a command, module, library, screen or route, or changing startup, errors or security |
| [DESIGN_SYSTEM.md](docs/DESIGN_SYSTEM.md) / [UI_FLOWS.md](docs/UI_FLOWS.md) | Building UI. The style is **Glassmorphism 2.0**: follow the glass layer rules in DESIGN_SYSTEM §2 (text always on a panel, never blur inside blur, blur budget) and use only the design tokens. The window is a fixed 1440 × 900, so don't add responsive breakpoints for the app window. |
| [ROADMAP.md](ROADMAP.md) | Choosing what to work on next. Tick the box when a task is done, and keep "Current focus" up to date. |
| [docs/decisions/](docs/decisions/README.md) | Understanding why a big decision was made. To reverse one, write a new record that supersedes it. Never edit the old one. |

Build **only the current phase**. Don't build features from a later phase, or out-of-scope features (PRD §3), unless asked.

## Rules you must never break

1. **Money is an integer in minor units** (`i64` / `Money` in Rust, `number` holding minor units in TypeScript). Never use `f64`, `parseFloat` or floating-point maths on money. In the UI, only `src/lib/money.ts` converts between text and amounts.
2. **The frontend never runs SQL and never calculates totals or balances.** All business logic lives in Rust (`services/` + `domain/`). React only displays what commands return.
3. **No network access, except the updater** ([ADR 0010](docs/decisions/0010-automatic-updates.md)). `tauri-plugin-updater`, running in Rust, may fetch this project's GitHub releases, and nothing else. Don't add other HTTP clients, analytics or remote fonts or scripts, never send anything from the database, and don't loosen the content security policy.
4. **No Tauri `fs`, `shell` or `http` plugins.** Rust does all file work and validates the paths it's given.
5. **Dates are `'YYYY-MM-DD'` strings** across the Rust ↔ TypeScript boundary. Never pass a JavaScript `Date`. `domain/` and `services/` never read the clock: `today` and `now` are passed in.
6. **Every database write runs inside a database transaction**, and SQL always uses bound parameters.
7. **Schema changes:** update DATA_MODEL.md first, then add a **new** migration file (`src-tauri/migrations/NNNN_name.sql`). **Never edit a migration that has already been released.**
8. **Never edit generated files** in `src/bindings/`. Change the Rust struct and run `cargo test` to regenerate them.
9. **The error log never records amounts, payees, notes, or account and category names.**
10. **Releases only happen through `/release`, and always ask first.** Never push a tag, never delete or move a published release, and never commit or print the private signing key. Merging a version bump to `main` publishes to every installed copy.

## Domain rules that are easy to get wrong

- **Transfers are never counted as income or expense**, and that includes paying the credit card bill. They have no category, payee or payment method.
- **Balances are signed.** Credit card debt is a negative balance, and `amount_owed = max(0, −balance)`. In forms, the user types the amount owed as a positive number. Limits aren't enforced: a card can go past its limit, and an account can go below 0.
- **Opening balances are not income**, and forms accept only 0 or more.
- **Future dates are rejected until Phase 3** (TXN-8). From then on, future-dated transactions don't count toward the current balance or any total until their date arrives.
- **Refunds are income** (in the "Refund" category). They offset net cashflow but don't reduce expense totals or budgets.
- **Archived accounts still count toward the net balance.** Archiving an account whose balance isn't 0 requires confirming a warning.
- **Weeks start on Monday** by default. A month can start on any day from 1 to 28 (`month_start_day`). **Years are always calendar years.**
- **Comparisons:** a period still in progress is compared with the same number of days at the start of the previous period, unless `comparison_mode = full`. If the comparison total is 0, show "—".
- **Recurring dates** are always calculated from `start_date + n × interval`, never from the previous occurrence. Days past the end of a short month fall on its last day.
- **`currency_decimals` can't change** once any amount is stored: a transaction, or an account's non-zero opening balance or credit limit.
- **In-use records are archived, not deleted.** Accounts, categories and payment methods use `ON DELETE RESTRICT`. Unused ones can be deleted. The last active account, and the last active category of each kind, can't be archived or deleted.
- **Custom periods** move by the length of their range, and ▶ stops at the current period. The frontend never works out period dates: it uses what `get_period` returns.
- **Debug builds use `jobertsvault-dev.db`**, so development never touches real data. Never remove this split. Debug builds also never check for or install updates.
- **Migrations are backed up first.** An existing database is copied before any pending migration runs, and a failed backup stops the migration (BAK-5).

## Code layout (details in ARCHITECTURE.md §3)

```
src/            React frontend
  api/          the only place that calls invoke(), one file per area
  hooks/        TanStack Query hooks; every money-related key starts with ['ledger', …]
  features/     screens
  components/   shared components; ui/ holds the shadcn/ui building blocks
  lib/          money.ts, dates.ts, queryKeys.ts
  bindings/     ⚙ generated by ts-rs, never edit
src-tauri/src/
  commands/     thin #[tauri::command(async)] wrappers, with no logic
  services/     rules, validation, database transactions
  domain/       pure functions with no I/O; most of the tests live here
  repo/         SQL only
```

Layer rules: `commands → services → (domain, repo)`. Never call `repo` from `commands`, and never put SQL in `services`.

## Commands

Run these from a path with no apostrophe (ARCHITECTURE §11). The project lives at `C:\Projects\JobertsVault`.

```bash
# Development
npm install
npm run tauri dev                 # run the app with hot reload

# Checks (run them all before calling a task done)
npm run lint && npm run typecheck && npm test
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml     # also regenerates src/bindings/

# Local test build of the Windows installer. Real releases come from the Release workflow (ARCHITECTURE §11.1).
npm run tauri build
```

## Common workflows

**Adding a backend command**
1. Write the domain logic and its tests (if any) in `domain/`.
2. Add the service function in `services/`, including validation that returns `AppError::Validation { field, message }`.
3. Add the SQL in `repo/` if it's needed.
4. Add a thin wrapper in `commands/`, declared `#[tauri::command(async)]` so it runs off the main thread (ARCHITECTURE §4.3), and register it in `lib.rs`.
5. Derive `TS` on the input and output structs, mark `i64` fields `#[ts(type = "number")]`, then run `cargo test` to regenerate the bindings.
6. Add a typed wrapper in `src/api/<area>.ts` and a hook in `src/hooks/`. After any change that writes data, refresh the `['ledger']` queries.
7. Add the command to the catalog in ARCHITECTURE.md §4.5.

**Adding a table or column:** update DATA_MODEL.md → write a new migration → update repo and services → add a service test against an in-memory database.

## Testing expectations

- **Every domain function needs unit tests**, especially edge cases: month ends, leap years, Monday week start, `month_start_day` greater than 1, comparison windows, card cycles.
- **Service tests** use an in-memory SQLite database with all migrations applied. Check balances after creating, editing and deleting transactions.
- **Bug fixes start with a failing test.**
- Fix the date in tests by passing `today` in, e.g. `NaiveDate::from_ymd_opt(2026, 10, 26)`.

## Definition of done

- [ ] It meets the PRD requirement(s) it implements (mention the IDs in the commit message or PR).
- [ ] All the checks under **Commands** pass, with no clippy warnings or ESLint errors.
- [ ] There are tests for any new logic, and the bindings are regenerated and committed.
- [ ] Docs are updated if behavior, the schema or the command list changed.
- [ ] It doesn't use `unwrap()` outside tests, `any` in TypeScript, or `console.log` / `dbg!` left behind.

## Conventions

- **Naming:** `snake_case` in Rust and SQL, `camelCase` in TypeScript (serde `rename_all = "camelCase"`), `PascalCase` for components.
- **Branches and pull requests:** every change goes through a PR on a `feat/`, `fix/`, `docs/`, `chore/` or `ci/` branch, and is squash-merged once CI is green. The PR title becomes a line in the release notes, so write it for the reader, with a type prefix and requirement IDs, e.g. `feat: add transfer validation (TXN-1)`. This applies once ROADMAP 1.0 is done.
- **Commits:** imperative mood, short, with requirement IDs, e.g. `Add transfer validation (TXN-1)`. Don't add Claude attribution, such as a `Co-Authored-By: Claude ...` line.
- **Dependencies:** ask before adding any new crate or npm package that isn't listed in ARCHITECTURE.md §2.
- **Never commit** `*.db`, `*.db-wal`, `*.db-shm`, backups or exported CSV files.
- When a requirement is unclear, ask, then record the answer in the relevant doc's decision log.
