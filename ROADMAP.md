# Roadmap: Jobert's Vault

| | |
|---|---|
| **Status** | Active |
| **Last updated** | 2026-11-12 |
| **Current focus** | **Phase 3: Planning** |
| **Related docs** | [PRD.md](docs/PRD.md) (requirements and IDs), [ARCHITECTURE.md](docs/ARCHITECTURE.md), [CLAUDE.md](CLAUDE.md) (definition of done) |

This file is the progress tracker.
- **Tick a box** (`[x]`) when its task meets the definition of done in CLAUDE.md.
- **IDs in brackets** are PRD requirements, such as `[TXN-1]`. Mention them in the commit message.
- **Each task is about one pull request.** Within a phase, work top to bottom: later tasks build on earlier ones.
- **Update "Current focus"** above when you move to the next milestone.

## Overview

| Phase | Goal | Version | Exit criteria | Progress |
|---|---|---|---|---|
| **0 · Setup** | An empty app that builds, runs and passes all checks | — | `npm run tauri dev` opens the fixed 1440 × 900 window, and every check in CLAUDE.md passes | 11 / 11 |
| **1 · MVP** | Log transactions and see cashflow by period | `v0.1.0` | Every item in PRD §10 is ticked on Windows 11, and an installed `v0.1.0` has updated itself | 53 / 53 |
| **2 · Reports & backups** | Understand spending, never lose data | `v0.2.0` | All P2 requirements done, a backup restored successfully | 14 / 14 |
| **3 · Planning** | Recurring transactions, budgets, goals | `v0.3.0` | All P3 requirements done | 13 / 16 |
| **4 · Polish** | Ready for daily use for years | `v1.0.0` | All P4 requirements done, installer tested on both monitors | 0 / 9 |

---

## Phase 0 · Project setup

- [x] Create the GitHub repository and commit the planning docs (`README.md`, `CLAUDE.md`, `ROADMAP.md`, `docs/`, `assets/brand/`)
- [x] Scaffold with `npm create tauri-app@latest` from a path with no apostrophe ([ARCHITECTURE §11](docs/ARCHITECTURE.md)): Tauri 2, React, TypeScript, Vite. Set `productName` to `Jobert’s Vault` (curly apostrophe) and `identifier` to `com.jobertsvault.app`. Remove `tauri-plugin-opener` and its permission if the template adds them.
- [x] Set up the window in `tauri.conf.json`: 1440 × 900, `resizable: false`, `maximizable: false`, `center: true`, `visible: false`. Also set the content security policy and keep `capabilities/default.json` to the minimum ([ARCHITECTURE §10](docs/ARCHITECTURE.md), [ADR 0008](docs/decisions/0008-fixed-window-size.md))
- [x] Frontend libraries: Tailwind CSS 4, shadcn/ui and its helpers, lucide-react, TanStack Query, React Router, react-hook-form, zod, `@hookform/resolvers`, `react-day-picker`, `date-fns`, `cmdk`, `sonner`, `@dnd-kit`, `@fontsource/sora` ([ARCHITECTURE §2](docs/ARCHITECTURE.md))
- [x] Rust crates: `rusqlite` `0.40` (with the `bundled` feature), `rusqlite_migration` `2.6`, `chrono`, `serde`, `thiserror`, `ts-rs`
- [x] Tooling: ESLint, Prettier, `rustfmt`, `clippy`, and the npm scripts `lint`, `typecheck`, `test`
- [x] Testing: set up Vitest, Testing Library and jsdom, plus one example test on each side (frontend and Rust)
- [x] Type generation: ts-rs writes to `src/bindings/` when `cargo test` runs, `i64` fields generate `number` ([ARCHITECTURE §4.6](docs/ARCHITECTURE.md)), and a check confirms the generated files are committed
- [x] App icon: `npm run tauri icon assets/brand/app-icon-1024.png`
- [x] CI: a GitHub Actions workflow on `windows-latest` that runs every check from CLAUDE.md and the bindings check on each push and pull request, without building the installer
- [x] Decision records 0001–0009 written ([docs/decisions/](docs/decisions/README.md))

---

## Phase 1 · MVP (`v0.1.0`)

