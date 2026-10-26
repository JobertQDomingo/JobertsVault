# Architecture: Jobert's Vault

| | |
|---|---|
| **Status** | Approved v1.0 |
| **Last updated** | 2026-10-26 |
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
3. **No network.** No HTTP plugin and no updater are installed, and the content security policy (CSP) blocks all outgoing connections (§10).

## 2. Technology choices

| Layer | Choice | Why |
|---|---|---|
| App shell | **Tauri 2** | Small installer (about 10 MB), low memory use, Rust backend, uses the WebView2 engine that ships with Windows 10 and 11 |
| UI framework | **React 19 + TypeScript** | Largest library ecosystem (charts, forms, accessible components), and the best supported by AI coding tools |
| Build tool | **Vite** | Default for Tauri projects; fast reloading while developing |
| Styling | **Tailwind CSS 4** | Design tokens map directly to CSS variables. The style is Glassmorphism 2.0, built from `glass` / `glass-raised` utilities and `backdrop-filter` (see DESIGN_SYSTEM.md) |
| Font | **@fontsource/sora** (400, 600, 700) | The UI typeface, bundled into the app so no font is ever downloaded. It has tabular digits and includes ₱. |
| Components | **shadcn/ui** (built on Radix UI) | Accessible, keyboard-friendly building blocks that are copied into the repo, so they're ours to change |
| Icons | **lucide-react** | Matches the icon names stored on categories |
| Server state | **TanStack Query** | Caching, loading and error states, and refreshing data after changes |
| Routing | **React Router** | Simple routes, with the selected period kept in the URL |
| Forms | **react-hook-form + zod** | Fast, keyboard-friendly forms with type-safe validation |
| Charts (Phase 2) | **Recharts** | Built for React, and covers the bar, line and donut charts we need |
| Database driver | **rusqlite** (`bundled` feature) | SQLite is compiled into the app, so its version is always the same. It's synchronous, simple and fast. |
| Migrations | **rusqlite_migration** | Tracks the schema version in `PRAGMA user_version` |
| Dates | **chrono** | Date arithmetic and the local "today" |
| Errors | **thiserror** | A typed `AppError` enum |
| Serialization | **serde** | Converts data to and from JSON for IPC. Fields are renamed from `snake_case` to `camelCase` for the frontend. |
| Rust → TS types | **ts-rs** | Generates TypeScript types from Rust structs, so the two sides can't drift apart |
| File dialogs (Phase 2) | **tauri-plugin-dialog** | Folder and file pickers for backup and restore |

Exact versions are pinned in `Cargo.lock` and `package-lock.json`, and upgraded on purpose rather than automatically.

## 3. Repository layout

```
jobertsvault/
├─ README.md  CLAUDE.md  ROADMAP.md  CHANGELOG.md
├─ docs/                           # PRD, DATA_MODEL, ARCHITECTURE, DESIGN_SYSTEM, UI_FLOWS
│  ├─ decisions/                   # decision records 0001–0009 (see docs/decisions/README.md)
│  └─ design/                      # visual previews of the design system and UI flows
├─ assets/brand/                   # logo.svg, one-color marks, app-icon-1024.png
├─ package.json  vite.config.ts  tsconfig.json
├─ src/                            # ── Frontend ──
│  ├─ main.tsx  App.tsx  routes.tsx
│  ├─ api/                         # one typed wrapper per Tauri command (the only place that calls invoke)
│  │  ├─ accounts.ts  transactions.ts  categories.ts  dashboard.ts  settings.ts
│  │  └─ client.ts                 # invoke wrapper + AppError handling
│  ├─ bindings/                    # ⚙ generated by ts-rs; never edit by hand
│  ├─ hooks/                       # useAccounts, useTransactions, useDashboard, … (TanStack Query)
│  ├─ features/                    # screens, one folder each
│  │  ├─ dashboard/  transactions/  accounts/  categories/  settings/
│  │  ├─ reports/ (P2)  recurring/ (P3)  budgets/ (P3)  goals/ (P3)
│  ├─ components/
│  │  ├─ ui/                       # shadcn/ui building blocks
│  │  └─ MoneyText.tsx  AmountInput.tsx  PeriodSwitcher.tsx  QuickAddDialog.tsx …
│  ├─ lib/
│  │  ├─ money.ts                  # parse and format minor units (the only place that does this)
│  │  ├─ dates.ts                  # formatting 'YYYY-MM-DD' for display only
│  │  └─ queryKeys.ts
│  └─ styles/globals.css           # design tokens (CSS variables)
└─ src-tauri/                      # ── Backend ──
   ├─ Cargo.toml  tauri.conf.json
   ├─ capabilities/default.json    # the minimum permissions the frontend needs
   ├─ migrations/0001_initial.sql
   └─ src/
      ├─ main.rs                   # entry point
      ├─ lib.rs                    # builds the app, registers commands, startup steps
      ├─ state.rs                  # AppState { db: Mutex<Connection>, … }
      ├─ error.rs                  # AppError, and how it's serialized
      ├─ db/  mod.rs  migrations.rs  seed.rs
      ├─ domain/                   # pure, unit-tested, no database
      │  ├─ money.rs  period.rs  comparison.rs  card_cycle.rs  recurrence.rs (P3)
      ├─ repo/                     # SQL only, one module per table
      │  ├─ accounts.rs  categories.rs  payment_methods.rs  transactions.rs  settings.rs
      ├─ services/                 # rules + database transactions
      │  ├─ accounts.rs  categories.rs  transactions.rs  dashboard.rs  settings.rs
      │  ├─ reports.rs (P2)  backup.rs (P2)  export.rs (P2)  recurring.rs (P3)  budgets.rs (P3)  goals.rs (P3)
      └─ commands/                 # #[tauri::command] wrappers, one module per area
```

