# Architecture: Jobert's Vault

| | |
|---|---|
| **Status** | Approved v1.1 |
| **Last updated** | 2026-11-04 |
| **Related docs** | [PRD.md](PRD.md), [DATA_MODEL.md](DATA_MODEL.md) |

---

## 1. System overview

Jobert's Vault is a single **Tauri 2** desktop app. The window shows a React UI inside Windows' built-in browser engine (WebView2). The UI talks to a Rust backend through Tauri's **IPC** (inter-process communication) layer. The Rust backend owns the SQLite database file.

```
┌───────────────────────────── Jobert's Vault.exe ─────────────────────────────┐
│                                                                              │
│  ┌──────────── Frontend (WebView2) ────────────┐                             │
│  │ React + TypeScript                          │                             │
│  │  screens ─▶ hooks (TanStack Query) ─▶ api/  │                             │
│  └───────────────────────────│─────────────────┘                             │
│                              │ invoke("create_transaction", {...})  JSON     │
│                              ▼                                               │
│  ┌──────────── Backend (Rust) ─────────────────────────────────────────────┐ │
│  │ commands/   thin Tauri command handlers (one place for parsing and      │ │
│  │             error mapping)                                              │ │
│  │     │                                                                   │ │
│  │ services/   business rules: validation, database transactions,          │ │
│  │     │       working out balances and totals                             │ │
│  │     ├──────▶ domain/  pure logic with no database: Money, Period,       │ │
│  │     │                 comparison, card cycle, recurrence                │ │
│  │ repo/       SQL only (rusqlite)                                         │ │
│  └─────│───────────────────────────────────────────────────────────────────┘ │
│        ▼                                                                     │
│   %APPDATA%\com.jobertsvault.app\jobertsvault.db   (SQLite, WAL mode)        │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Core rules**
1. **The frontend never runs SQL** and never calculates money totals. It displays what the backend returns.
2. **All business logic lives in Rust**: the `services` layer, plus the pure `domain` layer, which has no database access.
3. **No network, with one exception.** The only connection is the updater, running in Rust, to this project's GitHub releases ([ADR 0010](decisions/0010-automatic-updates.md)). No HTTP plugin is installed, and the content security policy (CSP) blocks all outgoing connections from the UI (§10).

## 2. Technology choices

| Layer | Choice | Why |
|---|---|---|
| App shell | **Tauri 2** | Small installer (about 10 MB), low memory use, Rust backend, uses the WebView2 engine that ships with Windows 10 and 11 |
| UI framework | **React 19 + TypeScript** | Largest library ecosystem (charts, forms, accessible components), and the best supported by AI coding tools |
| Build tool | **Vite**, with `@vitejs/plugin-react` | Default for Tauri projects; fast reloading while developing |
| Styling | **Tailwind CSS 4**, with `@tailwindcss/vite` | Design tokens map directly to CSS variables. The style is Glassmorphism 2.0, built from `glass` / `glass-raised` utilities and `backdrop-filter` (see DESIGN_SYSTEM.md) |
| Font | **@fontsource/sora** (400, 600, 700) | The UI typeface, bundled into the app so no font is ever downloaded. It has tabular digits. It has no ₱ sign, so ₱ comes from the Segoe UI fallback (DESIGN_SYSTEM §4). |
| Components | **shadcn/ui** (built on the `radix-ui` package), with `clsx`, `tailwind-merge` and `class-variance-authority` | Accessible, keyboard-friendly building blocks that are copied into the repo, so they're ours to change. shadcn's toast asks for `next-themes`: don't add it, because the app sets the `.dark` class itself (DESIGN_SYSTEM §3.5). |
| Date picker | **react-day-picker + date-fns** | The shadcn/ui calendar. `Date` objects stay inside the picker: `lib/dates.ts` converts them to `'YYYY-MM-DD'` strings before anything else sees them. |
| Combobox | **cmdk** | The shadcn/ui combobox, with type-to-filter |
| Toasts | **sonner** | The shadcn/ui toast |
| Reordering | **@dnd-kit** (`core`, `sortable`, `utilities`) | Drag with the mouse, plus built-in keyboard and screen reader support |
| Icons | **lucide-react** | Matches the icon names stored on categories |
| Server state | **TanStack Query** | Caching, loading and error states, and refreshing data after changes |
| Routing | **React Router** | Simple routes, with the selected period kept in the URL |
| Forms | **react-hook-form + zod**, joined by `@hookform/resolvers` | Fast, keyboard-friendly forms with type-safe validation |
| Frontend tests | **Vitest, Testing Library** (`@testing-library/react`, `@testing-library/dom`), **jsdom** | Fast unit and component tests (§9) |
| Linting | **ESLint** (`@eslint/js`, `typescript-eslint`, `eslint-plugin-react-hooks`), **Prettier** | Consistent TypeScript and catches common mistakes (§13) |
| Charts (Phase 2) | **Recharts** | Built for React, and covers the bar, line and donut charts we need |
| Database driver | **rusqlite** (`bundled` feature) | SQLite is compiled into the app, so its version is always the same. It's synchronous, simple and fast. Pin it to the version `rusqlite_migration` supports, currently `0.40`. Newer releases fail to build alongside it. |
| Migrations | **rusqlite_migration** (`2.6`) | Tracks the schema version in `PRAGMA user_version` |
| Dates | **chrono** | Date arithmetic and the local "today" |
| Errors | **thiserror** | A typed `AppError` enum |
| Serialization | **serde** | Converts data to and from JSON for IPC. Fields are renamed from `snake_case` to `camelCase` for the frontend. |
| Rust → TS types | **ts-rs** | Generates TypeScript types from Rust structs, so the two sides can't drift apart |
| Updates | **tauri-plugin-updater** (`2`, with default features off and `native-tls`, `system-proxy` and `zip` on) | Downloads and verifies signed updates (§11.1). Stay on version `2`, because crates.io's newest is a `3.0.0` alpha. |
| Release builds | **tauri-apps/tauri-action** (`v0`, GitHub Actions only) | Builds and signs the installer, and writes `latest.json` (§11.1) |
| File dialogs (Phase 2) | **tauri-plugin-dialog** | Folder and file pickers for backup and restore |

Exact versions are pinned in `Cargo.lock` and `package-lock.json`, and upgraded on purpose rather than automatically.

## 3. Repository layout

```
jobertsvault/
├─ README.md  CLAUDE.md  ROADMAP.md  RELEASING.md
├─ docs/                           # PRD, DATA_MODEL, ARCHITECTURE, DESIGN_SYSTEM, UI_FLOWS
│  ├─ decisions/                   # decision records 0001–0010 (see docs/decisions/README.md)
│  └─ design/                      # visual previews of the design system and UI flows
├─ assets/brand/                   # logo.svg, one-color marks, app-icon-1024.png
├─ .cargo/config.toml              # tells ts-rs to write to src/bindings/ (§4.6)
├─ .claude/skills/release/SKILL.md # the /release command (§11.1)
├─ .github/workflows/              # ci.yml (the checks in §9), release.yml (§11.1)
├─ package.json  package-lock.json  components.json  eslint.config.js  .prettierignore
├─ vite.config.ts  tsconfig.json  tsconfig.node.json
├─ src/                            # ── Frontend ──
│  ├─ main.tsx  App.tsx  routes.tsx
│  ├─ api/                         # one typed wrapper per Tauri command (the only place that calls invoke)
│  │  ├─ accounts.ts  transactions.ts  categories.ts  paymentMethods.ts
│  │  ├─ dashboard.ts  period.ts  settings.ts  system.ts
│  │  └─ client.ts                 # invoke wrapper + AppError handling
│  ├─ bindings/                    # ⚙ generated by ts-rs; never edit by hand
│  ├─ hooks/                       # useAccounts, useTransactions, useDashboard, … (TanStack Query)
│  ├─ features/                    # screens, one folder each
│  │  ├─ dashboard/  transactions/  accounts/  categories/  settings/  onboarding/
│  │  ├─ reports/ (P2)  recurring/ (P3)  budgets/ (P3)  goals/ (P3)
│  ├─ components/
│  │  ├─ ui/                       # shadcn/ui building blocks
│  │  └─ MoneyText.tsx  AmountInput.tsx  PeriodSwitcher.tsx  QuickAddDialog.tsx …
│  ├─ lib/
│  │  ├─ money.ts                  # parse and format minor units (the only place that does this)
│  │  ├─ dates.ts                  # formatting 'YYYY-MM-DD' for display, and date picker conversion
│  │  ├─ categoryIcons.ts          # the curated icon list (DESIGN_SYSTEM §6)
│  │  ├─ queryKeys.ts
│  │  └─ utils.ts                  # cn(), the class name helper shadcn/ui expects
│  ├─ styles/globals.css           # design tokens (CSS variables)
│  └─ test/setup.ts                # Vitest setup
└─ src-tauri/                      # ── Backend ──
   ├─ Cargo.toml  tauri.conf.json
   ├─ capabilities/default.json    # the minimum permissions the frontend needs
   ├─ migrations/0001_initial.sql
   └─ src/
      ├─ main.rs                   # entry point
      ├─ lib.rs                    # builds the app, registers commands, startup steps
      ├─ state.rs                  # AppState { db: Mutex<Connection>, … }
      ├─ error.rs                  # AppError, and how it's serialized
      ├─ log.rs                    # appends to the error log (§4.4)
      ├─ db/  mod.rs  migrations.rs  seed.rs
      ├─ domain/                   # pure, unit-tested, no database
      │  ├─ money.rs  period.rs  comparison.rs (P2)  card_cycle.rs  recurrence.rs (P3)
      ├─ repo/                     # SQL only, one module per table
      │  ├─ accounts.rs  categories.rs  payment_methods.rs  transactions.rs  settings.rs
      ├─ services/                 # rules + database transactions
      │  ├─ accounts.rs  categories.rs  payment_methods.rs  transactions.rs  dashboard.rs  settings.rs
      │  ├─ reports.rs (P2)  backup.rs (P2)  export.rs (P2)  recurring.rs (P3)  budgets.rs (P3)  goals.rs (P3)
      └─ commands/                 # #[tauri::command(async)] wrappers, one module per area