### 1.0 How changes ship
The same process as NBSBuddy ([ARCHITECTURE §11.1](docs/ARCHITECTURE.md), [ADR 0010](docs/decisions/0010-automatic-updates.md)). This comes first because every later task ships through it.
- [x] Align `ci.yml`: run on pull requests and pushes to `main`, cancel older runs on the same branch, and name the job `check`, keeping all our checks
- [x] Set up GitHub: a **Protect Main** ruleset (PR required, `check` must pass, no force pushes or deleting), squash merging only, and delete branches on merge
- [x] Write the "every change" loop in `RELEASING.md`, make `package.json` the only version number (`"version": "../package.json"`), and set it to `0.0.0` so the first release is a real version bump

### 1.1 Backend foundations
- [x] `AppError`, `AppState` (one connection behind a `Mutex`), the error log ([ARCHITECTURE §4.4](docs/ARCHITECTURE.md)), and `api/client.ts` with its typed errors
- [x] Database startup: open the database (debug builds use `jobertsvault-dev.db`), apply the connection settings (PRAGMAs), refuse a database from a newer app version, and report any failure through `get_startup_status`, with `open_data_folder` ([ARCHITECTURE §6](docs/ARCHITECTURE.md))
- [x] Migration `0001_initial.sql`, exactly as in DATA_MODEL §3
- [x] Seed data: the Cash account, payment methods, and the preset categories with their colors (DATA_MODEL §7)
- [x] Back up an existing database before running pending migrations, and stop if the backup fails, with tests for both cases and for a new database, which skips it (DATA_MODEL §8) [BAK-5]
- [x] Domain: the `Money` type, plus parsing and formatting in Rust and in `money.ts`, with tests
- [x] Domain: periods (day, Monday-start week, month with `month_start_day`, calendar year, custom range), with previous/next and edge-case tests ([DATA_MODEL §5.3](docs/DATA_MODEL.md))
- [x] Domain: credit card statement cycle and next due date, with tests (DATA_MODEL §5.5)

### 1.2 Settings
- [x] Commands `get_settings` and `update_settings`: currency (₱ default), symbol position, decimals (locked once any amount is stored), week start, month start day, payment method field, `onboarding_completed`, `auto_update_enabled` [SET-1] [SET-2] [SET-6] [ONB-1] [UPD-3]

### 1.3 Accounts
- [x] Create, edit, list and reorder accounts, with opening balances of 0 or more [ACC-1] [ACC-2]
- [x] Credit card fields and validation [CC-1] [CC-8]
- [x] Balances calculated when requested, the net balance, and the card's amount owed, available credit, utilization, cycle spending and due date [ACC-4] [ACC-5] [CC-2] [CC-6]
- [x] Archive and unarchive, with a warning when the balance isn't 0. Delete unused accounts, and block archiving or deleting the last active one [ACC-3]

### 1.4 Categories and payment methods
- [x] Categories: list, create, edit (name, 12-color palette, curated icons), reorder, archive, unarchive, and block archiving the last active one of a kind [CAT-1] [CAT-2] [CAT-3]
- [x] Merge categories, and delete a category that has no transactions [CAT-4]
- [x] Payment methods: list, create, edit, reorder, archive, unarchive, delete unused [TXN-7]

### 1.5 Transactions
- [x] Create, edit and delete income, expense and transfer, with every validation rule from DATA_MODEL §6, including rejecting future dates [TXN-1] [TXN-2]
- [x] Duplicate: the form opens pre-filled with today's date, and saving creates the copy [TXN-2]
- [x] The list: grouped by day, filters, search, pages of 100, with the summary and day totals from the backend. An account's list includes incoming transfers [TXN-3] [TXN-4]
- [x] `get_transaction_defaults` and `get_pay_bill_defaults` ([DATA_MODEL §5.8](docs/DATA_MODEL.md)) [TXN-1] [CC-9]
- [x] Card purchases, bill payments and refunds behave as specified, with service tests, including "a card payment never changes the expense total" [CC-3] [CC-4] [CC-5] [RPT-6]

### 1.6 Dashboard backend
- [x] `get_period`, and `get_dashboard`: period, totals, balances, net balance, card summary, the 5 most recent transactions and `today`, all in one call [DSH-1] [DSH-2] [DSH-4] [DSH-5] [DSH-8]