## 4. Backend design

### 4.1 Layers

| Layer | Allowed to | Not allowed to |
|---|---|---|
| `commands` | Convert input data (DTOs), call one service function, return `Result<T, AppError>` | Contain business rules or SQL |
| `services` | Validate input, open a database transaction, call the repo and domain layers, build response DTOs | Contain raw SQL text or anything Tauri-specific |
| `domain` | Do pure calculations on plain values | Do any input/output: no database, files or clock (the date `today` is passed in) |
| `repo` | Run parameterized SQL and map rows to structs | Contain business rules |

`domain` and `services` never read the system clock directly. "Today" is passed in as a value, so tests can fix the date. The command layer gets it from `chrono::Local::now().date_naive()`.

### 4.2 Key domain types

```rust
pub struct Money(i64);                 // minor units; checked arithmetic, never f64
pub enum PeriodKind { Day, Week, Month, Year, Custom }
pub struct Period { kind: PeriodKind, start: NaiveDate, end: NaiveDate }   // inclusive
impl Period {
    fn containing(kind, anchor, &PeriodSettings) -> Period;   // DATA_MODEL §5.3
    fn previous(&self) -> Period;  fn next(&self) -> Period;
}
fn comparison_window(current: &Period, today: NaiveDate, mode: ComparisonMode) -> Period; // §5.4
fn card_cycle(statement_day: u8, due_day: u8, today: NaiveDate) -> CardCycle;             // §5.5
```

### 4.3 Database access

- **One SQLite connection** stored in `AppState` behind a `Mutex`. With a single user and queries that take milliseconds, this is simpler than a connection pool and avoids write conflicts.
- Tauri runs commands on background threads, so database work never blocks the window.
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
}
```

The frontend's `api/client.ts` converts these into a typed `ApiError`:
- `validation` errors are shown on the matching form field (via react-hook-form's `setError`).
- Other errors are shown as a toast notification.

### 4.5 Command catalog

All commands use `camelCase` parameter names in TypeScript and `snake_case` in Rust. Tauri converts between them automatically.

| Phase | Area | Commands |
|---|---|---|
| 1 | Accounts | `list_accounts`, `get_account`, `create_account`, `update_account`, `archive_account`, `reorder_accounts` |
| 1 | Categories | `list_categories`, `create_category`, `update_category`, `archive_category`, `merge_categories` |
| 1 | Payment methods | `list_payment_methods`, `create_payment_method`, `update_payment_method`, `archive_payment_method` |
| 1 | Transactions | `list_transactions(filter, page)`, `get_transaction`, `create_transaction`, `update_transaction`, `delete_transaction`, `duplicate_transaction` |
| 1 | Dashboard | `get_dashboard(period_kind, anchor)` → period, totals, balances, net balance, card summary, recent transactions, `today` |
| 1 | Settings | `get_settings`, `update_settings` |
| 2 | Reports | `get_timeseries`, `get_category_breakdown`, `get_comparison`, `suggest_payees(prefix)` |
| 2 | Backup | `backup_now`, `list_backups`, `restore_backup(path)`, `export_csv(range, path)` |
| 3 | Planning | `list/create/update/pause/delete_recurring_rule`, `list_due_occurrences`, `post_occurrence`, `skip_occurrence`, budget CRUD + `get_budget_progress`, goal CRUD + `get_goal_progress` |

**Dashboard data comes from one command.** `get_dashboard` returns everything the dashboard needs in a single response, so the screen loads in one round trip and every number on it is consistent with the others.

### 4.6 Rust ↔ TypeScript types

- Every struct that crosses IPC derives `Serialize/Deserialize` and `ts_rs::TS` with `#[serde(rename_all = "camelCase")]`.
- Running `cargo test` regenerates the files in `src/bindings/`. These files are committed to git, and CI fails if they're out of date.
- **Money crosses IPC as a plain integer of minor units.** JavaScript numbers are exact up to 2⁵³ (about 9 × 10¹⁵), far above the app's maximum amount (DATA_MODEL §6).
- **Dates cross IPC as `'YYYY-MM-DD'` strings**, never as JavaScript `Date` objects, which can shift a date across midnight because of time zones.

