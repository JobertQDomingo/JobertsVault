# Data Model: Jobert's Vault

| | |
|---|---|
| **Status** | Approved v1.2 |
| **Last updated** | 2026-11-13 |
| **Database** | SQLite (bundled with the app through `rusqlite`) |
| **Related docs** | [PRD.md](PRD.md), [ARCHITECTURE.md](ARCHITECTURE.md) |

This document describes every table and field, plus the rules for calculating balances, totals and periods. **Any change to the database schema must update this doc first, then add a new migration.**

---

## 1. Global conventions

| Topic | Rule |
|---|---|
| **Money** | Stored as `INTEGER` in **minor units** (e.g. cents), as an `i64` in Rust. Floating-point numbers are never used for money, anywhere. |
| **Amount sign** | `transactions.amount` is always **positive**. The transaction's `kind` decides whether money goes in or out. |
| **Decimal places** | `currency_decimals` (0 or 2) decides how minor units are displayed. It **can only be changed while no amount is stored**: no transactions, no recurring rules, no budgets, no goals, no loans, and no account with an opening balance other than 0 or a credit limit. Changing it afterwards would change the meaning of every stored amount. |
| **Dates** | `TEXT` in `YYYY-MM-DD` format (ISO 8601), using the user's local calendar. This format sorts correctly as plain text. No time of day is stored. |
| **Timestamps** | `created_at` / `updated_at` are `TEXT` in ISO 8601 UTC (`2026-10-26T09:30:00Z`). They're for record-keeping only and are never used in calculations. Services receive the current time as a `now` value, the same way they receive `today`. |
| **IDs** | `INTEGER PRIMARY KEY`, assigned by SQLite. |
| **Booleans** | `INTEGER` 0 or 1, with a `CHECK` constraint. |
| **Enums** | `TEXT` with a `CHECK (... IN (...))` constraint. The values are lowercase `snake_case`. |
| **Table mode** | Every table is declared `STRICT`, so SQLite rejects a value of the wrong type. |
| **Connection settings** | `PRAGMA foreign_keys = ON`, `PRAGMA journal_mode = WAL`, `PRAGMA busy_timeout = 5000`, `PRAGMA synchronous = NORMAL` (as in ARCHITECTURE §4.3) |
| **Deleting records** | Accounts, categories and payment methods that are in use are **archived instead of deleted**. Unused ones can be deleted. Foreign keys use `ON DELETE RESTRICT`, so the database itself blocks a delete that would break history. A loan follows its own rule (§4.5). |
| **Names** | Names are trimmed and compared case-insensitively (`COLLATE NOCASE`). |

## 2. Entity overview

```
accounts 1───* transactions *───1 categories
    │  1            │ *
    │               └───0..1 payment_methods
    │ 1
    └──* transactions.to_account_id   (incoming transfers)

recurring_rules 1───* recurring_occurrences 0..1───1 transactions   (Phase 3)
categories 1───* budgets                                            (Phase 3)
accounts 1───* goals                                                (Phase 3)
loans 1───* loan_payments 0..1───1 transactions                     (Phase 4)
accounts 1───* loans *───1 categories                               (Phase 4)
transactions.payslip_id ───▶ the payslip's gross row (transactions)  (Phase 4)
settings  (a key/value table)
```

| Table | Phase | Purpose |
|---|---|---|
| `accounts` | 1 | Cash, bank, e-wallet and credit card accounts |
| `categories` | 1 | Income and expense categories |
| `payment_methods` | 1 | The editable list of payment methods |
| `transactions` | 1 | Every income, expense and transfer |
| `settings` | 1 | User preferences and app state |
| `recurring_rules` | 3 | Schedules plus a transaction template |
| `recurring_occurrences` | 3 | One row per due date of a rule, so a date is never created twice |
| `budgets` | 3 | Spending limits |
| `goals` | 3 | Savings goals |
| `loans` | 4 | Loans the user repays on a schedule |
| `loan_payments` | 4 | One row per scheduled payment of a loan |

Phase 4 also adds two columns: `transactions.payslip_id` (§4.7) and `recurring_rules.is_payday` (§4.8).

---

## 3. Tables: Phase 1 (migration `0001_initial.sql`)

### 3.1 `accounts`

| Column | Type | Null | Notes |
|---|---|---|---|
| `id` | INTEGER | no | Primary key |
| `name` | TEXT | no | Unique (case-insensitive), 1–50 characters |
| `kind` | TEXT | no | `cash` \| `bank` \| `ewallet` \| `credit_card` |
| `opening_balance` | INTEGER | no | Signed, in minor units. Forms accept only 0 or more (§6). **For a credit card, the amount owed is stored as a negative number** (see §5.1), and every other account stores 0 or more. Default 0. |
| `credit_limit` | INTEGER | card only | > 0. Must be NULL for every kind except a credit card. |
| `statement_day` | INTEGER | card only | 1–31. If the month is shorter, the last day of the month is used. |
| `due_day` | INTEGER | card only | 1–31. Same rule for shorter months. |
| `sort_order` | INTEGER | no | Display order. Default 0. |
| `archived` | INTEGER | no | 0 or 1 |
| `created_at`, `updated_at` | TEXT | no | |

```sql
CREATE TABLE accounts (
  id              INTEGER PRIMARY KEY,
  name            TEXT    NOT NULL COLLATE NOCASE UNIQUE,
  kind            TEXT    NOT NULL CHECK (kind IN ('cash','bank','ewallet','credit_card')),
  opening_balance INTEGER NOT NULL DEFAULT 0,
  credit_limit    INTEGER CHECK (credit_limit > 0),
  statement_day   INTEGER CHECK (statement_day BETWEEN 1 AND 31),
  due_day         INTEGER CHECK (due_day BETWEEN 1 AND 31),
  sort_order      INTEGER NOT NULL DEFAULT 0,
  archived        INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0,1)),
  created_at      TEXT    NOT NULL,
  updated_at      TEXT    NOT NULL,
  CHECK (
    (kind = 'credit_card' AND credit_limit IS NOT NULL
                          AND statement_day IS NOT NULL
                          AND due_day IS NOT NULL)
    OR
    (kind <> 'credit_card' AND credit_limit IS NULL
                           AND statement_day IS NULL
                           AND due_day IS NULL)
  )
) STRICT;
```

### 3.2 `categories`