### 1.7 Design system in code
- [x] Tokens in `globals.css` for light and dark, following the Windows setting, plus the `glass` and `glass-raised` utilities and the reduced-transparency fallback ([DESIGN_SYSTEM §2–3](docs/DESIGN_SYSTEM.md))
- [x] `AuroraBackground` and `GlassPanel`
- [x] Restyle the shadcn/ui basics: Button, Input, Select/Combobox, Dialog, Toast, Tabs, Segmented control, Date picker (follows `week_start`, US dates)
- [x] `MoneyText`, `AmountInput`, `CategoryChip`, `AccountRow`, `TransactionRow`, `StatCard`, `ProgressBar`
- [x] Empty states, skeletons (shown after 150 ms) and the startup error screen

### 1.8 App shell
- [x] Sidebar with the logo, top bar with + Add, routes, `PeriodSwitcher` with the period saved in the URL
- [x] TanStack Query hooks, where every money-related key starts with `['ledger', …]`, refreshed after every write
- [x] Shortcuts: `Ctrl+N`, `Ctrl+F`, `Ctrl+1`–`3`, `Ctrl+,`, `Alt+←/→`, `Alt+Home`

### 1.9 Screens
- [x] Dashboard ([UI_FLOWS §2](docs/UI_FLOWS.md))
- [x] Transactions screen with the filter toolbar ([UI_FLOWS §3](docs/UI_FLOWS.md))
- [x] Quick-add and edit dialog, including Save & add another and the keyboard path that saves in under 5 seconds [TXN-5]
- [x] Accounts list, new/edit account dialog, and account and credit card details. After **Delete** on the details view, remove that account's query (`queryKeys.account(id)`) and go back to `/accounts` before the ledger refresh, so the deleted account isn't fetched again and shown as "not found"
- [x] Pay bill: a transfer pre-filled with the full amount owed [CC-9]
- [x] Settings: the General, Categories and Payment methods tabs, with drag and keyboard reordering [SET-4]
- [x] First-launch setup form at `/welcome`: currency and first accounts, skippable [ONB-1]
- [x] Fixed-window fallback: open at 1280 × 800, zoomed to 88.9%, if 1440 × 900 doesn't fit the screen ([ADR 0008](docs/decisions/0008-fixed-window-size.md), [ARCHITECTURE §11](docs/ARCHITECTURE.md))

### 1.10 Automatic updates and releases
- [x] The owner generates the signing key, backs it up off this PC, and adds the two GitHub secrets. The public key goes in `tauri.conf.json` ([ARCHITECTURE §11.1](docs/ARCHITECTURE.md)) [UPD-4]
- [x] Updater backend: `tauri-plugin-updater` and its config, `check_update` and `install_update` with the 3-second check and no limit on the download, `get_app_version`, `AppError::Update`, and nothing happens in debug builds [UPD-1] [UPD-2] [UPD-4] [UPD-7]
- [x] Startup flow: "Checking for updates…" and "Updating Jobert's Vault to X…", and a failed check starts the app normally [UPD-1] [UPD-2]
- [x] Settings: the version, **Check for updates**, **Update now**, the automatic updates switch, and **Check for updates** on the startup error screen ([UI_FLOWS §6](docs/UI_FLOWS.md)) [UPD-3] [UPD-6]
- [x] `.github/workflows/release.yml`, adapted from NBSBuddy: every check from CI, plain ASCII asset names, and `JobertsVault-setup.exe`
- [x] The `/release` command (`.claude/skills/release/SKILL.md`) and the rest of `RELEASING.md`: a release, what to do when something goes wrong, the signing key, and the rehearsal. Leave out NBSBuddy's "What's new" check.
- [x] Rehearse on this PC with a separate app: starting offline, automatic update, **Update now**, a swapped installer (refused), a relabelled old release (refused), and an update that brings a migration (backup made, data intact) [UPD-1] [UPD-2] [UPD-3] [UPD-4] [UPD-5]