```

## 4. Backend design

### 4.1 Layers

| Layer | Allowed to | Not allowed to |
|---|---|---|
| `commands` | Convert input data (DTOs), call one service function, return `Result<T, AppError>` | Contain business rules or SQL |
| `services` | Validate input, open a database transaction, call the repo and domain layers, build response DTOs | Contain raw SQL text or anything Tauri-specific |
| `domain` | Do pure calculations on plain values | Do any input/output: no database, files or clock (the date `today` is passed in) |
| `repo` | Run parameterized SQL and map rows to structs | Contain business rules |

`domain` and `services` never read the system clock directly. "Today" and, for `created_at` / `updated_at`, the current time `now` are passed in as values, so tests can fix them. The command layer gets them from `chrono::Local::now().date_naive()` and `chrono::Utc::now()`.

### 4.2 Key domain types

```rust
pub struct Money(pub i64);             // minor units; checked arithmetic, never f64
pub enum Decimals { Zero, Two }        // currency_decimals, for Money::parse and Money::to_plain
pub enum PeriodKind { Day, Week, Month, Year, Custom }
pub struct Period { kind: PeriodKind, start: NaiveDate, end: NaiveDate }   // inclusive; private fields, read by kind()/start()/end()
pub struct PeriodSettings { week_start: Weekday, month_start_day: u32 }
impl Period {
    fn containing(kind, anchor, &PeriodSettings) -> Option<Period>;   // DATA_MODEL §5.3; None for Custom
    fn custom(start, end) -> Option<Period>;                          // None when end < start
    fn previous(&self, &PeriodSettings) -> Option<Period>;            // every fn is None past chrono's date range
    fn next(&self, &PeriodSettings, today) -> Option<Period>;         // None when it would start after today
}
fn comparison_window(current: &Period, today: NaiveDate, mode: ComparisonMode) -> Period; // §5.4
pub struct CardCycle { start: NaiveDate, end: NaiveDate, due: NaiveDate }  // end = statement closing date
fn card_cycle(statement_day: u8, due_day: u8, today: NaiveDate) -> Option<CardCycle>;     // §5.5; None past chrono's range
fn card_figures(balance: Money, credit_limit: Money) -> CardFigures;  // amount owed, available credit, utilization % (§5.1)
```

### 4.3 Database access

- **One SQLite connection** stored in `AppState` behind a `Mutex`. With a single user and queries that take milliseconds, this is simpler than a connection pool and avoids write conflicts.
- **Every command is declared `#[tauri::command(async)]`**, and a test in `commands/` fails if one isn't. Tauri runs a plain command on the main thread, which also draws the window, so a slow query, or a wait for the connection's `Mutex`, would freeze it. With `async`, a normal `fn` runs on a worker thread of Tauri's async runtime and can still borrow `State`. (An `async fn` that borrows `State` must return a `Result`.) Database calls take milliseconds, so briefly holding a worker is fine. Work that can take seconds, such as backup and restore (Phase 2), runs inside `tauri::async_runtime::spawn_blocking` so it doesn't hold up the runtime's few workers.
- Settings applied when the connection opens: `foreign_keys = ON`, `journal_mode = WAL`, `busy_timeout = 5000`, `synchronous = NORMAL`.
- **Every write runs inside a database transaction.** One change, such as merging categories or posting a recurring occurrence, is always one transaction.
- Queries always use **bound parameters**. Values are never pasted into SQL text.
- Balances and totals are calculated **when they're requested**, using the indexes (DATA_MODEL §3.4). Nothing is cached in the database, so a balance can never go stale. 50,000 rows add up in a few milliseconds.