| Column | Type | Null | Notes |
|---|---|---|---|
| `id` | INTEGER | no | Primary key |
| `name` | TEXT | no | 1–40 characters. Unique within its kind. |
| `kind` | TEXT | no | `income` \| `expense` |
| `color` | TEXT | no | A **palette key** such as `cat-3`, not a hex code, so the color adapts to light and dark mode. The palette is defined in DESIGN_SYSTEM.md. |
| `icon` | TEXT | no | Name of a [Lucide](https://lucide.dev) icon from the curated list (DESIGN_SYSTEM §6), e.g. `shopping-cart` |
| `sort_order` | INTEGER | no | Default 0 |
| `archived` | INTEGER | no | 0 or 1 |
| `created_at`, `updated_at` | TEXT | no | |

```sql
CREATE TABLE categories (
  id         INTEGER PRIMARY KEY,
  name       TEXT    NOT NULL COLLATE NOCASE,
  kind       TEXT    NOT NULL CHECK (kind IN ('income','expense')),
  color      TEXT    NOT NULL,
  icon       TEXT    NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  archived   INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0,1)),
  created_at TEXT    NOT NULL,
  updated_at TEXT    NOT NULL,
  UNIQUE (kind, name)
) STRICT;
```

**Merging (CAT-4):** in one database transaction, move every transaction, recurring rule and loan (§4.5) from category A to active category B (both must be the same kind), then delete A. A's budgets are deleted with it (§4.3).

### 3.3 `payment_methods`

```sql
CREATE TABLE payment_methods (
  id         INTEGER PRIMARY KEY,
  name       TEXT    NOT NULL COLLATE NOCASE UNIQUE,
  sort_order INTEGER NOT NULL DEFAULT 0,
  archived   INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0,1)),
  created_at TEXT    NOT NULL,
  updated_at TEXT    NOT NULL
) STRICT;
```

### 3.4 `transactions`

| Column | Type | Null | Notes |
|---|---|---|---|
| `id` | INTEGER | no | Primary key. It also sets the order of transactions entered on the same date. |
| `kind` | TEXT | no | `income` \| `expense` \| `transfer` |
| `amount` | INTEGER | no | **> 0**, in minor units |
| `date` | TEXT | no | `YYYY-MM-DD` |
| `account_id` | INTEGER | no | The account money goes into or out of. For a transfer, this is the **source** account. |
| `to_account_id` | INTEGER | transfer only | The **destination** account. It must differ from `account_id`. |
| `category_id` | INTEGER | income/expense only | Must be NULL for a transfer. The category's kind must match the transaction's kind. |
| `payee` | TEXT | yes | Who was paid or who paid. 0–100 characters. Must be NULL for a transfer. |
| `payment_method_id` | INTEGER | yes | Optional (TXN-7). Must be NULL for a transfer. |
| `note` | TEXT | yes | 0–500 characters |
| `created_at`, `updated_at` | TEXT | no | |
| `payslip_id` | INTEGER | yes | Phase 4: the gross row of the payslip this row belongs to (§4.7), or NULL |

```sql
CREATE TABLE transactions (
  id                INTEGER PRIMARY KEY,
  kind              TEXT    NOT NULL CHECK (kind IN ('income','expense','transfer')),
  amount            INTEGER NOT NULL CHECK (amount > 0),
  date              TEXT    NOT NULL CHECK (date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
  account_id        INTEGER NOT NULL REFERENCES accounts(id)        ON DELETE RESTRICT,
  to_account_id     INTEGER          REFERENCES accounts(id)        ON DELETE RESTRICT,
  category_id       INTEGER          REFERENCES categories(id)      ON DELETE RESTRICT,
  payee             TEXT,
  payment_method_id INTEGER          REFERENCES payment_methods(id) ON DELETE RESTRICT,
  note              TEXT,
  created_at        TEXT    NOT NULL,
  updated_at        TEXT    NOT NULL,
  CHECK (
    (kind = 'transfer' AND to_account_id IS NOT NULL
                       AND to_account_id <> account_id
                       AND category_id IS NULL
                       AND payee IS NULL
                       AND payment_method_id IS NULL)
    OR
    (kind <> 'transfer' AND to_account_id IS NULL
                        AND category_id IS NOT NULL)
  )
) STRICT;

CREATE INDEX idx_txn_date          ON transactions(date);
CREATE INDEX idx_txn_account_date  ON transactions(account_id, date);
CREATE INDEX idx_txn_to_account    ON transactions(to_account_id, date) WHERE to_account_id IS NOT NULL;
CREATE INDEX idx_txn_category_date ON transactions(category_id, date)   WHERE category_id IS NOT NULL;
CREATE INDEX idx_txn_payee         ON transactions(payee COLLATE NOCASE) WHERE payee IS NOT NULL;
```

**List order (TXN-3):** `ORDER BY date DESC, id DESC`.

**Account filter on the list (TXN-4):** a transaction matches when `account_id = account OR to_account_id = account`, so an account's list includes incoming transfers.

**Payees** don't have their own table. Suggestions (TXN-6, Phase 2) come from the distinct `payee` values on rows of the form's type (expense or income), sorted by most recent use (`date DESC, id DESC`), counting only rows dated today or earlier: a future-dated row (TXN-8) isn't a use yet. Each payee is suggested once, ignoring ASCII case and spelled as it was last used, and only when it starts with what's typed (ASCII case ignored, wildcards taken literally). At most 8 are suggested. The suggested category is the one on the latest such row of that type with that payee. If that category is archived now, the payee is still suggested but fills in no category.

### 3.5 `settings`

A key/value table. The Rust backend reads it into a typed `Settings` struct and fills in a default for any key that's missing.

```sql
CREATE TABLE settings (
  key   TEXT NOT NULL PRIMARY KEY,
  value TEXT NOT NULL
) STRICT;
```

| Key | Values | Default | Phase | PRD |
|---|---|---|---|---|
| `currency_symbol` | text, 1–5 characters | `₱` (Philippine peso, U+20B1) | 1 | SET-1 |
| `currency_position` | `before` \| `after` | `before` | 1 | SET-1 |
| `currency_decimals` | `0` \| `2` | `2` | 1 | SET-1 |
| `week_start` | `monday` \| `sunday` | `monday` | 1 | SET-2 |
| `month_start_day` | `1`–`28` | `1` | 1 | SET-2 |
| `show_payment_method` | `true` \| `false` | `true` | 1 | SET-6 |
| `last_used_account_id` | account id | (none) | 1 | TXN-1 |
| `onboarding_completed` | `true` \| `false` | `false` | 1 | ONB-1 |
| `auto_update_enabled` | `true` \| `false` | `true` | 1 | UPD-3 |
| `comparison_mode` | `to_date` \| `full` | `to_date` | 2 | SET-7 |
| `backup_folder` | absolute path, stored only when the user chose a folder | (none: `<app data>\backups`, or `backups-dev` in debug builds) | 2 | SET-3 |
| `auto_backup_enabled` | `true` \| `false` | `true` | 2 | SET-3 |
| `last_backup_at` | UTC timestamp of the last successful manual or automatic backup | (none) | 2 | BAK-7 |
| `last_auto_backup_date` | `YYYY-MM-DD`, the local date of the last successful automatic backup | (none) | 2 | BAK-2 |
| `theme` | `system` \| `light` \| `dark` | `system` | 5 | SET-5 |
| `reduce_transparency` | `true` \| `false` | `false` | 5 | SET-8 |

`month_start_day` is capped at 28 so that every month contains that day.

`backup_folder`, `auto_backup_enabled`, `last_backup_at` and `last_auto_backup_date` describe this computer's backups rather than the data, so a restore keeps the current values (§9).

`onboarding_completed` becomes `true` when the user chooses **Start using the app** or **Set up later**. Closing the app before that shows the form again on the next launch.

---

## 4. Tables: Phase 3 (migration `0002_planning.sql`) and Phase 4 (migration `0003_loans_payroll.sql`)

§4.1–4.4 are Phase 3. §4.5–4.8 are Phase 4.

### 4.1 `recurring_rules`

A rule holds a **transaction template** (the same fields as `transactions`) plus a schedule.

```sql
CREATE TABLE recurring_rules (
  id                INTEGER PRIMARY KEY,
  -- template
  kind              TEXT    NOT NULL CHECK (kind IN ('income','expense','transfer')),
  amount            INTEGER NOT NULL CHECK (amount > 0),
  account_id        INTEGER NOT NULL REFERENCES accounts(id)        ON DELETE RESTRICT,
  to_account_id     INTEGER          REFERENCES accounts(id)        ON DELETE RESTRICT,
  category_id       INTEGER          REFERENCES categories(id)      ON DELETE RESTRICT,
  payee             TEXT,
  payment_method_id INTEGER          REFERENCES payment_methods(id) ON DELETE RESTRICT,
  note              TEXT,
  -- schedule
  frequency         TEXT    NOT NULL CHECK (frequency IN ('daily','weekly','monthly','yearly')),
  interval          INTEGER NOT NULL DEFAULT 1 CHECK (interval >= 1),
  start_date        TEXT    NOT NULL,
  end_date          TEXT,
  max_occurrences   INTEGER CHECK (max_occurrences >= 1),
  next_index        INTEGER NOT NULL DEFAULT 0,  -- index of the next occurrence not yet processed
  mode              TEXT    NOT NULL CHECK (mode IN ('auto','confirm')),
  paused            INTEGER NOT NULL DEFAULT 0 CHECK (paused IN (0,1)),
  created_at        TEXT    NOT NULL,
  updated_at        TEXT    NOT NULL,
  CHECK (
    (kind = 'transfer' AND to_account_id IS NOT NULL AND to_account_id <> account_id AND category_id IS NULL
                       AND payee IS NULL AND payment_method_id IS NULL)
    OR (kind <> 'transfer' AND to_account_id IS NULL AND category_id IS NOT NULL)
  )
) STRICT;
```

**Calculating occurrence dates:** the *n*-th date is always calculated **from `start_date`**, never from the previous occurrence. This stops dates drifting: a rule for the 31st gives Jan 31, Feb 28, Mar 31, not Jan 31, Feb 28, Mar 28.

| Frequency | Occurrence *n* |
|---|---|
| daily | `start_date + n × interval` days |
| weekly | `start_date + n × interval × 7` days |
| monthly | `start_date + n × interval` months, with the day clamped to the month's last day (REC-2) |
| yearly | `start_date + n × interval` years. Feb 29 becomes Feb 28 in non-leap years. |

**When a rule ends:** after its `end_date` (included), or once it has `max_occurrences` rows in `recurring_occurrences`. The count is of dates actually processed (posted, pending or skipped), so it holds across schedule edits, and dates passed over while paused don't count.

**Start date:** any real date, past or future. Processing (ARCHITECTURE §8) posts or queues every date from `start_date` up to today, so a past start fills in the dates already gone by.

**Editing a rule (REC-5):**
- Template fields (amount, category, mode, `end_date`, `max_occurrences`, and so on) affect future occurrences only. So a rule that had ended and is given a later end date or more times carries on from its first date on or after today: the dates in between are never created.
- The schedule changes when `frequency`, `interval` or `start_date` changes. The new `start_date` comes from the form, which offers the rule's next date. It must be after the last date already processed, and not before today or the rule's current next date, whichever is earlier, so the dates passed over while paused are never created and an edit never fills in the past. `next_index` is reset to 0. Because the user picks the date, a rule on the 31st whose next date is Feb 28 can be started again on Mar 31, so it doesn't drift to the 28th.

**Coming up (DSH-7):** the dashboard lists every date an active rule falls on after today and up to 7 days on (today Nov 12: Nov 13–19), soonest first. Dates up to today are already posted or in the Due list. A paused rule has none, and dates still to be processed up to today count toward `max_occurrences`, as processing will count them. Only rules are listed, not one-off future-dated transactions (TXN-8). A payday recorded early (§4.8) already has its occurrence row, so its date isn't listed.

**Pausing (REC-5):** a paused rule isn't processed. When it's resumed, `next_index` moves to the first date on or after the day it's resumed. The dates it would have had while paused are never created.

### 4.2 `recurring_occurrences`

There is one row for every due date that has been processed. The unique key means **the same date can never be created twice** (REC-4).

```sql
CREATE TABLE recurring_occurrences (
  id              INTEGER PRIMARY KEY,
  rule_id         INTEGER NOT NULL REFERENCES recurring_rules(id) ON DELETE CASCADE,
  occurrence_date TEXT    NOT NULL,
  status          TEXT    NOT NULL CHECK (status IN ('pending','posted','skipped')),
  transaction_id  INTEGER REFERENCES transactions(id) ON DELETE SET NULL,
  created_at      TEXT    NOT NULL,
  updated_at      TEXT    NOT NULL,
  UNIQUE (rule_id, occurrence_date)
) STRICT;
```

- **Auto mode:** a transaction is created and the row is inserted with status `posted`, both in one database transaction. The transaction is the template with the occurrence's date, and it keeps the template's account, category and payment method even if they've since been archived, the same way an edit of the rule may keep them (§9).
- **Confirm mode:** the row is inserted as `pending`. When the user approves it (Post, or Edit & post, UI_FLOWS F10), it becomes `posted` and gets a transaction, in one database transaction. When the user skips it, it becomes `skipped`. Only a `pending` row can be posted or skipped.
  - **Post** creates the rule's template as it is when posted, dated the day it fell due: no copy of the template is kept per date, so a template edit also changes the dates still waiting.
  - **Edit & post** creates the edited fields instead, checked like any new transaction (§6), except that it may keep the template's archived account, category or payment method. It changes only that date, never the rule. The row keeps its `occurrence_date` even when the transaction gets another date.
- Deleting a posted transaction keeps the occurrence row, so the app doesn't create it again.
- Deleting a rule deletes its occurrence rows but **keeps** the transactions it created.

### 4.3 `budgets`

```sql
CREATE TABLE budgets (
  id          INTEGER PRIMARY KEY,
  category_id INTEGER REFERENCES categories(id) ON DELETE RESTRICT,  -- NULL = all spending
  period      TEXT    NOT NULL CHECK (period IN ('weekly','monthly','yearly')),
  amount      INTEGER NOT NULL CHECK (amount > 0),
  archived    INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0,1)),
  created_at  TEXT    NOT NULL,
  updated_at  TEXT    NOT NULL
) STRICT;

CREATE UNIQUE INDEX uq_budget_category ON budgets(category_id, period) WHERE category_id IS NOT NULL;
CREATE UNIQUE INDEX uq_budget_overall  ON budgets(period)              WHERE category_id IS NULL;
```

- The category must be an expense category. This is checked in Rust.
- One budget per category and period, and one per period for all spending. Every field can be edited, including the category and the period.
- **Budgets are edited or deleted, never archived.** Nothing refers to a budget, so it can always be deleted, and `archived` stays 0.
- **Deleting or merging a category deletes its budgets**, in the same database transaction. They don't move to the merge target, whose own limit is what the user set for it. A budget doesn't count as using a category, so it doesn't stop the category being deleted.
- A budget's amount locks `currency_decimals` (§1).
- A budget has no history of past amounts: if its amount changes, past periods are also measured against the new amount. That's acceptable because budgets never roll over (BUD-4).

### 4.4 `goals`

```sql
CREATE TABLE goals (
  id            INTEGER PRIMARY KEY,
  name          TEXT    NOT NULL COLLATE NOCASE UNIQUE,
  target_amount INTEGER NOT NULL CHECK (target_amount > 0),
  target_date   TEXT,
  account_id    INTEGER NOT NULL REFERENCES accounts(id) ON DELETE RESTRICT,
  achieved_at   TEXT,
  archived      INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0,1)),
  created_at    TEXT    NOT NULL,
  updated_at    TEXT    NOT NULL
) STRICT;
```

- The linked account can't be a credit card. This is checked in Rust. A new goal can't use an archived account, but an edit may keep the one it has, and a goal on an archived account still shows its progress.
- **A goal puts its account in use**, like a transaction or a recurring rule, so that account can only be archived, never deleted, while the goal exists (`ON DELETE RESTRICT`).
- Several goals may share an account, and each one counts the account's whole balance.
- `target_date` is optional and may already have passed. Every field can be edited, and nothing refers to a goal, so it can always be deleted.
- A goal's target amount locks `currency_decimals` (§1).
- `achieved_at` is set the first time the account's balance reaches the target amount (GOL-4). `list_goals` checks this each time it runs, in a database transaction, and stores today's date (`YYYY-MM-DD`). So it's the day Goals first found it reached, which can be later than the day the balance got there. It stays if the balance drops later. Raising the target amount or changing the account clears it, so the goal is checked again; lowering the target on the same account keeps it. An archived goal isn't checked, since it may have been given up. An achieved goal asks for no monthly amount (§5.7).
- **Any goal can be archived** (GOL-4), achieved or not, and unarchived. An archived goal still shows its progress and can still be edited or deleted.

### 4.5 `loans` (Phase 4)

A loan is a **list of scheduled payments**, not an account ([ADR 0011](decisions/0011-loans-are-payment-schedules.md)). It has no balance. What's left is the sum of its unpaid payments (§5.11).

```sql
CREATE TABLE loans (
  id              INTEGER PRIMARY KEY,
  name            TEXT    NOT NULL COLLATE NOCASE UNIQUE,
  account_id      INTEGER NOT NULL REFERENCES accounts(id)   ON DELETE RESTRICT,
  category_id     INTEGER NOT NULL REFERENCES categories(id) ON DELETE RESTRICT,
  is_new          INTEGER NOT NULL CHECK (is_new IN (0,1)),
  amount_borrowed INTEGER CHECK (amount_borrowed > 0),
  borrowed_on     TEXT    CHECK (borrowed_on GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
  added_on        TEXT    NOT NULL CHECK (added_on GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
  archived        INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0,1)),
  created_at      TEXT    NOT NULL,
  updated_at      TEXT    NOT NULL,
  CHECK (is_new = 0 OR borrowed_on IS NOT NULL)
) STRICT;
```

| Column | Notes |
|---|---|
| `name` | 1–50 characters, unique ignoring case, e.g. "E-wallet loan". The loans the user already has are kept one per lender app, with stacked loans together (PRD decision log). A **new** loan is always added as a loan of its own, even from an app that has one already, so it goes through Before you borrow and restarts the streak (LOAN-10). |
| `account_id` | The account its payments come from. Not a credit card (checked in Rust). A new loan needs an active account, and an edit may keep an archived one. **A loan puts its account in use**, like a goal, so that account can only be archived. |
| `category_id` | The expense category its payments are recorded in: **Loan** to start with (§7), when there's an active expense category of that name; otherwise the form starts empty. A new loan needs an active expense category, and an edit may keep an archived one. A loan puts its category in use, and merging categories moves it (§3.2). |
| `is_new` | 1 for a **new loan** (LOAN-10: it shows Before you borrow first and restarts the streak), 0 for **one I already have**. It can't change after the loan is created. |
| `amount_borrowed` | Optional, for the interest cost (LOAN-6, §5.11) |
| `borrowed_on` | The date borrowed. Required for a new loan (today to start with) and optional for one the user already has. Never after today. |
| `added_on` | The local date the loan was added to the app. It's a date, not a timestamp, because it's used in a calculation: on a loan the user already has, the payments dated before it are **paid before** (§4.6). |
| `archived` | Only a **paid-off** loan (no unpaid payment) can be archived, and it can be unarchived any time. An archived loan has no banner and nothing in the forecast, and it still counts in the Debt-free totals (§5.11). Anything that gives an archived loan an unpaid payment again (Mark unpaid, deleting a payment's expense, or adding rows) unarchives it in the same database transaction, so an archived loan never hides money still owed. |

- **Deleting:** a loan with any payment marked paid in the app can't be deleted (`Conflict`). It's archived once it's paid off. Any other loan can be deleted, and its payments go with it (`ON DELETE CASCADE`).
- A loan's amounts (its payments and amount borrowed) lock `currency_decimals` (§1).

### 4.6 `loan_payments` (Phase 4)

```sql
CREATE TABLE loan_payments (
  id             INTEGER PRIMARY KEY,
  loan_id        INTEGER NOT NULL REFERENCES loans(id) ON DELETE CASCADE,
  due_date       TEXT    NOT NULL CHECK (due_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
  amount         INTEGER NOT NULL CHECK (amount > 0),
  paid_before    INTEGER NOT NULL DEFAULT 0 CHECK (paid_before IN (0,1)),
  transaction_id INTEGER UNIQUE REFERENCES transactions(id) ON DELETE SET NULL,
  created_at     TEXT    NOT NULL,
  updated_at     TEXT    NOT NULL,
  CHECK (paid_before = 0 OR transaction_id IS NULL)
) STRICT;

CREATE INDEX idx_loan_payments_loan ON loan_payments(loan_id, due_date);
```

A payment's **status** is never stored. It's read from the row, so it can't go stale:

| Status | When |
|---|---|
| **Paid** | `transaction_id` is set: the user marked it paid (LOAN-3) |
| **Paid before** | `paid_before = 1`: on a loan the user already has, it's dated before the loan's `added_on`. It has no transaction, so past cashflow doesn't change. |
| **Unpaid** | Neither. It's **overdue** once `due_date` is before today. |

- `amount` is the **scheduled** amount. A payment's **paid amount** is its transaction's amount when it's paid (it may include a late fee), and its scheduled amount when it's paid before.
- Payments are listed by `due_date`, then `id`. Two payments may share a date.
- **An unpaid payment is not a transaction.** It counts toward no balance, total, report, budget or goal (§5.2). Only the forecast counts it, as a bill to come (§5.15).
- **Saving a loan** (LOAN-1, LOAN-2) writes the loan and its payments in one database transaction. A payment with a transaction is never changed by a save. Every other row can be edited, added or removed, and its `paid_before` is set again: 1 when the loan isn't new and the row is dated before `added_on`, otherwise 0. A new loan's payments can't be dated before its `borrowed_on`.
- **Mark paid** (LOAN-3) is allowed only on an unpaid payment. In one database transaction, it creates an expense from the fields the user confirmed (§6, keeping the loan's account and category even if they've been archived since, like a recurring rule's template, §4.2) and sets `transaction_id`. The fields start as today's date, the scheduled amount, the loan's account and category, the loan's name as the payee, and no payment method or note. It doesn't change `last_used_account_id`, the same as posting a due date.
- **Mark unpaid** deletes the payment's transaction. `ON DELETE SET NULL` then makes the payment unpaid, which is also what happens when that transaction is deleted from the transaction list. Editing the transaction keeps the link, and its amount becomes the paid amount. It stays an expense: `update_transaction` refuses to change its kind (`Conflict`: "This pays a loan. Mark it unpaid to change it.").

### 4.7 Payslips (Phase 4): `transactions.payslip_id`

```sql
ALTER TABLE transactions ADD COLUMN payslip_id INTEGER REFERENCES transactions(id) ON DELETE CASCADE;
CREATE INDEX idx_txn_payslip ON transactions(payslip_id) WHERE payslip_id IS NOT NULL;
```

A **payslip** (PAY-2) is one **gross** row, an income in the payday rule's category, plus any number of **deduction** rows, each an expense in an expense category. All of them are on the same account and date.
- **Every row of a payslip has `payslip_id` = the gross row's id**, the gross row included. So a payslip with no deductions is still a payslip, and the gross row is the income row whose `payslip_id` is its own `id`.
- **Net pay** = gross − Σ deductions. It must be more than 0.
- **Saving** a payslip writes every row in one database transaction: insert the gross row, set its `payslip_id` to its own id, then insert the deductions. The gross row takes the rule's payee, and the payslip's note.
- **Editing** a payslip updates the gross row in place, so its id (and the payday's link to it, §4.2) stays, and replaces the deduction rows, in one database transaction. The date and account change on every row.
- **Deleting any row deletes the whole payslip:** the service deletes the gross row, and `ON DELETE CASCADE` removes the deductions. The payday's occurrence stays posted, with `transaction_id` NULL, so it's never created again (§4.2). `update_transaction` refuses a payslip row (`Conflict`): it's edited as a payslip.
- Payslip rows are ordinary transactions in every total, list, report and export: the gross counts as income and each deduction as an expense (§5.2).

### 4.8 Paydays (Phase 4): `recurring_rules.is_payday`

```sql
ALTER TABLE recurring_rules ADD COLUMN is_payday INTEGER NOT NULL DEFAULT 0 CHECK (is_payday IN (0,1));
```

- **Only an income rule in Confirm mode, into a cash, bank or e-wallet account, can be a payday** (PAY-1, checked in Rust), since that's where a payslip goes. Its `amount` is the user's estimate of the net pay (§5.13), its category is the payslip's gross category, and its account is the payslip's account.
- Processing is unchanged (ARCHITECTURE §8): a payday date up to today is queued as `pending`.
- **A pending payday is settled with a payslip** (§4.7), never with Post or Edit & post, which return a `Conflict`. Skip works as usual.
- **Recording early (PAY-4):** a payday date after today and up to 3 days on (today Nov 13: Nov 14–16) that has no occurrence row yet can be recorded early, if the rule isn't paused and hasn't ended. In one database transaction, the service saves the payslip and inserts that date's occurrence row as `posted`, pointing at the gross row. When the date arrives, processing finds the row and passes over it (the unique key, §4.2), so the payday is never created twice. The row counts toward `max_occurrences`, like any processed date.

---

## 5. Calculation rules

All of these calculations are done in the Rust backend (see ARCHITECTURE.md §4). The SQL below shows the logic, not necessarily the exact queries.

### 5.1 Account balance

Balances are **signed** and use the same formula for every kind of account:

```
balance(account, as_of) =
    opening_balance
  + Σ amount  WHERE kind = 'income'   AND account_id    = account
  − Σ amount  WHERE kind = 'expense'  AND account_id    = account
  − Σ amount  WHERE kind = 'transfer' AND account_id    = account   -- money sent
  + Σ amount  WHERE kind = 'transfer' AND to_account_id = account   -- money received
  (only transactions with date ≤ as_of)
```

- **Current balance** uses `as_of = today`. Future-dated transactions are not included (TXN-8).
- **Credit cards:** a negative balance means money is owed.
  - `amount_owed = max(0, −balance)`. A positive balance means you overpaid and have credit on the card.
  - `available_credit = credit_limit − amount_owed`
  - `utilization = amount_owed ÷ credit_limit`
  - In the forms, the user enters the opening amount owed as a **positive** number. The app stores it as a negative number.
- **Net balance (ACC-5)** = the sum of `balance` across **all** accounts, including archived ones. Card debt is already negative, so it's subtracted automatically. Because archived accounts still count, `archive_account` returns a `Conflict` error when the account's current balance isn't 0, unless it's called again with `acknowledge_balance = true` after the user confirms the warning.
- **Limits:** spending past a card's limit or below 0 in an account is allowed. `available_credit` can go negative, and it's shown as it is.

### 5.2 Period totals

```
income(period)  = Σ amount WHERE kind = 'income'  AND date BETWEEN start AND end
expense(period) = Σ amount WHERE kind = 'expense' AND date BETWEEN start AND end
net(period)     = income − expense
```

- **Transfers are never included** (RPT-6). This is why paying the card bill doesn't count the purchases twice.
- **Future-dated transactions are never included** (TXN-8, Phase 3). Every total, budget and "recent" list counts only `date ≤ today`, matching balances, so the numbers on a screen always agree. Future-dated rows appear only under Upcoming: the transaction list of a period that hasn't ended shows every later row that matches its other filters there, whatever its date, while its summary and day nets still count only `date ≤ today`.
- If the report is filtered by account (RPT-5), only transactions with `account_id = account` are counted.
- **Refunds** (CC-5) use an income category, so a refund increases *income*, not lowers *expense*. This is intentional: refunds offset net cashflow, not the expense total or the category breakdown (decided 2026-10-26).
- **Unpaid loan payments are never included** (Phase 4): they aren't transactions until they're marked paid (§4.6). A payslip's rows count like any other: the gross as income, each deduction as an expense (§4.7).

### 5.3 Period boundaries

Every period is a closed date range `[start, end]` that includes both ends. Given an anchor date `d`:

| Period | start | end |
|---|---|---|
| Day | `d` | `d` |
| Week | `d − ((weekday(d) − week_start) mod 7)` days | `start + 6` |
| Month (`month_start_day` = s) | if `day(d) ≥ s`: the s-th of d's month; otherwise the s-th of the previous month | the day before the next month's start |
| Year | January 1 of d's year | December 31 |
| Custom | chosen by the user | chosen by the user |

- With s = 1, a "month" is the calendar month. With s = 25, Oct 2026 runs from Sep 25 to Oct 24.
- **The year is always the calendar year**, even when `month_start_day` isn't 1.
- **Previous and next** are found by moving the anchor date by one period length (one day, 7 days, one month or one year) and recalculating. There is no next period when it would start after today.
- **Custom** has no anchor. Previous and next move both dates by the length of the range, counting both ends. For example, Oct 1–10 (10 days) has Sep 21–30 before it and Oct 11–20 after it. The comparison in §5.4 uses this same previous range.
- **Today** goes to the period of the same kind that contains today. For Custom, that's the range in the same sequence (moved by whole range lengths, backward too) that contains today, which is where ▶ stops. For example, from Oct 1–10 with today Nov 3, Today goes to Oct 31–Nov 9.

### 5.4 Period comparison (RPT-4a)

```
elapsed     = today − current.start                     (in days)
if today is inside the current period AND comparison_mode = 'to_date':
    compare_end = min(previous.start + elapsed, previous.end)
    compare     = [previous.start, compare_end]
else:
    compare     = [previous.start, previous.end]        (the full period)

% change = (current − compare) ÷ |compare| × 100
```

If `compare` is 0, the change is shown as "—" instead of a percentage. The percentage is rounded to one decimal place, half away from zero, and sent as whole tenths of a percent (−563 means −56.3%), worked out with integers.

- The current side counts `[current.start, min(current.end, today)]` (§5.2), so a period in progress is compared day for day.
- `mode` is the `comparison_mode` setting (SET-7) unless Reports' **Compare to** picks one for that view (UI_FLOWS §8.1). The dashboard's % change always uses the setting (DSH-3).
- The comparison table (RPT-4) has income, expense and net cashflow, plus every category with an amount on either side, grouped by kind and ranked by this period's amount, then the comparison's, then category id. Net is income − expense on each side, so its % uses `|compare|` when the comparison's net was negative.

### 5.5 Credit card statement cycle and due date

The statement closing date in a given month is `min(statement_day, last day of the month)`.

- **Current cycle:** from the day after the most recent closing date before today, through the next closing date (today or later).
- **Cycle spending (CC-6):** total expenses on the card within the current cycle.
- **Next due date:** the first date on or after today whose day is `min(due_day, last day of the month)`.
- **Reminder (CC-7, Phase 3):** shown when the next due date is 5 days away or closer (0–5 days, 0 being today) and `amount_owed > 0`, for active cards only.

### 5.6 Budget progress (Phase 3)

```
spent     = Σ expense amounts in the budget's current period (in its category, or in all categories for an overall budget)
remaining = amount − spent    (below 0 once the budget is over)
used%     = ⌊spent × 100 ÷ amount⌋   →  amber when ≥ 80%, red when ≥ 100%
```

- **The current period** is the week, month or year that contains today, by `week_start` and `month_start_day` (§5.3). Years are calendar years. It never follows the period a screen shows, so the dashboard's budgets match the Budgets screen whatever the period switcher says (DSH-6).
- Only rows dated today or earlier count (§5.2), and transfers never do.
- **No rollover (BUD-4):** each period starts again from the full `amount`. What a period leaves unspent, or spends over, never moves to the next.
- `used%` is a whole percent **rounded down**, so the color changes only once the threshold is really reached: 79.9% shows 79% in the normal color, and 99.9% shows 99% in amber, never red while something is left.

Refunds don't reduce `spent`, because they're income (§5.2).

### 5.7 Goal progress (Phase 3)

```
progress = max(0, current balance of the linked account)
% done   = min(100, progress ÷ target_amount × 100)
monthly_needed = (target_amount − progress) ÷ months left until target_date   (rounded up, if there is a target date)
```

- **Progress** is the linked account's balance as of today (§5.1), so a future-dated row counts only once its date arrives (§5.2). A balance below 0 is no progress.
- **% done** is a whole percent **rounded down**, so it shows 100 only once the target is really reached.
- **Months left** are calendar months from today to `target_date`, a part month counting as a whole one: from Nov 12, Dec 12 is 1 month and Dec 13 is 2. Adding a month clamps to the month's end, so from Jan 31, Feb 28 is 1 month. Once the target date is today or has passed, months left is 0 and there's no monthly amount.
- **Monthly needed** is rounded up to the minor unit, so saving it every month always reaches the target: ₱2,600 over 6 months is ₱433.34. There's none once nothing is left to save, or without a target date.

### 5.8 Default accounts

| Used for | Rule |
|---|---|
| **Quick-add (TXN-1)** | `last_used_account_id`, which is updated whenever a transaction is created. If it's missing or the account is archived or deleted, use the first active account by `sort_order`. |
| **Pay bill's From account (CC-9)** | The active non-card account on the latest transaction dated today or earlier (`date DESC, id DESC`), going by each transaction's source `account_id`. If there is none, the first active non-card account by `sort_order`. If there's no such account, the field starts empty. |
| **Pay bill's amount (CC-9)** | The card's `amount_owed`. If that's 0, the field starts empty. |

### 5.9 Income-vs-expense chart bars (RPT-1)

The chart has one bar per day, week or month, covering the whole period with no gaps. Each bar holds the income, expense and net (§5.2) of its own dates.

| Period | Bars |
|---|---|
| Day | One bar, the day itself |
| Week, Month | One bar per day, so a month that starts on the 25th runs from the 25th to the 24th |
| Year | 12 calendar months, Jan to Dec, whatever `month_start_day` is |
| Custom, up to 31 days (counting both ends) | One bar per day |
| Custom, ending before the same date 6 months after its start | One bar per week, following `week_start` |
| Custom, longer | One bar per calendar month |

- On a Custom range, the first and last week or month are cut to the range, e.g. Oct 1 (a Thursday) – Nov 1 gives Oct 1–4, then Monday weeks, ending Oct 26 – Nov 1.
- "6 months after" uses calendar months, with the day clamped to the end of the month: Jan 1 – Jun 30 is by week and Jan 1 – Jul 1 by month. Six months after Aug 31 is Feb 28 (or 29).
- Bars after today hold 0 (§5.2), and the chart's net line stops at today.

### 5.10 Category breakdown (RPT-2)

```
amount(category) = Σ amount WHERE kind = <expense or income> AND category_id = category, in the period (§5.2)
share(category)  = amount(category) ÷ total × 100, in tenths of a percent, rounded half up
```

- Every category with an amount appears, archived ones included, ranked by amount, largest first (ties by category id).
- The donut shows the 8 largest, then one **Other** slice with the rest added up (DESIGN_SYSTEM §3.4).
- Shares are whole tenths of a percent (333 means 33.3%), worked out with integers, so they may add up to 99.9% or 100.1%.
- Refunds are income, so they appear in the income breakdown and never reduce an expense category (§5.2).

### 5.11 Loan figures (LOAN-5 to LOAN-8, LOAN-11, Phase 4)

For each loan, from its payments (§4.6):

```
left      = Σ amount of its unpaid payments
paid      = Σ paid amount of its paid and paid-before payments
% paid    = ⌊paid × 100 ÷ (paid + left)⌋                    (100 once nothing is left)
next      = its earliest unpaid payment (by due_date, then id)
interest  = (paid + left) − amount_borrowed                 (none without an amount borrowed, or when that's below 0)
paid off  = it has no unpaid payment
```

Across **every loan, archived ones included**:

```
total left   = Σ left
total paid   = Σ paid
debt-free on = the latest due_date of any unpaid payment, or today when that has passed
                                                               (none once nothing is left)
days to go   = debt-free on − today
interest     = Σ interest of the loans that have one           (none when no loan has one)
streak       = today − the latest borrowed_on of any loan      (in days; none when no loan has a borrowed_on)
```

- **% paid** is rounded down, so it says 100 only once the loan is really paid off.
- **When everything left is overdue**, the debt-free date is today, and the Debt-free screen says what's overdue instead of counting down (UI_FLOWS §8.5).
- **The streak** goes by dates borrowed only. Adding a loan the user already has never resets it to 0, but its date borrowed, if it's the newest, is when the user last borrowed, so the streak counts from there.
- **Money freed up (LOAN-7):** one row per month (the app's months, §5.3), from the current month to the month after the debt-free date:
  ```
  due(m)   = Σ scheduled amount of every payment due in m, whatever its status
             (for the current month, plus every unpaid payment due before it)
  freed(m) = due(current month) − due(m)
  ```
  The current month counts its paid payments too, so the starting point doesn't move as the user pays. `freed` is below 0 when a later month is heavier than this one.
- **The dashboard line (LOAN-11)** shows `total left` and `debt-free on`, and is hidden when nothing is left.
- **Reminder banners (LOAN-4):** for each active loan, its `next` payment when that's due within 5 days or is overdue (`due_date ≤ today + 5`), soonest first. `days = due_date − today`, below 0 when it's overdue, and the banner also gives the number of other overdue payments that loan has. **"Move money first"** goes by account. In date order, the payments these banners show from the same account are added up, and each banner gives only its own part of what that running total lacks against the account's balance today (§5.1). So following every banner moves exactly enough: with ₱200 in the account and ₱1,000 then ₱500 due, the banners say ₱800, then ₱500.

### 5.12 Pay windows (SAFE-1, Phase 4)

```
horizon  = today through the day before the same date 12 months on       (Nov 13, 2026 → Nov 12, 2027;
                                                                          the day is clamped to the month's end)
paydays  = every date of every payday rule that isn't paused, from its schedule (§4.1), up to the horizon's end,
           counting dates already processed and stopping where the rule ends
current  = from the latest payday on or before today (today, when there's none)
           to the day before the next payday after today (or the horizon's end)
windows  = the current window, then one from each later payday to the day before the next,
           the last one ending on the horizon's end
```

- Two payday rules on the same date make one window boundary.
- A bill due on a payday belongs to the window that starts that day (the pay arrives that day).
- Windows always use the **scheduled** dates, even when a payslip is recorded early (§4.8).

### 5.13 Expected pay (SAFE-4, Phase 4)

```
net(payslip)  = gross − Σ deductions                                   (§4.7)
recent(c)     = the nets of the 3 latest payslips whose gross is in category c,
                dated today or earlier, by date then id
expected(r)   = the lowest of recent(r.category), plus r.amount while recent has fewer than 3
```

- Paydays of the same category share their payslips: a Salary rule on the 15th and one on the 30th both use the last 3 Salary payslips. Each adds its own estimate until there are 3.
- What a payday date `d` of rule `r` adds to the forecast:

| That date's occurrence | Money in |
|---|---|
| None yet, `d` after today | `expected(r)`, on `d` |
| `pending` (`d` up to today) | `expected(r)`, as due today |
| `posted`, on time or early | Nothing: its payslip is already in the balances. A payslip dated after today counts like any future-dated row instead (§5.15). |
| `skipped` | Nothing |

- Pay counts only when the rule's account is a counted account (§5.15).

### 5.14 Card bills in the forecast (SAFE-3, Phase 4)

Each credit card, archived ones included, gets a **bill on each due date** in the horizon: the statement balance, as if it's paid in full every month (ADR 0006).

```
closing dates = min(statement_day, the month's last day), every month         (§5.5)
due dates     = min(due_day, the month's last day), from the next due date (§5.5) to the horizon's end
C(D)          = the latest closing date before the due date D: the statement that D pays
```

Walk forward from today, starting from what's owed now (`amount_owed`, §5.1):
- **On the next due date D₁:** if C(D₁) is today or earlier, the statement has already closed. The bill is what was owed at the end of C(D₁) (`max(0, −balance as of C(D₁))`), less the transfers into the card and the income on it dated after C(D₁) up to today, never below 0 and never more than what's owed now. If C(D₁) is after today, the bill is what's projected to be owed at the end of C(D₁).
- **On every later due date D:** the bill is what's projected to be owed at the end of C(D).
- **Projected owed:** a future-dated expense on the card, or a coming recurring expense date on it (a pending one counts as today), adds its amount on its date. A future-dated or recurring income on the card (a refund) takes its amount off. Each bill takes its own amount off on its due date.
- The bill is money out of the counted accounts on its due date (§5.15). A card with nothing owed and nothing projected has no bills.

### 5.15 Safe to spend and the forecast (SAFE-2, SAFE-3, SAFE-5, SAFE-7, Phase 4)

**Counted accounts** are every active cash, bank and e-wallet account, except one linked to a goal that isn't archived (§4.4).

```
money now = Σ balance(a, today) over the counted accounts                    (§5.1)
```

**Movements** are what's due in the horizon, each on a date. Anything overdue or waiting to be settled counts as **due today**.

| Movement | Counts as |
|---|---|
| An unpaid payment of an active loan | Out, on its due date, **whatever its account** (the debt is paid from spendable money) |
| A future-dated transaction (dated after today) | An expense from a counted account is out. An income into one is in. A transfer from a counted account to one that isn't counted is out, and the reverse is in. A transfer between two counted accounts, and **any transfer to or from a credit card**, isn't counted: card bills stand in for those (§5.14). |
| A recurring rule's dates: `pending` ones (as today) and the dates still to come in the horizon, for a rule that isn't paused and hasn't ended, other than a payday | Its template on that date, counted like a future-dated transaction |
| A payday | §5.13 |
| A card bill | Out, on its due date (§5.14) |

For each pay window `k` (§5.12), in order:

```
start(k)      = money now for the current window, else left(k − 1)
in(k), out(k) = the movements dated in window k
left(k)       = start(k) + in(k) − out(k)            -- what would be left at its end with nothing else spent
run(d)        = money now + every movement dated up to the end of day d   -- the running total, day by day

safe to spend = max(0, the lowest run(d) over every day of the horizon)
held back     = left(current) − safe to spend        -- when that's more than 0, held back for the window that
                                                        holds the lowest day (the first one on a tie)
short         = the first day with run(d) < 0: short by −(the lowest run(d) in that day's window), on that day
per day       = ⌊safe to spend ÷ days left in the current window, counting today⌋, in minor units
```

- **Safe to spend never looks only to the next payday.** It's the lowest point over the whole 12 months, day by day, so money for a short pay window later is held back now, and so is money for a bill due before other income arrives later in the same window (SAFE-5). Budgets and savings goals don't hold anything back (PRD §3).
- **A window runs short** when any day in it has `run(d)` below 0, even if money later in the window brings it back.
- **When it's short**, safe to spend is 0, and the dashboard and Forecast say "Short by ₱X on *date*", using `short`.
- **With no payday rule**, there are no pay windows: the forecast returns none and says how to set one up (SAFE-6).
- **A payslip already recorded** in the current window is in `money now`. The Forecast lists it as received, without adding it again.
- **The Months view (SAFE-7)** groups the same movements by the app's months (§5.3): from the current month, starting today, to the horizon's end, with the same `start`, `in`, `out` and `left`. Safe to spend always comes from the pay windows.

### 5.16 Before you borrow (LOAN-9, LOAN-10, Phase 4)

A **draft loan** is either the rows the user typed in the Add loan form (LOAN-10), or a payment amount, a number of payments and a first payment date (LOAN-9). The second form gives monthly dates from the first one, as a monthly rule does (§4.1: the *n*-th date is `first + n` months, with the day clamped to the month's end), so the day of the month is the first payment's.

The draft's payments are added as unpaid payments of an active loan, and nothing is saved. The result is:
- the **debt-free date** before and after (§5.11)
- **safe to spend** before and after (§5.15)
- **each pay window's** `left` before and after, so the screen can say which windows it hurts and which would run short

---

## 6. Validation rules (checked in Rust)

The database's CHECK constraints are a last line of defence. The Rust service layer checks everything **first** and returns clear, per-field errors that the forms can display.

| Rule | Error shown on |
|---|---|
| Amount is > 0 and ≤ 999,999,999,999 minor units | amount |
| The date is a real calendar date | date |
| An opening balance is 0 or more and ≤ 999,999,999,999 minor units, and a credit limit is > 0 and ≤ the same maximum | that field |
| A transfer has no payee or payment method | payee / payment method |
| The last active account, or the last active category of a kind, can't be archived or deleted (`Conflict`: "Keep at least one active account") | shown as a toast |
| A category's color is a palette key and its icon is on the curated list (DESIGN_SYSTEM §3.4, §6) | color / icon |
| A transaction's category kind matches the transaction kind | category |
| A transfer's destination differs from its source | to_account |
| A **new** transaction can't use an archived account, category or payment method. Existing transactions keep theirs, and a recurring rule's dates keep its template's (§4.2). | account / category |
| Name lengths (account ≤ 50, category ≤ 40, payment method ≤ 40, payee ≤ 100, note ≤ 500) | that field |
| Names are unique within their scope | name |
| Credit card fields are present if and only if the account is a credit card | the card fields |
| `currency_decimals` can't change once any amount is stored: a transaction, a recurring rule, a budget, a goal, a loan, or an account's non-zero opening balance or credit limit (§1) | settings |
| A budget's category is an expense category, and a goal's account isn't a credit card | category / account |
| A goal's name is 1–50 characters and unique, ignoring case; its target amount is > 0 and ≤ the maximum; its target date, if any, is a real date (past dates are allowed); a new goal can't use an archived account, but an edit may keep the one it has | name / targetAmount / targetDate / accountId |
| A new budget can't use an archived category, but an edit may keep the one it has. A category, or all spending, has at most one budget per period. | categoryId / period |
| A recurring rule's template follows the transaction rules above: its date is the start date (§4.1) | that field |
| A rule's interval is 1–999, its number of times is 1–9,999, and its end date is a real date on or after the start date. A rule ends on a date or after a number of times, not both. | interval / maxOccurrences / endDate |
| A rule's new start date, when its schedule changes, is after the last date it already processed, and not before today or its current next date, whichever is earlier (§4.1) | date |
| A loan's name is 1–50 characters and unique, ignoring case. Its account is a cash, bank or e-wallet account, and its category an expense category, each active for a new loan (an edit may keep an archived one). Its amount borrowed, if any, is > 0 and ≤ the maximum. | name / accountId / categoryId / amountBorrowed |
| A new loan's date borrowed is required, a real date and not after today. One the user already has may leave it empty, but a date given follows the same rule. | borrowedOn |
| A loan has 1–600 payments, each with a real date and an amount > 0 and ≤ the maximum. A new loan's payments aren't dated before its date borrowed. | payments |
| Only an unpaid payment can be marked paid, and only a paid one marked unpaid. A payment with a transaction can't be edited or removed (`Conflict`). | shown as a toast |
| A loan with a payment marked paid can't be deleted, and only a paid-off loan can be archived (`Conflict`: "Pay or remove its unpaid payments first") | shown as a toast |
| A payday rule is an income rule in Confirm mode, into a cash, bank or e-wallet account (§4.8) | isPayday / mode / accountId |
| A payslip's gross is > 0 and ≤ the maximum. It has 0–30 deduction lines, each with an expense category (active, though an edit may keep an archived one) and an amount > 0. Its net is more than 0 ("Deductions can't add up to the gross pay or more"). Its date is a real date, and its account an active cash, bank or e-wallet account (an edit may keep an archived one). | gross / deductions / date / accountId |
| A payday can be recorded early only when it's 1–3 days after today, has no occurrence yet, and its rule isn't paused or ended (§4.8, `Conflict` otherwise) | shown as a toast |

Text is trimmed before saving, and empty optional text is stored as `NULL`.

---

## 7. Seed data (created on first launch only)

**Accounts:** `Cash` (cash, opening balance 0). This lets quick-add work right away. Onboarding then asks for the other accounts.

**Payment methods:** Cash, Debit Card, Credit Card, Bank Transfer, E-wallet, Other

Income and expense colors are **numbered separately**, because the breakdown chart never shows income and expense together. The 12 most common expense categories each get their own color, and only 5 rarely used ones share. A category the user adds later defaults to the least-used color among the **active** categories of its kind, the lowest number on a tie (e.g. `cat-2` before `cat-10`), and the user can change it.

**Income categories:**

| Name | Icon | Color |
|---|---|---|
| Salary | `briefcase` | `cat-12` emerald |
| Bonus | `award` | `cat-9` lime |
| Freelance/Side income | `laptop` | `cat-3` teal |
| Interest | `percent` | `cat-7` cyan |
| Gifts received | `gift` | `cat-4` pink |
| Refund | `rotate-ccw` | `cat-10` fuchsia |
| Other income | `circle-plus` | `cat-11` stone |

**Expense categories:**

| Name | Icon | Color |
|---|---|---|
| Food & Groceries | `shopping-cart` | `cat-2` orange |
| Dining Out | `utensils` | `cat-6` amber |
| Transportation | `car` | `cat-1` blue |
| Rent/Housing | `house` | `cat-11` stone |
| Utilities | `zap` | `cat-7` cyan |
| Phone | `smartphone` | `cat-5` violet |
| Shopping | `shopping-bag` | `cat-4` pink |
| Health & Medical | `heart-pulse` | `cat-8` rose |
| Insurance | `shield` | `cat-3` teal |
| Entertainment | `film` | `cat-10` fuchsia |
| Subscriptions | `repeat` | `cat-9` lime |
| Personal Care | `sparkles` | `cat-12` emerald |
| Education *(rarely used)* | `graduation-cap` | `cat-1` blue, shared with Transportation |
| Gifts & Donations *(rarely used)* | `heart-handshake` | `cat-4` pink, shared with Shopping |
| Travel *(rarely used)* | `plane` | `cat-3` teal, shared with Insurance |
| Fees & Charges *(rarely used)* | `receipt` | `cat-5` violet, shared with Phone |
| Other expense *(rarely used)* | `ellipsis` | `cat-11` stone, shared with Rent/Housing |

**Added in Phase 4 (PRD Appendix A):** migration 0003 adds these expense categories to every database, new or existing, that doesn't already have an expense category of the same name (ignoring case). A category that's already there, such as one the user made called "Loan", is kept as it is. They go after the existing expense categories, in this order. Each one gets the least-used color among the active expense categories at that point, the lowest number on a tie (the same rule as a category the user adds). On a new database, that gives:

| Name | Icon | Color (on a new database) | Used for |
|---|---|---|---|
| Loan | `coins` | `cat-2` orange | Loan payments, the default (LOAN-3) |
| SSS | `shield` | `cat-6` amber | Payslip deductions (PAY-3) |
| PhilHealth | `heart-pulse` | `cat-7` cyan | Payslip deductions |
| Pag-IBIG | `house` | `cat-8` rose | Payslip deductions |
| Withholding tax | `receipt` | `cat-9` lime | Payslip deductions |
| Salary loan | `banknote` | `cat-10` fuchsia | Payslip deductions, for a loan repaid through payroll |
| Other deductions | `ellipsis` | `cat-12` emerald | Payslip deductions |

Every icon is already on the curated list (DESIGN_SYSTEM §6). These are ordinary expense categories: they can be renamed, recolored, archived, merged or deleted like any other.

---

## 8. Migrations

- The migration files live in `src-tauri/migrations/`, named `NNNN_description.sql`, and are run by `rusqlite_migration`. The current schema version is stored in SQLite's `PRAGMA user_version`.
- **A migration is never edited once it has been released.** Every schema change is a new file.
- Each migration runs inside a single database transaction, so it either completes fully or changes nothing.
- **The database is backed up automatically before any migration runs (BAK-5).** This applies when an existing database has pending migrations, not when a new one is created (`user_version` 0, which has nothing to lose). The copy goes to `<app data>\backups\pre-migration\jobertsvault-pre-migration-v<schema version>-YYYY-MM-DD_HHMM.db`, whatever `backup_folder` says, and is never deleted automatically. `<schema version>` is the version the copy holds, from before the migration, and the time is local. Debug builds write `<app data>\backups-dev\pre-migration\jobertsvault-dev-pre-migration-…` instead. If the backup fails, the migration doesn't run and the app shows the startup error screen.
- The app refuses to open a database whose `user_version` is **newer** than it knows about (e.g. a backup made by a later version). It shows a message instead of risking damage to the data.

| File | Phase | Contents |
|---|---|---|
| `0001_initial.sql` | 1 | accounts, categories, payment_methods, transactions, settings, indexes |
| — | 2 | Phase 2 needed no schema change: its new data is settings keys (§3.5) |
| `0002_planning.sql` | 3 | recurring_rules, recurring_occurrences, budgets, goals |
| `0003_loans_payroll.sql` | 4 | loans, loan_payments, `transactions.payslip_id`, `recurring_rules.is_payday`, and the Phase 4 categories a database doesn't have yet (§7), all in the migration's one database transaction |

### CSV export format (BAK-6, Phase 2)

UTF-8 **with a byte order mark (BOM)** so Excel shows non-ASCII text correctly. Commas separate the columns, every line ends in CRLF, and the first row is a header. A field that holds a comma, a double quote or a line break is put in double quotes, with its own quotes doubled (RFC 4180, `domain::csv`). Rows are every transaction dated from the chosen start to the chosen end, both included, oldest first (then by id), with the names of archived accounts, categories and payment methods as they are. Dates are `YYYY-MM-DD`.

```
date,type,amount,account,to_account,category,payee,payment_method,note
2026-10-25,expense,250.00,Cash,,Dining Out,Corner Cafe,Cash,Lunch
2026-10-25,transfer,5000.00,Bank Savings,Credit Card,,,,October bill
```

The amount is written as a positive number with `currency_decimals` decimal places and no currency symbol, built from the stored integer (`Money::to_plain`), never through a float. A transfer leaves category, payee and payment method empty, and an income or expense leaves to_account empty.

---

## 9. Open questions

None at the moment. Answered on 2026-10-26:

1. **Refunds** count as income. They offset net cashflow, and the PRD wording (CC-5) was updated to match.
2. **Archived accounts** still count toward the net balance. Archiving one whose balance isn't 0 shows a warning (§5.1).

Answered on 2026-11-01:

3. **Future dates** are rejected until Phase 3 (§6). Phase 3 lifted the rule (27).
4. **Custom periods** move by the length of their range (§5.3).
5. **Opening balances** are entered as 0 or more (§6).
6. **Transfers** have no payee or payment method (§3.4).
7. **Limits** aren't enforced: going past a card's limit or below 0 in an account is allowed (§5.1).
8. **Default accounts** follow §5.8, and first-launch completion is stored in `onboarding_completed` (§3.5).
9. **Future-dated transactions** stay out of every total until their date arrives (§5.2).
10. **Moving forward** stops at the current period (§5.3).

Answered on 2026-11-03:

11. **Debug builds' pre-migration backups** go in `backups-dev\pre-migration`, named `jobertsvault-dev-pre-migration-…`, so a dev copy never sits next to a real one or replaces it (§8).

Answered on 2026-11-04:

12. **Payment method names** are 1–40 characters, the same as categories (§6).
13. **A new category's default color** counts only active categories of its kind, so archived ones don't hold a color, and a tie goes to the lowest palette number (§7).
14. **The decimals lock** covers every stored amount, not only transactions: an account's opening balance other than 0, or a credit limit, locks `currency_decimals` too, because changing it would reinterpret those amounts (§1). The seeded Cash account holds 0, so a new database starts unlocked. ADR 0002's rule still holds, since a transaction still locks it.
15. **Pay bill's From account** goes by each transaction's source `account_id`: the latest transaction whose source is an active non-card account. A transfer's destination doesn't count as using that account (§5.8).
16. **Today on a Custom period** jumps to the range in the same sequence that contains today, which is where ▶ stops (§5.3).

Answered on 2026-11-06:

17. **Chart bars (RPT-1)** are calendar-aligned: a Day period is one bar, a Year is 12 calendar months even when `month_start_day` > 1, and Custom weeks follow `week_start` and Custom months are calendar months, cut to the range. "Up to 6 months" means ending before the same date 6 months after the start (§5.9).
18. **Payee suggestions (TXN-6)** match the form's type: an expense suggests only payees used on expenses, with the category of the latest expense to that payee, and the same for income. A payee whose last category is archived is still suggested, but choosing it leaves the category as it is (§3.4).

Answered on 2026-11-07:

19. **A restore keeps this computer's backup settings** (BAK-4): `backup_folder`, `auto_backup_enabled`, `last_backup_at` and `last_auto_backup_date` keep their current values, and are deleted where the current data has none, while every other setting comes from the backup. Restoring on a second Windows account then doesn't point its backups at the first account's folder, and the last backup date doesn't go back in time (§3.5, ARCHITECTURE §7).
20. **Phase 3's migration is `0002_planning.sql`**, not `0003`: Phase 2 needed no schema change, and keeping each file's number equal to the schema version it creates keeps the pre-migration backups' `v<schema version>` names easy to match to a file (§8).

Answered on 2026-11-11:

21. **Editing a rule's schedule** takes the start date from the form, after the last date already processed (and not before today or the current next date, whichever is earlier), so a month-end rule never drifts. `max_occurrences` counts the dates actually processed, so the count holds across the edit (§4.1).
22. **Resuming a paused rule** skips the dates it would have had while paused (§4.1).
23. **A rule's amount locks `currency_decimals`** like any other stored amount (§1). A rule's account, category or payment method counts as in use, so it's archived instead of deleted, and merging categories moves rules along with transactions.
24. **A rule's start date** may be in the past; processing fills in the dates since then (§4.1).
25. **Saving a rule processes it** (REC-4): creating, editing or resuming a rule posts or queues its dates up to today in the same database transaction, so a rule that starts today shows up at once rather than the next day or launch (ARCHITECTURE §8).
26. **An archived account, category or payment method** that a rule's template uses doesn't stop its dates: they post with it, the same way an edit of the rule may keep it (§4.2, §6).
27. **Upcoming (TXN-8)** lists every future-dated row that matches the list's other filters, whatever its date, whenever the selected period hasn't ended, since ▶ stops at the current period and a row dated later would otherwise never be seen. Phase 3 drops the "Date can't be in the future" rule (§5.2, §6).
28. **"Most recent" lookups skip future-dated rows** (TXN-8): payee suggestions and Pay bill's From account count only rows dated today or earlier, so one row dated months ahead can't hold the suggestion or the default until its date (§3.4, §5.8).
29. **Budgets are never archived** (BUD-1): they're edited or deleted, because nothing refers to one and a budget keeps no history. The `archived` column stays 0 (§4.3).
30. **Deleting or merging a category deletes its budgets** in the same database transaction, and the confirm dialogs say so. A budget doesn't make a category in use, and a merge doesn't move budgets to the target (§3.2, §4.3).
31. **A budget's amount locks `currency_decimals`** like every other stored amount (§1).
32. **Budgets always show their own current period** (BUD-2, DSH-6): the dashboard's budgets don't follow the period switcher, the same way its account balances don't (§5.6).

Answered on 2026-11-12:

33. **Months left for a goal (GOL-3)** are calendar months to the target date, a part month counting as a whole one, and the monthly amount is rounded up (§5.7).
34. **A goal whose target date is today or has passed** has no monthly amount: months left is 0, and the Goals screen says the date has passed. Past target dates are accepted on save (§4.4).
35. **A goal puts its account in use** (GOL-2), so an account with a goal is archived instead of deleted, matching `ON DELETE RESTRICT` (§4.4).
36. **A goal's target amount locks `currency_decimals`** like every other stored amount (§1).
37. **A goal is marked achieved when `list_goals` runs** (GOL-4) and finds its progress at the target: that day is stored in `achieved_at`, and it stays if the balance drops. A higher target or another account clears it, and archived goals aren't checked. A reach and drop between two visits to Goals isn't recorded (§4.4).
38. **Any goal can be archived**, achieved or not, and archived goals fold under Show archived with Unarchive and Delete (§4.4, UI_FLOWS §8.4).
39. **Coming up (DSH-7)** covers tomorrow to 7 days from today, since dates up to today are already posted or in the Due list. It lists recurring rules' dates only, not one-off future-dated transactions, and every date of a rule, the dashboard showing at most 8 and then "+ N more" (§4.1, UI_FLOWS §2).

Answered on 2026-11-13 (Phase 4):

40. **A loan is a list of scheduled payments**, not an account. An unpaid payment isn't a transaction, so it counts toward nothing until it's marked paid (§4.5, §4.6, ADR 0011).
41. **A payment's status is read, never stored:** paid when it has a transaction, paid before when its flag is set, unpaid otherwise. Deleting the transaction makes it unpaid through `ON DELETE SET NULL`, so the two can't disagree (§4.6).
42. **"Paid before" goes by the loan's `added_on`**, the day it was added, rather than by today. So a row added later on a loan the user already has is unpaid when it falls after that day, and becomes overdue (§4.6).
43. **Only a paid-off loan can be archived,** so an archived loan never hides money still owed. A loan with a payment marked paid can't be deleted (§4.5).
44. **A loan's account can't be a credit card**, and a loan puts its account and category in use. Merging categories moves loans (§3.2, §4.5).
45. **A new loan is always a loan of its own**, even from a lender app that already has one, so it goes through Before you borrow and restarts the streak. The loans the user already has stay one per lender app (§4.5).
46. **The Debt-free totals count every loan, archived ones included**, so the total paid and the interest don't drop when a paid-off loan is archived (§5.11).
47. **Money freed up** compares the scheduled amounts due each month with the current month's, counting the current month's paid payments too, so its starting point doesn't move as the user pays (§5.11).
48. **A payslip's rows share `payslip_id`, the gross row's id**, and are saved, edited and deleted together. Deleting any one of them deletes the whole payslip (§4.7).
49. **A payday is an income rule in Confirm mode.** It's settled with a payslip, never Post, and it can be recorded up to 3 days early, which adds its occurrence row so processing never creates it again (§4.8).
50. **Expected pay** is the lowest net of the last 3 payslips of the rule's category, plus the rule's estimate until there are 3. Rules of the same category share their payslips (§5.13).
51. **Pay windows** start on each payday rule's scheduled dates. A bill due on a payday belongs to the window that starts that day (§5.12).
52. **Card bills** are the statement balance on each due date, as if it's paid in full. Transfers to or from a card aren't counted in the forecast, because the bills stand in for them (§5.14).
53. **Every unpaid loan payment counts in the forecast,** whatever its account, since the debt is paid from spendable money (§5.15).
54. **The horizon** runs from today through the day before the same date 12 months on (§5.12).
55. **Safe to spend** is the lowest point of the running total, day by day, over the horizon, never below 0, with the held-back amount and the first short day reported beside it. Checking only each window's end would miss a bill due before money that arrives later in the same window (§5.15).
56. **A loan's amounts lock `currency_decimals`** like every other stored amount (§1).
57. **An archived loan is unarchived** by anything that gives it an unpaid payment again: Mark unpaid, deleting a payment's expense, or adding rows (§4.5).
58. **A loan payment's expense stays an expense:** `update_transaction` refuses to change its kind, so a paid payment always has a loan expense behind it (§4.6).
59. **When everything left on the loans is overdue**, the debt-free date is today, and the screen shows what's overdue rather than a date in the past (§5.11).
60. **A payday rule goes into a cash, bank or e-wallet account**, because that's where its payslip is recorded (§4.8).
61. **Coming up (DSH-7) leaves out a payday recorded early**, since it already has its occurrence row (§4.1, §4.8).
62. **"Move money first"** gives each banner only its own part of the shortfall, so following every banner moves exactly enough (§5.11).
63. **A new loan's category** starts as the active expense category named Loan, when there is one. If the user's own "Loan" category is archived, the form starts empty (§4.5).