## 5. Frontend design

### 5.1 State

| What | Where |
|---|---|
| Data from the backend (accounts, transactions, totals, settings) | TanStack Query cache |
| The selected period and filters | URL search parameters, e.g. `/?period=month&anchor=2026-10-26`, so the back button and reloads work |
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
| `/reports` | Reports | 2 |
| `/settings/backup` | Backup and restore | 2 |
| `/recurring`, `/budgets`, `/goals` | Planning | 3 |

Quick-add (**Ctrl+N**) is a dialog that can be opened from any route. It isn't a page of its own.

## 6. Startup sequence

```
1. Resolve the app data folder           (%APPDATA%\com.jobertsvault.app\)
2. Open jobertsvault.db, apply the connection settings
3. Check user_version  ── newer than the app knows? ─▶ error screen, stop
4. [P2] If migrations are pending: back up the database first (BAK-5)
5. Run migrations
6. If the database is new: add the seed data (DATA_MODEL §7) → show onboarding
7. [P2] Daily auto-backup, if not done yet today (BAK-2), then delete old ones (BAK-3)
8. [P3] Process recurring rules due up to today (REC-4)
9. Store AppState and show the main window
```

If any step fails, the app shows an error screen with the database path and the error message, plus an "Open data folder" button. It never shows a blank window.

## 7. Backup and restore (Phase 2)

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

**Example tests that must exist:** "Paying the card bill doesn't change the month's expense total", "Week of 2026-10-26 (a Monday) is Oct 26 to Nov 1", "Month start day 25: Oct 2026 is Sep 25 to Oct 24", "Comparing Oct 1–26 uses Sep 1–26".

**CI (optional, GitHub Actions on `windows-latest`):** `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, a check that the bindings are up to date, `npm run lint`, `npm test` and `npm run tauri build`.

## 10. Security and privacy

- **Tauri capabilities:** the frontend is granted only the core window permissions and, from Phase 2, the dialog plugin. There's **no** `fs`, `shell` or `http` plugin. All file work happens in Rust, which validates the paths it's given.
- **Content security policy:** `default-src 'self'; connect-src ipc: http://ipc.localhost; img-src 'self' data:; style-src 'self' 'unsafe-inline'`. With this, the UI can't load anything from the internet.
- **No telemetry, crash reporting or auto-updater.**
- **The data is not encrypted.** It's protected by your Windows account and disk encryption, such as BitLocker. Encrypting the database could be added later (e.g. with SQLCipher) if needed. This is recorded as a known limitation.

## 11. Build and distribution

- `npm run tauri dev` starts the app for development, with hot reload.
- **App icon:** `npm run tauri icon assets/brand/app-icon-1024.png` generates `src-tauri/icons/` (the `.ico` and every size) from the logo (DESIGN_SYSTEM §6.1).
- `npm run tauri build` produces an **NSIS `.exe` installer** (recommended for a single user, as it doesn't need administrator rights) and, optionally, an `.msi`.
- `productName`: `Jobert's Vault` · `identifier`: `com.jobertsvault.app` · WebView2 is installed by the installer if it's missing.
- **Window:** a normal opaque window. No `windowEffects` (Mica or Acrylic) and no transparent window. The app paints its own aurora background (DESIGN_SYSTEM §2.2), so the desktop wallpaper never affects readability.
- **Fixed size:** `tauri.conf.json` sets the window to `"width": 1440, "height": 900, "resizable": false, "maximizable": false, "center": true`. At startup, Rust checks the current monitor's work area (`current_monitor()?.work_area()`). If 1440 × 900 plus the title bar doesn't fit, it switches to 1280 × 800 before showing the window. No window-state plugin is needed.
- **Updates are manual:** download and run the new installer. Your data is kept because it's in `%APPDATA%`, not the install folder.
- **Code signing:** the installer isn't signed, so Windows SmartScreen shows a "More info → Run anyway" warning. That's acceptable for a personal app.

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
| [0009](decisions/0009-offline-only.md) | Offline only: no network access |