### 4.4 Errors

```rust
#[derive(thiserror::Error, Debug, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum AppError {
    Validation { field: String, message: String },  // shown next to the form field
    NotFound   { entity: String, id: i64 },
    Conflict   { message: String },                 // e.g. "category is in use"
    Database   { message: String },                 // logged in full; the user sees a generic message
    Io         { message: String },
    Update     { message: String },                 // a failed update check or install; says why, and that nothing changed
}
```

The frontend's `api/client.ts` converts these into a typed `ApiError`:
- `validation` errors are shown on the matching form field (via react-hook-form's `setError`).
- Other errors are shown as a toast notification. For `database` errors, the toast text is generic ("Something went wrong with the database…"), and SQLite's own text stays in `ApiError.detail`.
- Anything Tauri itself rejects with (an unknown command, bad arguments) becomes code `unknown`.

**Error log:** Rust appends `Database` and `Io` errors (and `Update` errors, once ROADMAP 1.10 adds them) and startup failures to `<app data>\jobertsvault.log`, using only the standard library. When the file passes 1 MB, it's renamed to `jobertsvault.log.1`, replacing the older copy, so the log never takes more than about 2 MB. If that rename fails, the entry is skipped rather than let the log grow. **It never records amounts, payees, notes, or account and category names.**
- A startup failure of any other kind (`validation`, `conflict`, `not_found`) is logged by its code only, because those messages can hold what the user typed. `startup_failures_never_record_user_text` guards this.
- Converting a rusqlite error drops the stored values that some of its messages carry. SQL text can still appear, but it never holds user data, because every query binds its values. `the_log_never_records_user_data` guards this.
- Each entry is one line, and a failure to write the log is ignored, so it can't stop the app.

### 4.5 Command catalog

All commands use `camelCase` parameter names in TypeScript and `snake_case` in Rust. Tauri converts between them automatically.

| Phase | Area | Commands |
|---|---|---|
| 1 | System | `get_startup_status` → `ok`, or the error message and database path; `open_data_folder`; `get_app_version` |
| 1 | Updates | `check_update` → the newer version, or none (always none in debug builds); `install_update` (doesn't return on success, because the app restarts) |
| 1 | Period | `get_period(query)` → start, end, previous query, next query (none when it would start after today), whether it's the current period, `today` |
| 1 | Accounts | `list_accounts` → every account (archived included) and the net balance; `get_account(id)`; `create_account(account)`; `update_account(id, account)` (the kind can't change); `archive_account(id, acknowledge_balance)`; `unarchive_account(id)`; `delete_account(id)` (unused accounts only); `reorder_accounts(ids)`. Each account carries its signed `balance` as of today, `inUse`, and for a credit card a `card` summary: amount owed, available credit, utilization as a whole percent, the current cycle, its spending so far and the next due date |
| 1 | Categories | `list_categories`, `create_category`, `update_category`, `archive_category`, `unarchive_category`, `delete_category`, `reorder_categories`, `merge_categories` |
| 1 | Payment methods | `list_payment_methods`, `create_payment_method`, `update_payment_method`, `archive_payment_method`, `unarchive_payment_method`, `delete_payment_method`, `reorder_payment_methods` |
| 1 | Transactions | `list_transactions(filter, page)`, `get_transaction`, `create_transaction`, `update_transaction`, `delete_transaction`, `get_transaction_defaults` → account and today (DATA_MODEL §5.8), `get_pay_bill_defaults(card_id)` → From account, amount and today |
| 1 | Dashboard | `get_dashboard(query)` → period, totals, balances, net balance, card summary, recent transactions, `today` |
| 1 | Settings | `get_settings` → every Phase 1 key and whether decimals are locked; `update_settings(settings)` saves the whole set and returns it the same way |
| 2 | Reports | `get_timeseries`, `get_category_breakdown`, `get_comparison`, `suggest_payees(prefix)` |
| 2 | Backup | `backup_now`, `list_backups`, `restore_backup(path)`, `export_csv(range, path)` |
| 3 | Planning | `list/create/update/pause/delete_recurring_rule`, `list_due_occurrences`, `post_occurrence`, `skip_occurrence`, budget CRUD + `get_budget_progress`, goal CRUD + `get_goal_progress` |

**Dashboard data comes from one command.** `get_dashboard` returns everything the dashboard needs in a single response, so the screen loads in one round trip and every number on it is consistent with the others.

**Period queries.** `query` is `{ kind, anchor }` for Day, Week, Month and Year, with no anchor meaning today, or `{ kind: "custom", start, end }` for Custom. The frontend never works out period dates. ◀ and ▶ use the previous and next queries that `get_period` returns (DATA_MODEL §5.3).

**The transaction list.** `filter` includes a period query. `list_transactions` returns:
- `items`, the page of 100 rows, and whether there's another page.
- `summary`: the number of matches and their income and expense totals, across all pages.
- `dayNets`: the net for every date on the page, covering all of that date's matching rows, even when a date continues on the next page.

**Duplicating.** There's no duplicate command. The form loads the row with `get_transaction`, uses today's date from `get_transaction_defaults`, and saves with `create_transaction`, so nothing is created until the user saves (TXN-2).

### 4.6 Rust ↔ TypeScript types

- Every struct that crosses IPC derives `Serialize/Deserialize` and `ts_rs::TS` with `#[serde(rename_all = "camelCase")]`.
- Running `cargo test` regenerates the files in `src/bindings/` (the folder is set by `TS_RS_EXPORT_DIR` in `.cargo/config.toml`). These files are committed to git, and CI fails if they're out of date. ts-rs doesn't delete old files, so delete the file by hand when you remove a type.
- **Money crosses IPC as a plain integer of minor units.** JavaScript numbers are exact up to 2⁵³ (about 9 × 10¹⁵), far above the app's maximum amount (DATA_MODEL §6).
- **Mark `i64` as `number`.** ts-rs generates `bigint` for `i64` by default, but the JSON holds a plain number. Add `#[ts(type = "number")]` to `Money` and to every `i64` field, including IDs.
- **Dates cross IPC as `'YYYY-MM-DD'` strings**, never as JavaScript `Date` objects, which can shift a date across midnight because of time zones.

## 5. Frontend design

### 5.1 State

| What | Where |
|---|---|
| Data from the backend (accounts, transactions, totals, settings) | TanStack Query cache |
| The selected period and filters | URL search parameters, e.g. `/?period=month&anchor=2026-10-26` or `/?period=custom&start=2026-10-01&end=2026-10-10`, so the back button and reloads work |
| Open dialogs, form drafts | Local component state |

There's no global state library. With TanStack Query plus the URL, none is needed.

### 5.2 Query keys and refreshing data

- Every query key that involves money starts with `['ledger', …]`, e.g. `['ledger','dashboard',period]` or `['ledger','transactions',filter]`.
- **After any successful change, all `['ledger']` queries are refreshed.** This is a deliberately blunt rule: any transaction can affect every balance and total, and reloading the data is cheap.
- Settings use their own key `['settings']`. Changing settings also refreshes `['ledger']`, because things like the week start change the totals.
- `refetchOnWindowFocus` stays on, so "today" moves forward if the app is left open past midnight.

### 5.3 Money in the UI

- `lib/money.ts` is the **only** place that converts between text and minor units:
  - `parseAmount("1,234.5", decimals) → 123450`
  - `formatMoney(123450, settings) → "₱1,234.50"`
- `<AmountInput>` and `<MoneyText>` use these functions. Components never format money themselves.
- `<MoneyText>` marks income and expenses with a **sign or icon as well as color** (PRD accessibility rule).

### 5.4 Routes

| Path | Screen | Phase |
|---|---|---|
| `/` | Dashboard | 1 |
| `/transactions` | Transaction list and filters | 1 |
| `/accounts`, `/accounts/:id` | Accounts and account details (card details for a credit card) | 1 |
| `/settings` (tabs: general, categories, payment methods) | Settings | 1 |
| `/welcome` | First-launch form, shown while `onboarding_completed` is false | 1 |
| `/reports` | Reports | 2 |
| `/settings/backup` | Backup and restore | 2 |
| `/recurring`, `/budgets`, `/goals` | Planning | 3 |

Quick-add (**Ctrl+N**) is a dialog that can be opened from any route. It isn't a page of its own.

## 6. Startup sequence

```
1. Resolve the app data folder           (%APPDATA%\com.jobertsvault.app\)
2. Open jobertsvault.db, apply the connection settings
3. Check user_version  ── newer than the app knows? ─▶ error screen, stop
4. If an existing database has pending migrations: back it up first (BAK-5) ── fails? ─▶ error screen, stop
5. Run migrations
6. If the database is new: add the seed data (DATA_MODEL §7), inside migration 0001's transaction, so it lands with the schema or not at all
7. [P2] Daily auto-backup, if not done yet today (BAK-2), then delete old ones (BAK-3)
8. [P3] Process recurring rules due up to today (REC-4)
9. Store AppState, apply the window size (§11) and show the main window
```

**Development builds use their own files.** Debug builds (`cfg!(debug_assertions)`, which includes `npm run tauri dev`) open `jobertsvault-dev.db`, write to `jobertsvault-dev.log`, put their pre-migration backups in `backups-dev\pre-migration` (DATA_MODEL §8) and, from Phase 2, default to a `backups-dev` folder. An unreleased migration can therefore never reach real data. Release builds use the real names.

The window starts hidden and is shown in step 9, even if an earlier step failed. A failure is logged (§4.4) and stored in `AppState` in place of the connection, and every data command then returns an error.

The frontend calls `get_startup_status` first:
- **On failure:** it shows an error screen with the database path and the error message, plus "Open data folder" and "Check for updates" buttons (UPD-6). It never shows a blank window.
- **On success:** it reads the settings. If `auto_update_enabled` is true, it runs the update check (§11.1) before anything else. Then it goes to `/welcome` if `onboarding_completed` is false.

`open_data_folder` opens Explorer at the app data folder with `std::process::Command`. It takes no path from the frontend, and needs no plugin.

## 7. Backup and restore (Phase 2)

**The pre-migration backup (BAK-5) is in Phase 1**, because automatic updates can carry a migration. It uses the same `VACUUM INTO` and rename steps, runs in startup step 4, and is kept as DATA_MODEL §8 describes. Everything else in this section is Phase 2.

- **Backup:** run `VACUUM INTO '<folder>/jobertsvault-YYYY-MM-DD_HHMM.db.tmp'`, then rename the file to `.db`. Renaming at the end means a half-written file never looks like a real backup. `VACUUM INTO` produces a consistent, compacted copy even while the app has the database open.
  - **Automatic** backups go in `<backup_folder>/auto/`, and the newest 30 are kept.
  - **Manual** backups go in `<backup_folder>/` and are never deleted automatically.
- **Restore:**
  1. Open the chosen file **read-only** and run `PRAGMA integrity_check`. Check that its `user_version` is no newer than the app's.
  2. Make a safety backup of the current database (`pre-restore-…db`).
  3. Close the connection, copy the chosen file over `jobertsvault.db`, and delete the leftover `-wal` and `-shm` files.
  4. Reopen the database, run migrations to bring it up to date, and refresh the whole UI.
- **CSV export:** done by the backend (DATA_MODEL §8), which writes the file to a path the user chooses in a save dialog.

## 8. Recurring transactions (Phase 3)

For each rule that isn't paused, in one database transaction per rule:

```
while occurrence(rule, next_index) ≤ today and the rule hasn't ended (end_date / max_occurrences):
    INSERT a recurring_occurrences row (rule_id, date)   -- the UNIQUE key blocks duplicates
    if mode = auto:    create the transaction, status = 'posted'
    else:              status = 'pending'
    next_index += 1
```

This runs at startup and again whenever the date changes while the app is open (checked when the window regains focus).

## 9. Testing strategy

| Level | Tool | What's covered |
|---|---|---|
| **Domain unit tests** (most important) | `cargo test` | Period boundaries for every kind of period, week start and month start day, including leap years and month ends; the comparison window; the card cycle and due date; recurrence dates (31st → end of month, Feb 29); converting money to and from text |
| **Service tests** | `cargo test` with an in-memory SQLite database and migrations applied | Balances after creating, editing and deleting transactions; transfers left out of totals; validation errors; archive and merge rules; recurring transactions not duplicated |
| **Frontend unit tests** | Vitest + Testing Library | `money.ts` parsing and formatting, `AmountInput`, `PeriodSwitcher`, form validation |
| **Manual acceptance tests** | A checklist | The PRD §10 acceptance criteria, run on Windows 11 before each release |

**Files, clock and build in tests.** Tests that need real files (startup, backups, the log) use `test_support::TempDir`, a folder under the system temp directory that's wiped afterwards, never the app data folder. Wherever code takes the time as a value, tests pass the fixed `test_support::now()` (2026-11-03 14:05 local). The log is the one exception: it stamps its own entries. A test that needs a file name the app itself uses builds it from `db::NAME` and `db::BACKUPS_DIR`, so it passes in both debug and release builds. No `.db` file or backup is ever committed.

**Example tests that must exist:** "Paying the card bill doesn't change the month's expense total", "Week of 2026-10-26 (a Monday) is Oct 26 to Nov 1", "Month start day 25: Oct 2026 is Sep 25 to Oct 24", "Comparing Oct 1–26 uses Sep 1–26".

**CI (required from Phase 0, GitHub Actions on `windows-latest`):** on every pull request and every push to `main`, `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, a check that the bindings are up to date, `npm run lint`, `npm run typecheck` and `npm test`. A newer run on the same branch cancels the older one. CI doesn't build the installer. The Release workflow does that (§11.1), after running the same checks.

## 10. Security and privacy

- **Tauri capabilities:** the frontend is granted only the core window permissions and, from Phase 2, the dialog plugin. There's **no** `fs`, `shell`, `http` or `opener` plugin. If the Tauri project template adds `tauri-plugin-opener`, remove it and its permission during Phase 0. All file work happens in Rust, which validates the paths it's given.
- **Updates run in Rust.** The frontend gets no updater permissions, and calls only our own `check_update` and `install_update` commands.
- **Content security policy:** `default-src 'self'; connect-src ipc: http://ipc.localhost; img-src 'self' data:; style-src 'self' 'unsafe-inline'`. With this, the UI can't load anything from the internet.
- **The updater is the only network use.** It requests `latest.json` and the installer from GitHub, and sends nothing from the database ([ADR 0010](decisions/0010-automatic-updates.md)).
- **No telemetry or crash reporting.** The error log (§4.4) stays on the computer and holds no financial details.
- **Update signing:** the app holds only the public key. The private key is kept by the owner and in GitHub's secrets, and it's never committed (§11.1).
- **The data is not encrypted.** It's protected by your Windows account and disk encryption, such as BitLocker. Encrypting the database could be added later (e.g. with SQLCipher) if needed. This is recorded as a known limitation.

## 11. Build and distribution

- `npm run tauri dev` starts the app for development, with hot reload.
- **App icon:** `npm run tauri icon assets/brand/app-icon-1024.png` generates `src-tauri/icons/` (the `.ico` and every size) from the logo (DESIGN_SYSTEM §6.1).
- `npm run tauri build` produces an **NSIS `.exe` installer** (recommended for a single user, as it doesn't need administrator rights) and, optionally, an `.msi`.
- `productName`: `Jobert’s Vault`, with a **curly apostrophe** (U+2019) · `identifier`: `com.jobertsvault.app` · WebView2 is installed by the installer if it's missing.
- **Apostrophes break the build:** a straight apostrophe (`'`) in `productName` makes the NSIS installer fail, and one in the project's path makes the Rust build fail at the Windows resource step. Keep the repository at a short path with no apostrophe, such as `C:\Projects\JobertsVault`. Paths past 260 characters also fail to link. Text inside the app, including the window title, can use a straight apostrophe.
- **Window:** a normal opaque window. No `windowEffects` (Mica or Acrylic) and no transparent window. The app paints its own aurora background (DESIGN_SYSTEM §2.2), so the desktop wallpaper never affects readability.
- **Fixed size:** `tauri.conf.json` sets the window to `"width": 1440, "height": 900, "resizable": false, "maximizable": false, "center": true, "visible": false`. At startup, Rust checks the current monitor's work area (`current_monitor()?.work_area()`). If 1440 × 900 plus the title bar doesn't fit, it switches to 1280 × 800 and sets the webview zoom to 8/9 (about 88.9%) before showing the window. No window-state plugin is needed.
  - **The layout doesn't change:** at that zoom, the page still measures 1440 × 900 CSS pixels, so there's no second layout and no breakpoints.
  - **Known limit:** on a 1080p monitor at 125% scaling, only about 816 px of height is usable, and the fallback needs about 832 px. This is accepted because both of the owner's monitors run at 100% ([ADR 0008](decisions/0008-fixed-window-size.md)).
- **Updates are automatic** (§11.1). Your data is kept because it's in `%APPDATA%`, not the install folder.
- **Code signing:** the installer isn't signed for Windows, so Windows SmartScreen shows a "More info → Run anyway" warning on a first install. That's acceptable for a personal app. The separate update signature (§11.1) is what protects updates.

### 11.1 Releases and updates

The process is the same as NBSBuddy's. [RELEASING.md](../RELEASING.md) has the step-by-step guide.

```
a change:   branch → PR → CI ✓ → squash-merge → main                   (nothing released)
a release:  release/vX.Y.Z → PR → CI ✓ → merge → Release workflow → installed copies update
```

**The rules**
- `main` is always ready to ship. A **Protect Main** ruleset allows changes only through a PR with green CI, and blocks force pushes and deleting the branch.
- `package.json` holds the app's **only version number**, and `tauri.conf.json` reads it (`"version": "../package.json"`).
- **Merging a change releases nothing. Merging a version bump releases that version.**
- **Never push a tag by hand.** Publishing a release creates its tag.
- **Forward only:** a published release or tag is never deleted or moved. A bad release is fixed by a newer one.
- The `/release` command runs a release, and always asks before the merge that publishes it.

**The Release workflow** (`.github/workflows/release.yml`) runs when `package.json` changes on `main`, or by hand after a failed run:
1. **`version`** reads `package.json`. If that version's tag already exists, it stops, which is why ordinary merges release nothing.
2. **`release`**, on Windows:
   - Runs every check from §9.
   - Removes any draft left by a failed run.
   - Uses `tauri-action` to build and sign the installer into a **hidden draft**.
   - Uploads `JobertsVault-setup.exe`, a stable name for a direct download.
   - Checks that all 4 files are there: `latest.json`, `JobertsVault_X.Y.Z_x64-setup.exe`, its `.sig` and `JobertsVault-setup.exe`.
   - Writes the notes from the PR titles, then publishes.

A failed release stays a hidden draft, so no installed copy ever sees it.

**Asset names must be plain ASCII.** The curly apostrophe and space in the product name don't survive as GitHub asset names, and `latest.json` would then point at a file that doesn't exist. So `tauri-action` gets `assetNamePattern: JobertsVault_[version]_[arch][setup][ext]`. This is the one real difference from NBSBuddy's workflow. `tauri-action` only runs on GitHub, so the rehearsal can't test it. The workflow's check for all 4 files is what catches a wrong name, and the release then stays a draft.

**Configuration**

```json
"bundle": {
  "targets": ["nsis"],
  "createUpdaterArtifacts": true,
  "publisher": "Jobert Domingo",
  "windows": { "nsis": { "installMode": "currentUser" } }
},
"plugins": {
  "updater": {
    "pubkey": "<the public key>",
    "endpoints": ["https://github.com/JobertQDomingo/JobertsVault/releases/latest/download/latest.json"],
    "requireSignedVersion": true,
    "windows": { "installMode": "passive" }
  }
}
```

**In the app**
- **At startup:** if `auto_update_enabled` is true, the frontend calls `check_update`, then `install_update` if there's a newer version. "Checking for updates…" appears only if the check takes more than 150 ms. Any failure is logged, and the app carries on (UPD-2).
- **Timeouts:** the check gives up after 3 seconds. The plugin's builder timeout also limits the download, so `install_update` must clear it (`update.timeout = None`) or a slow connection would fail every install.
- **Debug builds:** `check_update` returns nothing and `install_update` refuses, checked in Rust with `cfg!(debug_assertions)` (UPD-7).
- **Only at safe moments:** the app updates at startup or when the user clicks **Update now**, never in the background, so it can't restart while a transaction is being typed. Every write is a database transaction, so closing for the installer can't leave data half-saved.
- **The installer doesn't touch `%APPDATA%`.** The uninstaller's "delete the application data" checkbox doesn't apply during an update.

**The signing key**
- **Private key:** `~/.tauri/jobertsvault.key`, protected by a password and backed up off the PC. GitHub holds copies in the secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. It's never committed, and `*.key` is in `.gitignore`.
- **Public key:** `plugins.updater.pubkey` in `tauri.conf.json`. It's safe to share.
- **Local builds:** once `createUpdaterArtifacts` is on, a local `npm run tauri build` needs both variables set. `npm run tauri dev` doesn't.
- **If the key or its password is lost:** no installed copy can accept a new update. Make a new key, replace the public key and both secrets, release, and reinstall by hand once.
- **Never** run `tauri signer generate -f` on that path, because it replaces the key.

**Rehearsal:** any change to the updater is first tested with two signed builds of a separate "Rehearsal" app, served from `localhost`. It has its own `identifier`, so it has its own data folder and can't touch real data. RELEASING.md lists the tests: starting offline, automatic update, **Update now**, a swapped installer, a relabelled old release, and an update that brings a migration.

## 12. Performance targets (from the PRD)

| Target | How it's met |
|---|---|
| Cold start < 2 s | Small Rust binary, bundled SQLite, migrations only run when needed, screens are loaded only when first opened |
| Smooth glass UI | Blur only on panels and raised elements (about 12 or fewer on screen), never on list rows. Lists scroll inside one panel, and nothing is blurred when Windows transparency is off (DESIGN_SYSTEM §11). |
| Each view loads in < 300 ms with 50,000 transactions | Date and account indexes; totals calculated with SQL `SUM` (never adding up rows in JS); one IPC call for the dashboard; the transaction list loads in pages of 100 |

## 13. Coding conventions

- **Rust:** `rustfmt`, `clippy -D warnings`. No `unwrap()` outside tests. Money is always `Money` or `i64`, never `f64`.
- **TypeScript:** `strict` mode, ESLint + Prettier, no `any`. Components use `PascalCase`, and hooks start with `use`.
- **Naming:** `snake_case` in SQL and Rust, `camelCase` in TypeScript (converted by serde).
- **Commits:** short, written in the imperative mood ("Add card cycle calculation").

## 14. Decision records

The reasoning behind the big decisions lives in [docs/decisions/](decisions/README.md):

| # | Decision |
|---|---|
| [0001](decisions/0001-tauri-and-react.md) | Tauri 2 with React, rather than Electron |
| [0002](decisions/0002-money-in-minor-units.md) | Money stored as integers in minor units |
| [0003](decisions/0003-logic-in-rust.md) | Business logic in Rust, and the frontend never runs SQL |
| [0004](decisions/0004-rusqlite-single-connection.md) | rusqlite with a single connection, rather than sqlx with a connection pool |
| [0005](decisions/0005-balances-calculated-on-request.md) | Balances calculated when requested, never stored |
| [0006](decisions/0006-credit-card-accounting.md) | Credit card purchases counted as expenses when made, and bill payments as transfers |
| [0007](decisions/0007-glassmorphism-2.md) | Visual style: Glassmorphism 2.0, with the aurora painted by the app |
| [0008](decisions/0008-fixed-window-size.md) | A fixed 1440 × 900 window |
| [0009](decisions/0009-offline-only.md) | Offline only: no network access. Superseded by 0010. |
| [0010](decisions/0010-automatic-updates.md) | Automatic updates: the one exception to offline |