### 1.11 Release `v0.1.0`
- [x] Go through PRD §10 by hand on Windows 11, on both monitors
- [x] Check performance: cold start under 2 s, and the dashboard and transaction list under 300 ms with 50,000 test transactions. The glass must scroll smoothly.
- [x] `/release 0.1.0`, install it from the GitHub release, and confirm the data is in `%APPDATA%\com.jobertsvault.app\`
- [x] Prove the real path: `/release 0.1.1` with a small fix, and confirm the installed `v0.1.0` updates itself with all its data
- [x] Reminder until Phase 2: now and then, close the app and copy `jobertsvault.db`, `jobertsvault.db-wal` and `jobertsvault.db-shm` somewhere safe by hand

---

## Phase 2 · Reports & backups (`v0.2.0`)

### 2.1 Reports
- [x] Income vs expense chart over time, grouped by the period [RPT-1]
- [x] Category breakdown (8 largest plus Other) and the ranked list, with clicking through to the transactions [RPT-2] [RPT-3]
- [x] Period comparison with the "same days so far" rule and the comparison setting [RPT-4] [RPT-4a] [SET-7]
- [x] % change on the dashboard stat cards [DSH-3]
- [x] Account filter on reports [RPT-5]
- [x] Reports screen ([UI_FLOWS §8.1](docs/UI_FLOWS.md)), its sidebar item and the `Ctrl+4` shortcut
- [x] Payee suggestions that fill in the last category used [TXN-6]

### 2.2 Backups and export
- [x] Add `tauri-plugin-dialog`, scoped to folder and file pickers only
- [x] Back up now (`VACUUM INTO`, then rename the finished file) [BAK-1]
- [x] Daily automatic backup, keeping the newest 30 [BAK-2] [BAK-3]
- [x] Restore: check the file, make a safety copy, swap the database, reopen [BAK-4]
- [x] Export to CSV (UTF-8 with BOM) [BAK-6]
- [x] Backup settings tab, plus a warning when the last backup is more than 7 days old [BAK-7] [SET-3]
- [x] Release `v0.2.0` with `/release`, and test a real restore on a second Windows user account

---

## Phase 3 · Planning (`v0.3.0`)

- [x] Migration `0002_planning.sql` (DATA_MODEL §4)
- [x] Domain: recurrence dates calculated from `start_date`, with the end-of-month and Feb 29 tests (DATA_MODEL §4.1)
- [x] Recurring rules: create, edit, pause, delete, with auto or confirm mode [REC-1] [REC-2] [REC-3] [REC-5]
- [x] Process recurring rules at startup and when the date changes, never creating a date twice [REC-4]
- [x] Review due items: Post, Edit & post, Skip (UI_FLOWS F10)
- [x] Recurring screen
- [x] Future-dated transactions and the Upcoming section: lift the Phase 1 date rule, and keep them out of every total until their date arrives ([DATA_MODEL §5.2](docs/DATA_MODEL.md)) [TXN-8]
- [x] Budgets for a category or all spending, weekly, monthly or yearly [BUD-1] [BUD-4]
- [x] Budget progress with amber and red thresholds, the Budgets screen, and budgets on the dashboard [BUD-2] [BUD-3] [DSH-6]
- [x] Goals linked to an account, with progress and the monthly amount needed [GOL-1] [GOL-2] [GOL-3]
- [x] Mark goals achieved and archive them, plus the Goals screen [GOL-4]
- [x] Coming up in the next 7 days on the dashboard [DSH-7]
- [x] Card due-date banner with Pay bill [CC-7]
- [ ] The Recurring, Budgets and Goals items appear in the sidebar
- [ ] Recurring service tests: none missed and none duplicated after the app was closed for 3 months
- [ ] Release `v0.3.0` with `/release`

---

## Phase 4 · Polish (`v1.0.0`)

- [ ] Theme switch: system, light or dark [SET-5]
- [ ] Reduce transparency switch [SET-8]
- [ ] Polished onboarding: step indicator and a short tour [ONB-2]
- [ ] `?` opens the keyboard shortcut help dialog, and every shortcut is reviewed
- [ ] Accessibility pass: focus order, screen reader labels, contrast re-check of any changed tokens
- [ ] Performance pass on the glass UI with a full year of data
- [ ] Installer polish: icon, product name, version info. Test on both monitors.
- [ ] Review all docs against the shipped app, and update any that drifted
- [ ] Release `v1.0.0` with `/release`

---

## Not planned

The PRD §3 non-goals stay out of scope unless the PRD is changed first: bank sync, statement import, receipts, multiple users, cloud sync, multiple currencies, investments, tax, other platforms, and split transactions.
