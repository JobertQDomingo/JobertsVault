# Data Model: Jobert's Vault

| | |
|---|---|
| **Status** | Approved v1.3 |
| **Last updated** | 2026-11-20 |
| **Database** | SQLite (bundled with the app through `rusqlite`) |
| **Related docs** | [PRD.md](PRD.md), [ARCHITECTURE.md](ARCHITECTURE.md) |

This document describes every table and field, plus the rules for calculating balances, totals and periods. **Any change to the database schema must update this doc first, then add a new migration.**

---

## 1. Global conventions

| Topic | Rule |
|---|---|
| **Money** | Stored as `INTEGER` in **minor units** (e.g. cents), as an `i64` in Rust. Floating-point numbers are never used for money, anywhere. |
| **Amount sign** | `transactions.amount` is always **positive**. The transaction's `kind` decides whether money goes in or out. |
| **Decimal places** | `currency_decimals` (0 or 2) decides how minor units are displayed. It **can only be changed while no amount is stored**: no transactions, no recurring rules, no budgets, no goals, no loans, no check-in focus with an amount (§4.9), and no account with an opening balance other than 0 or a credit limit. Changing it afterwards would change the meaning of every stored amount. |
| **Dates** | `TEXT` in `YYYY-MM-DD` format (ISO 8601), using the user's local calendar. This format sorts correctly as plain text. No time of day is stored. |
| **Timestamps** | `created_at` / `updated_at` are `TEXT` in ISO 8601 UTC (`2026-10-26T09:30:00Z`). They're for record-keeping only and are never used in calculations, with one exception: a card's `created_at`, as a local date, is the day it was added, which the Bills stat's `overdue` needs (§5.20). Services receive the current time as a `now` value, the same way they receive `today`. |
| **IDs** | `INTEGER PRIMARY KEY`, assigned by SQLite. |
| **Booleans** | `INTEGER` 0 or 1, with a `CHECK` constraint. |
| **Enums** | `TEXT` with a `CHECK (... IN (...))` constraint. The values are lowercase `snake_case`. |
| **Table mode** | Every table is declared `STRICT`, so SQLite rejects a value of the wrong type. |
| **Connection settings** | `PRAGMA foreign_keys = ON`, `PRAGMA journal_mode = WAL`, `PRAGMA busy_timeout = 5000`, `PRAGMA synchronous = NORMAL` (as in ARCHITECTURE §4.3) |
| **Deleting records** | Accounts, categories and payment methods that are in use are **archived instead of deleted**. Unused ones can be deleted. Foreign keys use `ON DELETE RESTRICT`, so the database itself blocks a delete that would break history. A loan follows its own rule (§4.5). A check-in is never edited or deleted (§4.9). |
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
checkins *───0..1 categories, accounts, loan_payments  (its focus)   (Phase 5)
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
| `checkins` | 5 | One row per finished check-in of a week, month or year: its stat snapshot and its focus |

Phase 4 also adds two columns: `transactions.payslip_id` (§4.7) and `recurring_rules.is_payday` (§4.8). Phase 5 adds two to `goals`: `moved_on` and `moved_from` (§4.4).

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
| `spend_soft`, `spend_mid`, `spend_hard` | INTEGER | card only | Phase 4 (CC-10, migration `0004`): the spending levels for a statement cycle, `0 < soft < mid < hard`. NULL for every kind except a credit card. A new card starts at 3,000, 4,000 and 5,000 in the main currency unit (`3000 × 10^currency_decimals` minor units), and `0004` gives every existing card the same. |
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

Phase 4's migration `0004` adds the spending levels. `ADD COLUMN` can't add a table CHECK, so the whole rule sits on the last column, whose CHECK may name the others. They stay nullable even for a card, since the CHECK is tested on the existing rows as the column is added; the service always writes all three for a card.

```sql
ALTER TABLE accounts ADD COLUMN spend_soft INTEGER;
ALTER TABLE accounts ADD COLUMN spend_mid INTEGER;
ALTER TABLE accounts ADD COLUMN spend_hard INTEGER CHECK (
  (spend_soft IS NULL AND spend_mid IS NULL AND spend_hard IS NULL)
  OR (kind = 'credit_card' AND spend_soft IS NOT NULL AND spend_mid IS NOT NULL
      AND spend_hard IS NOT NULL AND 0 < spend_soft AND spend_soft < spend_mid
      AND spend_mid < spend_hard)
);
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

**Merging (CAT-4):** in one database transaction, move every transaction, recurring rule, loan (§4.5) and check-in focus (§4.9) from category A to active category B (both must be the same kind), then delete A. A's budgets are deleted with it (§4.3).

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

## 4. Tables: Phase 3 (migration `0002_planning.sql`), Phase 4 (migration `0003_loans_payroll.sql`) and Phase 5 (migration `0005_checkins.sql`)

§4.1–4.4 are Phase 3. §4.5–4.8 are Phase 4. §4.9 is Phase 5.

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
- **Phase 5 (CHK-3, migration `0005`):** two nullable columns record the last time GOL-7 moved the date by itself (§5.17), so a check-in can show it:

  ```sql
  ALTER TABLE goals ADD COLUMN moved_on   TEXT CHECK (moved_on GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]');  -- the local date of the move
  -- the target date it had before; a column's CHECK may name an earlier column, so both are set or neither is
  ALTER TABLE goals ADD COLUMN moved_from TEXT CHECK ((moved_from IS NULL) = (moved_on IS NULL)
    AND (moved_from IS NULL OR moved_from GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'));
  ```

  GOL-7's move sets both, in the same database transaction as the save that caused it. An edit that changes the target date by hand clears both. The table refuses one without the other. GOL-7 only moves a goal with a target date, so `moved_from` always has one. They never feed a calculation.

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
- **Recording early (PAY-4):** a payday date after today and up to 3 days on (today Nov 13: Nov 14–16) that has no occurrence row yet can be recorded early, if the rule isn't paused and hasn't ended. In one database transaction, the service saves the payslip and inserts that date's occurrence row as `posted`, pointing at the gross row. When the date arrives, processing finds the row and passes over it (the unique key, §4.2), so the payday is never created twice. The row counts toward `max_occurrences`, like any processed date. The rule's next date moves on to the payday after it, so a schedule edit can start there. If the date arrived while the form was open, the save first queues it, then settles it as a due payday.

### 4.9 `checkins` (Phase 5)

One row for every week, month or year whose check-in was finished (CHK-1). The row holds a **snapshot** of the level and the six stats on the day it was finished, and the **focus** the user picked.

```sql
CREATE TABLE checkins (
  id                    INTEGER PRIMARY KEY,
  kind                  TEXT    NOT NULL CHECK (kind IN ('week','month','year')),
  period_start          TEXT    NOT NULL CHECK (period_start GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
  period_end            TEXT    NOT NULL CHECK (period_end   GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
  completed_on          TEXT    NOT NULL CHECK (completed_on GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
  -- snapshot, as of completed_on
  level                 INTEGER NOT NULL CHECK (level BETWEEN 0 AND 10),
  stat_cashflow         INTEGER CHECK (stat_cashflow BETWEEN 0 AND 100),   -- NULL = didn't apply
  stat_bills            INTEGER CHECK (stat_bills    BETWEEN 0 AND 100),
  stat_card             INTEGER CHECK (stat_card     BETWEEN 0 AND 100),
  stat_debt             INTEGER CHECK (stat_debt     BETWEEN 0 AND 100),
  stat_savings          INTEGER CHECK (stat_savings  BETWEEN 0 AND 100),
  stat_habit            INTEGER CHECK (stat_habit    BETWEEN 0 AND 100),
  debt_free_on          TEXT    CHECK (debt_free_on GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),  -- §5.11, NULL = nothing left
  -- focus (§5.22); every focus column is NULL when the row set none
  focus_kind            TEXT    CHECK (focus_kind IN ('category_cap','spending_cap','card_level','loan_on_time','logging')),
  focus_category_id     INTEGER REFERENCES categories(id)    ON DELETE RESTRICT,
  focus_account_id      INTEGER REFERENCES accounts(id)      ON DELETE RESTRICT,
  focus_loan_payment_id INTEGER REFERENCES loan_payments(id) ON DELETE SET NULL,
  focus_target          INTEGER CHECK (focus_target > 0),
  focus_start           TEXT    CHECK (focus_start  GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
  focus_end             TEXT    CHECK (focus_end    GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
  focus_result          TEXT    CHECK (focus_result IN ('kept','missed','void')),  -- NULL = not graded yet
  focus_actual          INTEGER,
  created_at            TEXT    NOT NULL,
  updated_at            TEXT    NOT NULL,
  UNIQUE (kind, period_start),
  CHECK (focus_kind IS NOT NULL OR (
    focus_category_id IS NULL AND focus_account_id IS NULL AND focus_loan_payment_id IS NULL
    AND focus_target IS NULL AND focus_start IS NULL AND focus_end IS NULL
    AND focus_result IS NULL AND focus_actual IS NULL)),
  CHECK (focus_kind IS NULL OR (kind <> 'year' AND focus_start IS NOT NULL AND focus_end IS NOT NULL)),
  CHECK (focus_kind <> 'category_cap' OR focus_category_id IS NOT NULL),
  CHECK (focus_kind <> 'card_level'   OR focus_account_id  IS NOT NULL)
) STRICT;

CREATE INDEX idx_checkins_completed ON checkins(completed_on);
```

| Column | Notes |
|---|---|
| `kind`, `period_start`, `period_end` | The period reviewed (§5.19). **The unique key means a period is never reviewed twice.** |
| `completed_on` | The local date Finish was pressed. It's a date, not a timestamp, because the Habit stat and the check-in streak use it (§5.20, §5.23). |
| `level`, `stat_*`, `debt_free_on` | What `get_vault_status` and the loan figures gave that day. A snapshot is **never recalculated**: editing an old transaction changes the live figures, not a past check-in. They're only for showing change: the hexagon's outline, the trend lines, "the debt-free date moved" and Your year. |
| `focus_kind` | Which kind of target (§5.22). The row for a year never has a focus, and in a merged check-in only one row has it. |
| `focus_category_id` | The category of a `category_cap`. **A focus puts its category in use**, so it's archived rather than deleted, and merging categories moves it (§3.2). |
| `focus_account_id` | The card of a `card_level`. A focus puts that account in use too. |
| `focus_loan_payment_id` | The payment of a `loan_on_time`. If the payment is removed (the loan is edited or deleted), `ON DELETE SET NULL` clears it: the focus is left out of the active focuses (§5.22) and is graded `void`. |
| `focus_target` | Minor units for `category_cap`, `spending_cap` and `card_level`, a number of days for `logging`, NULL for `loan_on_time`. |
| `focus_start`, `focus_end` | The week or month the focus runs for: the one that contains `completed_on` (§5.22). |
| `focus_result`, `focus_actual` | Set once, when a later check-in grades the focus (§5.22). `void` means it couldn't be graded. `focus_actual` is what was reached, in the target's unit, and NULL for `loan_on_time` and `void`. |

- **Finishing a check-in** writes its rows in one database transaction: one row per kind that was due, all with the same `completed_on` and snapshot, and the grade of every focus that has ended and isn't graded yet (§5.22).
- **A check-in row is never edited afterwards, except to grade its focus, and never deleted.** There's no command for either.
- **The table's checks hold the row's shape** as well as its values, since a stored row can't be put right later: every date is `YYYY-MM-DD`, a row without a focus has no focus fields, a year row has no focus, a focus has its `focus_start` and `focus_end`, a `category_cap` has its category, and a `card_level` has its card. A `loan_on_time` focus may lose its payment (`ON DELETE SET NULL`), so its payment isn't required.
- A focus target that's an amount locks `currency_decimals` (§1).
- A restore brings the check-ins with the rest of the data.

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
  - `utilization = amount_owed ÷ credit_limit`, shown as a whole percent rounded half up. The Card stat (§5.20) uses that same whole percent, so the stat agrees with the screen.
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
- **Spending level (CC-10, Phase 4):** see §5.18.

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

- **Phase 4 (GOL-5, GOL-6):** the target date is required, and the suggested date and the earliest one a goal may have come from §5.17. A goal saved before Phase 4 without a date still shows its progress, with no monthly amount, until it's edited.

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
% paid       = ⌊total paid × 100 ÷ (total paid + total left)⌋  (100 once nothing is left)
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
- **The dashboard line (LOAN-11)** shows `total left` and `debt-free on`, and is hidden when nothing is left. When everything left is overdue, it says so instead of the date.
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
C₀            = the latest closing date on or before today: the latest statement
D₀            = the first due date after C₀ (min(due_day, the month's last day)): the date that pays it
due dates     = min(due_day, the month's last day), every month from D₀ to the horizon's end
C(D)          = the latest closing date before the due date D: the statement that D pays
```

Walk forward from today, starting from what's owed now (`−balance`, §5.1, so below 0 when the card is in credit, which later charges use up):
- **The latest statement** is billed what was owed at the end of C₀ (`max(0, −balance as of C₀)`), less the transfers into the card and the income on it dated after C₀ up to today, never below 0 and never more than what's owed now. It's due on D₀, or today once D₀ has passed (anything overdue counts as due today, §5.15).
- **On every later due date D:** the bill is what's projected to be owed at the end of C(D). A due date whose C(D) is the one before it already paid bills nothing.
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
| A recurring rule's dates, other than a payday: `pending` ones (as today, even while the rule is paused), and the dates still to come in the horizon for a rule that isn't paused and hasn't ended, with any date up to today not yet processed counting as today | Its template on that date, counted like a future-dated transaction |
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
- **With no payday rule**, there are no pay windows: the forecast returns none and says how to set one up (SAFE-6). A paused payday rule is still a payday rule: its dates make no window, so there's one window to the horizon's end (§9).
- **A payslip already recorded** in the current window is in `money now`. The Forecast lists it as received, by its own date, without adding it again. In the Months view, the current month lists what was received since the month's real start, though its row starts today.
- **The Months view (SAFE-7)** groups the same movements by the app's months (§5.3): from the current month, starting today, to the horizon's end, with the same `start`, `in`, `out` and `left`. Safe to spend always comes from the pay windows.

### 5.16 Before you borrow (LOAN-9, LOAN-10, Phase 4)

A **draft loan** is either the rows the user typed in the Add loan form (LOAN-10), or a payment amount, a number of payments and a first payment date (LOAN-9). The second form gives monthly dates from the first one, as a monthly rule does (§4.1: the *n*-th date is `first + n` months, with the day clamped to the month's end), so the day of the month is the first payment's.

The draft's payments are added as unpaid payments of an active loan, and nothing is saved. The result is:
- the **debt-free date** before and after (§5.11), and how many days later it is: after − before, counting from today when nothing is left
- **safe to spend** before and after (§5.15), none with no payday rule
- **each pay window's** `left` before and after, so the screen can say which windows it hurts and which would run short
- **short by**, on each window the draft makes short: it runs short with the draft (§5.15) and doesn't without it. It's `−(its lowest running total with the draft)`.

```
debt-free after = max(debt-free before (today when none), the draft's latest date)
days later      = debt-free after − (debt-free before, or today)
short by(k)     = −low_after(k)   when window k runs short with the draft and not without it
```

### 5.17 Smart goal target date (GOL-5, GOL-6, Phase 4)

How much the user could set aside each month for a goal, and so the earliest realistic target date. It's worked out **as if the goal already exists**, so its account isn't a counted account (§5.15).

```
saving dates = today + k months, for k = 1, 2, …   (the day clamped to the month's end, as §5.7 counts months)
n(d)         = the number of saving dates on or before day d
others(d)    = Σ over every other active goal that isn't achieved and has a target date:
               its monthly_needed (§5.7) × min(n(d), its months left (§5.7))
run(d)       = the forecast's running total (§5.15), day by day over the horizon, leaving out
               transfers from the counted accounts into a dated, unachieved goal's account
monthly room = 0 when any day of the horizon has run(d) < 0, else
               max(0, the lowest of ⌊(run(d) − others(d)) ÷ n(d)⌋ over every day d with n(d) ≥ 1)
left to save = target_amount − max(0, the account's balance today)
earliest     = today + ⌈left to save ÷ monthly room⌉ months   (the day clamped to the month's end)
```

- **The monthly room** is the most the user could move into the goal's account on every saving date for the next 12 months without any day's running total going below 0. Other goals with a date are already taking their monthly amount on their first months-left saving dates, as many as §5.7 divides by, so two goals never count on the same money. A goal saved before Phase 4 without a date asks for nothing, so it takes nothing.
- **The earliest date** is when saving the monthly room reaches the target. Saving §5.7's `monthly_needed` for that date is then never more than the monthly room. Past the 12-month horizon, the same monthly room is assumed to continue.
- **The suggestion** is the earliest date. A new goal's date field starts on it once the target and account are chosen, and the user may pick a later one.
- **A target date is required (GOL-6).** A goal with none is refused on add and on every edit (`targetDate`: "Enter a target date"), clearing it included. The column stays nullable only for goals saved before Phase 4, which keep working until they're edited.
- **The check (GOL-6):** a target date before the earliest is refused (`targetDate`: "That's too soon. Even setting aside the most you could, about ₱650.00 a month, the earliest you can reach this is Mar 5, 2027."). It runs when a goal is added, and when an edit changes the date. An edit that keeps the date it already has is never refused, so a goal can't become unsaveable by itself as bills change.
- **No payday rule:** there's no forecast (§5.15), so the date can't be judged. Adding a goal, and an edit that changes its date, are refused (`targetDate`: "Mark your pay as a payday first, so the app can work out a realistic date."). A paused payday rule still counts (§5.15). An edit that keeps its date, or changes only the name or target, isn't refused for this.
- **Order of the checks:** no date, then no payday rule, then too soon, so the message names the first thing to fix.
- **A monthly room of 0** (some window already runs short, even before the first saving date, or every peso is spoken for) makes every date too soon while there's something left to save, so the goal can't be added yet: "You can't set anything aside each month right now, so no date is realistic yet. Free up money first."
- **Nothing left to save** (the account already holds the target): any date today or later is allowed, and the suggestion is today. It still needs a date and a payday rule.
- Budgets still don't hold anything back, and Safe to spend doesn't set money aside for goals (PRD §3). The monthly room is advice for the goal only.

**Goals follow the forecast (GOL-7).** After these saves, in the same database transaction:
- a transaction created or updated as an expense or a transfer, whatever screen it comes from (quick-add, Edit, Duplicate, Mark paid)
- a recurring rule created or updated as an expense or a transfer (an income rule, a payday included, only adds money, §9 item 107)
- a loan created or updated

each active goal that isn't achieved and has a target date and something left to save is checked again, newest first:

```
room(goal)     = §5.17's monthly room, worked out after the save, with the other goals at their dates now
                 (a goal moved earlier in this pass counts at its new date)
late(goal)     = its months left (§5.7) is 0, or its monthly_needed (§5.7) > room(goal)
if room(goal) is 0  → the date stays, and the goal is behind
else if late(goal)  → its target_date becomes earliest(goal), §5.17's earliest date with room(goal)
```

- **The save always goes ahead.** The goal is what changes, never the expense.
- **Phase 5:** a move also records `moved_on` (today) and `moved_from` (the date it had) on the goal, for the check-in's Goals step (§4.4, §5.19).
- **Only a date that's no longer realistic moves**, and only later. A save that leaves every goal on track changes nothing, and a goal never moves earlier by itself.
- **Judged by month, not by day.** The earliest date moves a day later every day, so comparing it with the target date would make a goal at its suggested date late the next day. A goal is on track while its monthly amount fits in the monthly room; on the day a date is set this is the same test as the date being no sooner than the earliest (§9 item 99).
- **Newest first**, so a goal added earlier keeps its claim on the monthly room: the newest goal gives way first, and moving it later lowers its monthly amount, which leaves more room for the older goals checked after it (§9 item 100).
- **Behind** (`list_goals`' `behind`): there's a payday rule, something left to save, and either the monthly room is 0 or the goal is late (`late` above, with every other goal at its date now). Time passing, a payslip below the estimate, a deleted income, or an income rule lowered, ended or moved to an account that isn't counted can do this without a save that moves it. The card says so; nothing else changes.
- With no payday rule, nothing is checked or moved.
- **A goal's own deposits aren't spending.** A future-dated transfer, or a transfer rule's dates, from the counted accounts into the account of a goal with a date (or the goal being added) is the saving its monthly amount already asks for, so the monthly room leaves it out rather than counting it twice. Safe to spend still sees the money leave (§9 item 102).
- Recurring dates posted by processing (Post and Edit & post included) and payslips aren't saves that move goals: their dates were already in the forecast. Income isn't either, as a transaction or as a rule. Deleting a row never moves a goal either; a goal it leaves behind shows as behind.

### 5.18 Card spending levels (CC-10, CC-11, Phase 4)

```
spent = the card's cycle spending (§5.5): Σ its expenses dated in the current statement cycle, up to today
level = hard  when spent ≥ spend_hard
        mid   when spent ≥ spend_mid
        soft  when spent ≥ spend_soft
        none  otherwise
```

- **"Reached" means equal or more.** Exactly ₱3,000.00 spent is the soft level.
- **Archived cards** show no level; they take no new spending.
- **The check before saving (CC-11):** for a new or edited expense on a card, dated in the card's current cycle and up to today, the backend works out the level **after** the save: `spent` plus the new amount, less the old amount when an edit keeps it on the same card. When that's `hard`, the form asks "Spend anyway?" first. When it reaches `soft` or `mid` for the first time (the level after is higher than before), it saves and shows a warning toast. A future-dated expense, one dated in an earlier cycle, a save that doesn't raise this cycle's spending (an edit of only the note, or one that lowers the amount), and recurring dates posted by processing (Post and Edit & post included, as in §5.17) are never asked about. `preview_card_spend` returns none for those, and for an amount of 0 or less, a missing account or one that isn't an active card, so the save names any error.
- The levels are warnings only: like the credit limit, they never block a save (CLAUDE.md).
- They're amounts, but every card already has a credit limit, which locks `currency_decimals` (§1), so they add nothing to that lock.

### 5.19 Check-in periods (CHK-1 to CHK-5, Phase 5)

```
week   = the week (§5.3) that ended most recently before today
month  = the month (§5.3) that ended most recently before today
year   = the calendar year before today's, only while today is in January

due(p) = there's no checkins row for (p.kind, p.start)
         AND some transaction is dated on or before p.end

shown  = month when the month is due, else week
```

- **A check-in is due** when the week or the month is. Every kind that's due is settled by the one check-in (§4.9). The year is never due by itself: it's settled, with its extra step, by a check-in finished in January.
- **Only the newest completed period of each kind is ever due.** Older ones that were never reviewed are skipped and get no row.
- **A new database** has no transaction, so nothing is due. An existing one gets its first check-in at the first start after the update that brings the feature.
- **The figures** of the Money in and out step are the shown period's totals (§5.2), compared **in full** with the period before it (§5.4: a period that's over is always compared in full).
  - The 3 largest expense categories are ranked like the breakdown (§5.10), each with its change (§5.4).
  - **Budgets kept or broken:** each weekly budget for a week, or each monthly budget for a month (§4.3), measured over the shown period: kept when `spent ≤ amount` (§5.6), else `over = spent − amount` says by how much. Yearly budgets show in Your year.
- **The Card step** shows each active card as it is today (§5.5, §5.18), not as it was at the period's end.
- **The Loans and bills step** shows the loan payments whose expense is dated in the shown period and their total, what's overdue today, `total left` and `debt-free on` (§5.11), and the forecast as it is today (§5.15). **The debt-free date moved** when it differs from `debt_free_on` on the latest earlier check-in row. Nothing is said when there's no earlier row, when either date is none, or when everything left is overdue, now or on that earlier row (the date is then that day, §5.11), so a stored date on or before its row's `completed_on` never counts as moved.
- **The Goals step** lists every active goal with its progress (§5.7), marks the ones that are Behind (§5.17), and names each goal whose `moved_on` (§4.4) is on or after the shown period's start. It shows in a check-in whose month is due, when there's at least one active goal. When only the week is due, it shows only if some goal is Behind or was moved since the week's start.
- **The Balance check step (CHK-4)** shows when the month is due. It lists every active account, and any archived one whose balance isn't 0, with its balance today (§5.1), a card as its amount owed. The answers are held by the screen and never sent or stored.
- **Changing the week start or `month_start_day`** moves the periods, so the latest one may have no row and be reviewed again, and the on-time count and check-in streak (§5.20, §5.23) start over. That's accepted: the settings rarely change.
- **Your year (CHK-5)** shows when the year is due: the year's totals compared in full with the year before, its 3 largest expense categories, each yearly budget kept or broken, the loans whose last payment was paid in that year, and the `level` on the first and the last check-in row with `completed_on` in that year (none when there's no such row).

### 5.20 Stats (LVL-1, Phase 5)

Six whole numbers from 0 to 100, worked out for `today`. Every division rounds down, toward minus infinity when the number divided is below 0 (Rust's `div_euclid`, not `/`), and nothing here uses a float. A stat is **none** ("—") when it doesn't apply.

Shared figures:

```
months    = the 3 latest completed months (§5.3), leaving out any that ended before the earliest transaction's date
            (so 0 to 3 months)
I, E      = income and expense over those months (§5.2)
D         = the payslip deductions in E: its expenses with payslip_id set (§4.7)
borrowed  = Σ amount_borrowed of the new loans (is_new = 1, §4.5) whose borrowed_on is in those months
take_home = max(0, I − D − borrowed)          -- pay after deductions, with borrowed money left out
spending  = E − D                             -- what left the accounts, loan payments included
avg_take_home, avg_spending = each ÷ the number of months
earned(m) = net(m) − borrowed(m), for one month m   -- income − expense, with borrowed money left out
money     = max(0, Σ balance(a, today) over every cash, bank and e-wallet account, archived ones included)   (§5.1)
logged    = the number of dates from today − 29 to today with at least one transaction of any kind
overdue   = the number of unpaid loan payments dated before today (§4.6)
          + the number of active cards whose latest statement (§5.14) still has something to pay,
            whose due date D₀ is before today, and whose closing date C₀ isn't before the day the
            card was added
paid late(n) = the number of loan payments whose expense is dated after their due_date
               and within the last n days (today − (n − 1) to today)
```

| Stat | Score | None when |
|---|---|---|
| **Cashflow** | `clamp(50 + (take_home − spending) × 500 ÷ take_home, 0, 100)`, and 0 when `take_home` is 0. Breaking even is 50, leaving 10% of take-home pay unspent or more is 100, and overspending by 10% or more is 0. | There's no completed month, or `I` and `E` are both 0 |
| **Bills** | `max(0, 100 − 25 × overdue − 10 × paid late(90))` | There's no loan (paid-off and archived ones count as loans) and no active credit card |
| **Card** | For each active card: `(L × 60 + U × 40) ÷ 100`, where `L` is 100, 70, 40 or 10 for the level none, soft, mid or hard (§5.18), and `U` comes from its utilization `u`, a whole percent (§5.1): 100 when `u ≤ 10`, 0 when `u ≥ 90`, else `(90 − u) × 100 ÷ 80`. With several cards, the lowest score (the first by account id on a tie). | There's no active card |
| **Debt** | 100 when `total left` is 0 (§5.11). Otherwise 95 when nothing is due this month, else 0 when `avg_take_home` is 0, else `clamp(95 − share × 95 ÷ 90, 0, 95)`, where `share = due(current month) × 100 ÷ avg_take_home` and `due(current month)` is §5.11's (LOAN-7): 0 when loans take 90% of an average month's take-home pay or more. | Something is left on a loan and there's no completed month |
| **Savings** | `cushion − 15 × behind`, never below 0, where `cushion = min(100, money × 100 ÷ (3 × avg_spending))` (when `avg_spending` is 0: 100 if `money > 0`, else 0) and `behind` is the number of active goals that are Behind (§5.17). Three months of spending covered is 100. | There's no completed month |
| **Habit** | `days + checkins`, where `days = min(70, logged × 70 ÷ 25)` and `checkins = on_time × 30 ÷ 4`, and `on_time` counts, of the 4 latest completed weeks, those whose week row (§4.9) has `completed_on ≤ period_end + 3`: finished on the day it came due or within the 2 days after. | Never |

- **The check-in being finished counts as finished today** in `get_checkin`'s `status` and in the snapshot Finish stores (§4.9), for `on_time` here and for both streaks (§5.23). Without that, a stored Habit could never reach 100. Elsewhere the live figure counts only rows that exist, and while a check-in is due nothing else is on screen to show it.
- **Each stat names its reason**, a code for the main thing holding it down: the first that applies below. A stat at 100, or one that's none, has no reason. The frontend turns the code into the sentence. No amount or name is ever logged (ARCHITECTURE §4.4).

| Stat | Reason codes, the first that applies |
|---|---|
| **Cashflow** | `no_take_home` when `take_home` is 0; `spending_over_income` when `spending > take_home`; else `little_left` |
| **Bills** | `loan_overdue` when an unpaid loan payment is dated before today; `card_overdue` when a card's statement is unpaid past its due date; else `paid_late` |
| **Card** | For the card that gives the score: `card_past_hard`, `card_past_mid` or `card_past_soft` (its level) when its level costs at least as many points as its utilization, `(100 − L) × 60 ≥ (100 − U) × 40`; else `card_high_use` |
| **Debt** | `loans_due` when something is due this month; else `loans_left` |
| **Savings** | `goal_behind` when `15 × behind > 100 − cushion`; else `cushion_low` |
| **Habit** | `few_days_logged` when `70 − days ≥ 30 − checkins`; else `late_checkins` |

- **Days logged go by a transaction's `date`**, never by when it was typed in (§1). A day with no spending at all isn't a logged day, which is why 25 days of 30 is full marks, not 30.
- **Take-home pay, not gross.** A payslip records the gross as income and each deduction as an expense (§4.7), so the stats take the deductions off both sides: `take_home` and `spending`. The difference between them is the same as `I − E`; only what it's measured against changes.
- **Borrowed money isn't earnings.** A new loan's money is logged as income (ADR 0011), so its `amount_borrowed` is taken off, in the month it was borrowed. Without that, borrowing would raise Cashflow and reach level 4. It works only when the loan has an amount borrowed, and assumes the money was logged as income.
- **The Debt stat uses the month's scheduled payments**, paid ones included, so it doesn't jump as each payment is marked paid, and uses average past take-home pay rather than the forecast, so it works without a payday rule.
- **Savings counts goal accounts**, unlike Safe to spend (§5.15): money put aside is still a cushion.
- **Late means dated after the due date, with no grace.** A payment logged a day after it was made keeps its real date on the expense (§4.6).
- **The scales were chosen with the owner on 2026-11-17** (§9 items 128–135), wide enough that each stat moves as things improve rather than sitting at 0 or 100. They're fixed rules, not settings. A change updates this section first.

### 5.21 Levels and the next best move (LVL-2, LVL-4, Phase 5)

The **level** is the highest `n` from 0 to 10 for which every condition of levels 1 to `n` holds today. It's worked out live, like a balance, so it can go down.

Each condition has a code, and a level's conditions are checked in the order given. The gap is the figure the next best move and the checklist show.

| Lv | Name | Conditions, in order: `code`, the rule (gap) |
|---|---|---|
| 1 | Tracker | `logged_20`: `logged ≥ 20` (§5.20) (gap: `20 − logged` days still to log) |
| 2 | Planner | `payday_rule`: there's a payday rule (a paused one counts, §5.15); `budget`: there's at least one budget |
| 3 | Bill Keeper | `nothing_overdue`: `overdue = 0`; `none_late`: `paid late(30) = 0` (§5.20) |
| 4 | In the black | `earned_month`: the latest completed month has `earned > 0` (§5.20) |
| 5 | Steady | `earned_3_months`: each of the 3 latest completed months has `earned > 0` (gap: how many of them in a row, counting back from the latest, have it: "1 of 3 so far"); `no_short_day`: the forecast has no short day (§5.15) |
| 6 | Card under control | `card_under_mid`: for every active card, in each of its 3 latest completed statement cycles (§5.5), the card's expenses dated in that cycle total less than its `spend_mid` as it is today. A cycle with no expenses is under. (gap: the first card that fails, by account id, and how far its current cycle's spending (§5.18) is over its `spend_mid`, 0 when it's under) |
| 7 | Two weeks ahead | `two_weeks_saved`: `money × 2 ≥ avg_spending`, with `avg_spending > 0` (§5.20) (gap: `(avg_spending + 1) ÷ 2 − money`) |
| 8 | Debt-free | `loans_paid`: `total left = 0` (§5.11) (gap: `total left`); `loan_free_90`: the loan-free streak (§5.11) is 90 days or more, or there's none (gap: the days left) |
| 9 | One month ahead | `month_saved`: `money ≥ avg_spending`, with `avg_spending > 0` (gap: `avg_spending − money`) |
| 10 | Vault Master | `stats_80`: every stat that applies is 80 or more (gap: the lowest stat); `no_goal_behind`: no active goal is Behind; `loan_free_365`: the loan-free streak is 365 days or more, or there's none (gap: the days left) |

- **A condition about something the user doesn't have counts as met:** with no active card, level 6 asks nothing.
- **A condition that needs history waits for it:** levels 4, 7 and 9 need one completed month, and level 5 needs three. Until then they aren't met.
- **Level 0** is the start: no level's conditions are met yet. It has no name.
- **The ring** on the sidebar chip (LVL-5) is the number of the next level's conditions that hold, out of how many it has (1 to 3, from the table: one `card_under_mid` covers every card). At level 10 it's full: level 10's 3 of 3.
- **A gap is never below 0**, so a gap that's a distance is 0 once met. Two gaps aren't distances: `earned_3_months`' is the months so far, and `stats_80`'s is the lowest stat's score. `card_under_mid`'s gap also names its card. Some conditions have no gap while there's no figure for one yet: `two_weeks_saved` and `month_saved` while `avg_spending` is 0, and `loan_free_90` and `loan_free_365` when there's no loan-free streak. A tie for the lowest stat goes to the first in §5.20's order.
- **The order** puts two weeks of savings before debt-free and one month after, so an emergency doesn't force a new loan (PRD §14). Three months stays the long-run aim: level 10 needs the Savings stat at 80, which is about 2.4 months of spending.
- **Level 1 asks for 20 logged days, not Habit's 25.** Every level rests on level 1, so a week away from the app shouldn't drop them all to 0.
- **`levelChange`** compares the level today with the latest earlier check-in row's: `up`, `down` or `same`. With no earlier row it's `same`, so the first check-in has no level-up moment. When it's `down`, the reason given is the first unmet condition of the level above today's, by its code.

**The next best move** is one code with its figures, the first of these that applies:

| # | When | Code | Move |
|---|---|---|---|
| 1 | A loan payment is overdue | `pay_overdue_loan` | Pay it: the earliest overdue payment (by due date, then id), with its loan and amount |
| 2 | An active card's statement is unpaid past its due date, as `overdue` counts it (§5.20) | `pay_overdue_card` | Pay that card's statement amount (§5.14). With several, the one with the earliest due date, then the lowest account id |
| 3 | The level is below 10 | the condition's code | The first unmet condition of the next level, with its gap from the table where it has one |
| 4 | The level is 10 | the stat's reason code | The reason of the lowest stat (§5.20), the first in §5.20's order on a tie. None when every stat that applies is 100, and the screen then says only "You're at the top level." |

### 5.22 Focus (CHK-7, CHK-8, Phase 5)

A focus runs for the week or month that **contains the day the check-in was finished**: the current month when the month was due, else the current week (§5.3). Spending already recorded since that period started counts toward it.

**Everyday spending** is what a cap counts: the expenses dated in a range, on any account, **leaving out** those that pay a loan payment (`loan_payments.transaction_id`), that a recurring date posted (`recurring_occurrences.transaction_id`), or that are payslip deductions (`payslip_id` set). Those are bills, which Safe to spend has already set aside (§5.15).

**A cap** is a `category_cap` or a `spending_cap`. A `card_level` focus isn't one: the card's spending levels already warn (§5.18).

**Offers.** Each kind is tied to a stat. The kinds that apply are ordered by their stat's score today, lowest first (ties in the order below), and the first 3 are offered. A kind whose stat is none ranks as if it were 100. Logging always applies, so there's always at least one.

**Every offer can still be kept on the day it's offered.** A check-in can be finished late in the period, and what was recorded since the period started already counts, so a target is fitted to what's left of the period, or the offer is left out:

```
recorded   = what's already recorded for the focus period, future-dated rows included (grading counts
             them once their date arrives, and Safe to spend has already set them aside, §5.15)
days left  = the days from today to the focus period's end, today included
reach      = the dates before today in the focus period with at least one transaction + days left
```

| Kind | Stat | Applies when | Target |
|---|---|---|---|
| `loan_on_time` | Bills | An active loan has an unpaid payment due in the focus period, today or later: the earliest one | That payment |
| `card_level` | Card | An active card's `peak` isn't at hard (§5.18), where `peak` is the highest recorded cycle spending, over every statement cycle with a day in the focus period, as grading measures it: with several, the one with the lowest Card score (ties by account id) | The lowest of its three levels that's above its `peak` |
| `spending_cap` | Cashflow | The focus is for a week, there's a payday rule, and `per day` (§5.15) is above 0 | The week's recorded everyday spending `+ per day × days left` |
| `category_cap` | Cashflow | An active expense category's everyday spending in the period reviewed rose over the period before it, where that earlier amount is above 0, and its recorded everyday spending in the focus period isn't above that earlier amount: the one with the largest rise in amount, not percent (ties by category id) | Its everyday spending in that earlier period |
| `logging` | Habit | Always | `min(5, reach)` days for a week, `min(20, reach)` for a month |

**Grading.** A focus is graded once `focus_end` is before today, by the next check-in that's finished. Finishing grades **every** ended focus that isn't graded yet. The Start step shows, of those, the one that ended latest, and nothing when there are none, so a focus graded at an earlier check-in never shows again. `get_checkin` works those grades out for the Start step without storing them; Finish works them out again and stores them.

| Kind | `focus_actual` | Kept when |
|---|---|---|
| `category_cap` | The category's everyday spending from `focus_start` to `focus_end` | `actual ≤ target` |
| `spending_cap` | Everyday spending from `focus_start` to `focus_end` | `actual ≤ target` |
| `card_level` | The highest, over every statement cycle of the card with a day in the focus period, of its expenses dated in that cycle up to `focus_end` | `actual < target` (reaching a level counts as reaching it, §5.18) |
| `loan_on_time` | None | The payment has an expense dated on or before its `due_date` |
| `logging` | The number of dates in the focus period with at least one transaction | `actual ≥ target` |

- A `loan_on_time` focus whose payment was removed is `void`. A `void` focus counts as neither kept nor missed.
- A grade is stored once and never worked out again (§4.9).

**State (CHK-8).** While a focus is active (`focus_start ≤ today ≤ focus_end`), `get_vault_status` returns its progress: the `actual` so far, by the grading table's rule counted to today. A `loan_on_time` focus whose payment was removed isn't returned. A cap also has its `percent` (`actual × 100 ÷ target`, rounded down) and a state, which sets its bar's color: amber when `near`, red when `over`.

```
over  when actual > target
near  when actual × 100 ≥ target × 80
ok    otherwise
```

When a save raises a cap's state, the app shows a warning toast. It never blocks the save or asks to confirm (ADR 0012). A week's focus and a month's can be active together, and each is shown.

### 5.23 Wins, personal bests and streaks (CHK-6, LVL-7, Phase 5)

**The win** is the first of these that applies, for the period shown (§5.19). With none, there's no win, and nothing takes its place.

| # | Win | When |
|---|---|---|
| 1 | The focus was kept | The focus the Start step shows is graded `kept` |
| 2 | A stat rose | A stat is 5 or more above its value on the latest earlier check-in row: the largest rise (ties in §5.20's order) |
| 3 | A category fell | An expense category's amount fell from the period before: the largest fall (ties to the lower category id), with the amount |
| 4 | Spending fell | The period's expense is below the period before's, which is above 0 |
| 5 | Loans paid on time | At least one loan payment was due in the period, leaving out paid-before ones (§4.6), and each has an expense dated on or before its `due_date` |

**Personal bests** look at the 12 latest completed periods of the shown kind, the shown one included, leaving out any that ended before the earliest transaction's date. With 4 or more of them, the shown period sets one when it has the **lowest expense** of them, or else the **highest net** of them, with no tie. Only one is named, the lowest expense first.

**Streaks:**

```
check-in streak = counting back from the latest completed week, the number of weeks in a row whose week row (§4.9)
                  has completed_on ≤ period_end + 3
focus streak    = counting back from the focus that ended latest, the number of graded focuses in a row
                  whose result is kept, passing over void ones
```

- **The latest week doesn't break the streak while it can still be on time:** until `period_end + 3` has passed, a week with no row yet is passed over rather than counted as missed.
- The check-in being finished counts as finished today, in its own Stats and level step and in its snapshot (§5.20).
- Neither streak feeds a stat or a level, beyond the on-time check-ins that Habit already counts (§5.20).

**History (LVL-8).** The trend lines use the 12 latest check-ins, one point per `completed_on` (a merged check-in's rows hold the same snapshot). The focus record is the 8 latest graded focuses, `void` ones left out.

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
| `currency_decimals` can't change once any amount is stored: a transaction, a recurring rule, a budget, a goal, a loan, a check-in focus with an amount, or an account's non-zero opening balance or credit limit (§1) | settings |
| A budget's category is an expense category, and a goal's account isn't a credit card | category / account |
| A goal's name is 1–50 characters and unique, ignoring case; its target amount is > 0 and ≤ the maximum; its target date, if any, is a real date (past dates are allowed until Phase 4's check below); a new goal can't use an archived account, but an edit may keep the one it has | name / targetAmount / targetDate / accountId |
| Phase 4 (GOL-6): a goal has a target date, on add and on every edit; adding one, or changing its date, needs a payday rule; and the date isn't before the earliest realistic date (§5.17), checked on add and when an edit changes the date | targetDate |
| Phase 4 (CC-10): a card's spending levels are each > 0 and ≤ the maximum, with soft < mid < hard; every other kind of account has none. A card sent without a level keeps the one it has, and a new card gets the default (§2) | spendSoft / spendMid / spendHard |
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
| Phase 5 (CHK-1): a check-in can be finished only while one is due (§5.19, `Conflict` otherwise: "This check-in is already done"). The backend works out which periods are due itself, and takes none from the frontend. | shown as a toast |
| Phase 5 (CHK-7): finishing a check-in needs a focus, and it must be one of the offers the backend would make now (§5.22): the same kind, category, account, loan payment, target, start and end ("Pick one of the offered focuses") | focus |

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
| `0004_card_spend_levels.sql` | 4 | `accounts.spend_soft`, `spend_mid` and `spend_hard`, with existing cards set to 3,000, 4,000 and 5,000 in the main currency unit (CC-10, §2) |
| `0005_checkins.sql` | 5 | checkins and its index (§4.9), and `goals.moved_on` and `moved_from` (§4.4) |

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

Answered on 2026-11-13, while building loans (LOAN-1, LOAN-2):

64. **A payment row's error names it by its place in the rows sent**, from 1 ("Payment 3: enter a real date"). A paid row isn't sent, so an error about one names it by its date (§6).
65. **A new loan's paid rows can't come before its date borrowed either.** An edit that moves the date borrowed past a paid payment is refused, and the message says to change the date borrowed (§4.6, §6).

Answered on 2026-11-15, while building Mark paid (LOAN-3):

66. **Mark paid takes any active account and expense category**, like a new expense, and also keeps the loan's own if they've been archived since. Only an expense is accepted (`Validation` on `kind`). A payment that's paid or paid before is a `Conflict` ("This payment is already paid"), and Mark unpaid on one without a transaction is a `Conflict` ("This payment isn't marked paid") (§4.6).
67. **Mark paid may be dated after today.** The payment is paid at once, and its expense counts once its date arrives, like any future-dated row (TXN-8, §5.2).

Answered on 2026-11-15, while building the Debt-free figures (LOAN-5 to LOAN-8):

68. **The summary has a `% paid` across every loan**, rounded down like each loan's, because the Debt-free screen's top bar shows the share paid and the frontend never works out money figures (§5.11, UI_FLOWS §8.5).

Answered on 2026-11-15, while building the dashboard's loan banners (LOAN-4, LOAN-11):

69. **"Move money first" below 0:** what a running total lacks is `max(0, total − balance)`, so an account at −₱100.00 with ₱1,000.00 due says to move ₱1,100.00, enough to pay without going further below 0. The first banner from that account carries the deficit, and following every banner still moves exactly enough (§5.11).
70. **Banners on the same date** come in the order their loans were added, and the running totals follow the banners' order (§5.11).
71. **A loan whose account was archived since** still says to move money there. Archived accounts take no new transfers, so the user unarchives it or edits the loan's account (§4.5).
72. **The dashboard's Loans left line says "all overdue"** when everything left is overdue, rather than "debt-free" and today's date, which would read as if nothing were owed. `loansLeft` carries `overdue` for it (§5.11, UI_FLOWS §2).

Answered on 2026-11-15, while building the forecast's domain rules (SAFE-1 to SAFE-5):

73. **Two due dates that pay the same statement bill it once.** With a statement day of 30 and a due day of 31, Jan 31 and Feb 28 both pay the Jan 30 statement: Jan 31 bills it, Feb 28 bills nothing, and what's bought after Jan 30 waits for the next statement (§5.14).
74. **A payday rule with no date in the horizon** (it starts after it) still has a current window, from today to the horizon's end. Only having no payday rule at all means no windows (§5.12, §5.15).
75. **An overdue card statement is due today.** What's still owed on the latest statement once its due date has passed counts as due today, like anything overdue. Someone who doesn't pay in full sees the rest as due today until the next statement closes, because bills assume the statement is paid in full (ADR 0006, §5.14).
76. **Credit on a card carries forward.** Bills start from `−balance`, not `amount_owed`, so an overpaid card's credit is used up by later charges before anything is billed (§5.14).
77. **Known limit: a future-dated transfer out of a card**, such as a cash advance, isn't counted, as §5.15 says for every card transfer. Once its date arrives it's in the balance and the next bill picks it up. Until then, a transfer from a card to an account that isn't counted (a goal's) makes the forecast too high by its amount.

Answered on 2026-11-15, while building `get_forecast` and the Safe to spend card (SAFE-6):

78. **A paused payday rule still counts as having a payday rule**, so pausing the only one keeps Safe to spend, over one window to the horizon's end, rather than asking the user to mark a payday they've already marked. Only its dates are left out (§5.12, §5.15).
79. **Pending dates of a paused rule still count**, a payday's too: they wait in the Due list whatever the rule does next. Only the dates still to come need a rule that isn't paused (§5.13, §5.15).
80. **A date up to today that processing hasn't reached yet** (the app was open past midnight) counts as due today, once, so the figures don't change when processing runs (§5.15).
81. **Received pay is listed by the payslip's date.** A payslip recorded early shows in the window it was recorded in, which is where its money already is (§5.15).

Answered on 2026-11-16, while building Before you borrow (LOAN-9, LOAN-10):

82. **A window already short isn't blamed on the loan.** `short by` is set only on a window that runs short with the draft and not without it ("any window it would make short", UI_FLOWS F15). A window that runs short anyway isn't named, even when the draft makes it shorter; the panel still lists its `left` before and after (§5.16).
83. **The draft's typed rows are checked like a new loan's payments**, without a date borrowed, since the preview doesn't have one: 1–600 rows, real dates, amounts above 0. The other loan fields are checked when the loan is saved, after the confirm (§5.16, §6).

Answered on 2026-11-16, planning the smart goal date and the card spending levels (GOL-5, GOL-6, CC-10, CC-11):

84. **A goal's monthly room is the forecast-safe amount**, the most that can be set aside on every saving date for 12 months without any day going below 0, rather than an average monthly surplus, which could suggest saving money a bill needs later in the month (§5.17).
85. **Saving dates start a month from today**, the way §5.7 counts months left, so saving `monthly_needed` for the earliest date never asks for more than the monthly room (§5.17).
86. **A too-soon date is refused on add and on a date edit only.** An edit that keeps the date passes, so a goal never becomes unsaveable as bills change (§5.17).
87. **Other goals with a date count against the monthly room**, so two goals never plan on the same money. Safe to spend still doesn't hold anything back for goals (PRD §3, §5.17).
88. **Spending levels are per card, for the current statement cycle,** stored on the account, ₱3,000.00, ₱4,000.00 and ₱5,000.00 to start with. They need migration `0004`: `0003` was already run on development databases, so a new file is safer than changing it before `v0.4.0` (§2, §8).
89. **The hard level asks "Spend anyway?" but never blocks**, like the credit limit. Soft and mid only warn (§5.18).
90. **A goal needs a target date and a payday rule** (the owner, 2026-11-16). A goal with no date can't be judged and asks for no monthly amount, so it's refused, on add and on every edit. Without a payday rule there's no forecast to judge a date by, so adding a goal, or changing its date, is refused with a message to mark the pay as a payday first (§5.17).
91. **Goals saved before Phase 4 without a date** stay as they are: `target_date` stays nullable, with no migration, and they show their progress with no monthly amount. Archive, Unarchive and Delete never need a date; Edit does (§4.4, §5.7, §5.17).
92. **Goals follow the forecast** (the owner, 2026-11-16): a save that leaves a goal's date unrealistic moves the date to the new earliest one by itself, with a note, rather than asking or blocking the save. The expense is what the user meant to record; the goal is the plan that adjusts (§5.17).
93. **What moves goals:** saving an expense or a transfer, a recurring rule or a loan, the changes that add spending to the forecast. Processing, payslips and deletes don't move them, and a goal a delete leaves behind is shown as behind instead (§5.17).
94. **No room at all keeps the date**, since no date would be realistic, and the goal shows as behind until money frees up (§5.17).
95. **Oldest goal first** when one save affects several, each worked out with the goals before it at their new dates (§5.17).

Answered on 2026-11-16, building the smart goal date (GOL-5, GOL-6):

96. **Another goal takes its monthly amount on its first months-left saving dates** (the owner, 2026-11-16), not only on the saving dates before its target date. `monthly_needed` divides what's left by the months left, so counting one date fewer would leave its last month's money free for another goal to plan on (§5.7, §5.17).
97. **Any short day in the 12 months makes the monthly room 0**, even one before the first saving date, so the definition ("without any day's running total going below 0") and the room-0 bullet agree. A short window has to be fixed before a goal can count on money (§5.15, §5.17).
98. **The too-soon message's amount is the monthly room**, the same figure as the line under the date, not the smaller `monthly_needed` for the earliest date (§5.17). Its wording changed in Phase 5 (item 148).

Answered on 2026-11-16, building goals that follow the forecast (GOL-7):

99. **A goal is judged by month** (the owner, 2026-11-16): it's late when its months left is 0 or its `monthly_needed` is more than the monthly room, not when the earliest date is after its target date. `today + ⌈left ÷ room⌉ months` moves a day later every day, so the day-by-day test made a goal at its suggested date late the next day, and nearly every save moved it by a day (§5.17).
100. **The newest goal gives way first** (the owner, 2026-11-16), replacing item 95's oldest first: checking the oldest first, with the newer goals still taking their share, made the oldest goal the one that moved, the opposite of letting a goal added earlier keep its claim (§5.17).
101. **A goal whose earliest date is past the calendar's range** (room above 0, a huge target) is kept at its date and shown as behind, like a room of 0, as the too-soon check already does (§5.17).
102. **A goal's own scheduled deposits don't count against it** (the owner, 2026-11-16): a future transfer from the counted accounts into a dated, unachieved goal's account is left out of the goal forecast, for the suggestion (GOL-5) and the GOL-7 check. Counting it as money out while the goal also asks for its monthly amount counted the saving twice, so scheduling a goal's own deposit moved it later. A deposit bigger than its goal's monthly amount slightly overstates the room the other goals see, which errs toward keeping dates (§5.17).
103. **Post and Edit & post on a due date are processing**, so they don't move goals, even with an edited amount: the date was already in the forecast, and a goal it leaves late shows as behind and moves on the next save that does (§5.17, item 93).


Answered on 2026-11-16, building card spending levels (CC-10, CC-11):

104. **A save that doesn't raise this cycle's spending asks nothing**, even past the hard level: an edit of only the note, or one that lowers the amount, takes nothing further, so "Spend anyway?" would only be noise. A new expense past hard still asks (§5.18).
105. **Edit & post doesn't ask**, like Post: both are processing a date the rule already scheduled, as §5.17 treats them (§5.18).
106. **A card sent without a level keeps the one it has**, and a new card gets the default, so a caller that leaves them out never resets the user's levels. The form always sends all three, and an empty one as 0, for the backend to name (§6).

Answered on 2026-11-16, testing Phase 4 on a copy of the owner's data (GOL-7):

107. **An income rule never moves a goal**, like an income transaction. Setting up paydays one at a time showed why: with only some paydays saved, a goal can't be reached (room 0) and stays put, then the next payday rule gives a little room and the goal moved months later, never to come back once the last payday was added. Adding pay can't make a date less realistic, so only expense and transfer rules are checked. An income rule edited lower leaves a goal Behind instead (§5.17).

Answered on 2026-11-17, planning the check-in and the Vault level (CHK-1 to CHK-9, LVL-1 to LVL-8, Phase 5):

108. **A check-in row is a snapshot, never recalculated** (the owner, 2026-11-17). The live stats and level follow the data, as balances do (ADR 0005), but a past check-in keeps what it showed, and a graded focus keeps its grade, so editing an old transaction can't rewrite the record (§4.9).
109. **Only the newest completed week and month are ever due.** Periods missed while the app was closed are skipped, not stacked, so the user never faces a queue of reviews (§5.19).
110. **A yearly review is one step, not a third check-in.** It's settled by a check-in finished in January, and skipped for a year when the app isn't opened that month (§5.19).
111. **The first check-in on an existing database** comes at the first start after the update: its last completed week or month already has transactions. A new database gets none until it has one (§5.19).
112. **The level is live and can drop** (the owner, 2026-11-17), and each check-in row stores the level it saw, so a level-up, a drop and Your year can be shown (§5.21).
113. **The Debt stat doesn't need the forecast.** It compares the month's scheduled loan payments (LOAN-7's figure, which doesn't move as payments are marked paid) with average past take-home pay, so it's steady through the month and works with no payday rule (§5.20).
114. **The Savings stat counts goal accounts**, although Safe to spend leaves them out: money set aside is still a cushion (§5.20).
115. **A day logged goes by the transaction's date**, not `created_at`, since timestamps are never used in calculations (§1, §5.20).
116. **Level 2 asks for a payday rule and a budget**, not card spending levels: every card has levels since migration `0004`, so that condition would always be met (§5.21).
117. **A cap counts everyday spending only**: an expense that pays a loan payment, that a recurring date posted, or that's a payslip deduction is a bill, which Safe to spend has already set aside. Counting it would make a spending cap taken from Safe to spend unfair (§5.22).
118. **A focus runs for the whole current week or month**, counting what was already spent in it before the check-in, so its dates always match a period the app already knows (§5.22).
119. **A merged check-in sets a focus for the month only**, on its month row, so the user makes one choice per check-in (§5.22).
120. **Finishing grades every ended focus not yet graded**, so one skipped at a check-in that showed a newer one doesn't come back later as if it were new (§5.22).
121. **A loan-on-time focus whose payment was removed is `void`**, counted as neither kept nor missed, and passed over by the focus streak (§5.22, §5.23).
122. **A focus puts its category or card in use**, like any other reference, and merging categories moves it. In practice both already have transactions (§3.2, §4.9).
123. **A focus target that's an amount locks `currency_decimals`**, like every other stored amount (§1).
124. **Goals record when GOL-7 last moved them** (`moved_on`, `moved_from`), so a weekly check-in can show its Goals step only when something changed (the owner, 2026-11-17). Nothing else reads them (§4.4).
125. **The balance check stores nothing.** It's a prompt to compare with the real accounts, and the fix is an ordinary transaction (§5.19).
126. **With no win, none is named.** Finishing the check-in is never the win, so the Start step can be only the focus result, or only one line (§5.23).
127. **The stat formulas and level conditions are fixed rules, not settings.** A change to them updates §5.20–5.21 first.

Answered on 2026-11-17, tuning the stats and levels with the owner:

128. **The stats use take-home pay and spending without deductions.** A payslip's gross is income and its deductions are expenses, so measuring against gross made a month of spending include SSS and tax, and compared loans with pay the owner never receives (§5.20).
129. **Borrowed money is left out of what was earned** (the owner): a new loan's amount borrowed is taken off income in the month it was borrowed, for Cashflow and for levels 4 and 5. A "Loan proceeds" category was the alternative, and breaks when it's renamed or the money is logged elsewhere (§5.20, §5.21).
130. **Cashflow is 100 at 10% of take-home pay unspent** (the owner), not the usual 20%, because loan payments already count as spending (§5.20).
131. **Debt reaches 0 at 90% of take-home pay** (the owner), a wide scale, so the stat keeps moving as each loan finishes. The first scale, 0 at 50%, can sit at 0 for months while loans are heavy and show nothing (§5.20).
132. **Habit is full at 25 logged days of 30** (the owner), for an app that's used nearly every day. Level 1 stays at 20, so a short break doesn't drop every level (§5.20, §5.21).
133. **The savings levels are two weeks (level 7) and one month (level 9)** (the owner), so there's a savings level within reach before debt-free. The Savings stat still scores three months as 100 (§5.21).
134. **A late payment has no grace** (the owner): on the due date or before is on time (§5.20).
135. **The level names stay** (the owner): Tracker, Planner, Bill Keeper, In the black, Steady, Card under control, Two weeks ahead, Debt-free, One month ahead, Vault Master (§5.21).

Answered on 2026-11-17, reviewing the Phase 5 docs before building (PR #111):

136. **Each stat's reason and each level condition has a fixed code**, picked by the order in §5.20's reason table and §5.21's conditions, so a reason, a level drop and the next best move are the same everywhere and testable (§5.20, §5.21).
137. **The check-in being finished counts as finished today** in its own status and stored snapshot, for Habit as well as the streaks, so a stored Habit can reach 100 (§5.20).
138. **The Start step shows only a focus this check-in grades**, so one graded earlier never shows, or counts as the win, twice (§5.22).
139. **A kind whose stat is none ranks as 100** among the focus offers, and a focus must match its offer's period too (§5.22, §6).
140. **Divisions below 0 round toward minus infinity**, so a month a little over break-even scores 49 for Cashflow, never 50 (§5.20).

Answered on 2026-11-19, building migration `0005` (PR #119):

141. **The `checkins` table and the goals' `moved_on` and `moved_from` check the rules §4.4 and §4.9 state in words**: the date format, a year row with no focus, a focus with its dates, its category or its card, and both moved columns set or neither. A service bug is then refused instead of stored for good, since a check-in row is never edited (the owner's choice, §4.4, §4.9).

Answered on 2026-11-19, building the level ladder (PR #121):

142. **The next best move's own codes are `pay_overdue_loan` and `pay_overdue_card`.** A gap is never below 0, and there's none while there's no figure for one yet: the savings levels before any spending, the loan-free levels with no streak. The card move is for active cards only, as `overdue` counts them, so the move and level 3 always agree. With several cards overdue, it names the earliest due, then the lowest account id (§5.21).

Answered on 2026-11-19, building the wins and streaks (PR #123):

143. **Equal category falls go to the lower category id**, as equal rises do for the focus offers, so the win is always the same one for the same figures (§5.22, §5.23).

Answered on 2026-11-19, building the Vault level backend (PR #124):

144. **A card's statement that closed before the card was added is never overdue** (the owner's choice, 2026-11-19). The app only knows what's owed on the day the card is added, not what was on the statement before it, so a card added after its due date would otherwise be overdue on day one: Bills at 75, the level stuck at 2 and "pay this card" as the next move. It applies to the Bills stat, level 3 and the next best move (§5.20, §5.21). The forecast still bills that statement as due today (§5.14), which errs toward holding money back. The day added is the local date of the card's `created_at`, the one time a timestamp feeds a figure (§1).

Answered on 2026-11-20, building the check-in backend (PR #125):

145. **A debt-free date stored while everything was overdue is never "moved"** (§5.19). §5.11 makes that date the day the check-in was finished, so comparing it with a later real date would report a move that never happened. A stored date on or before its row's `completed_on` is left out of the comparison.

Answered on 2026-11-20, testing on a copy of the owner's data (T18):

146. **Every focus offer can still be kept on the day it's offered** (the owner's choice, 2026-11-20). A weekly check-in finished late in the week offered a spending cap of `per day × 7` and a category cap that the week's spending had already passed, so the focus started red. The spending cap is now what's recorded for the week plus `per day` for each day left; a card level is aimed above the highest cycle spending in the period, and a category cap already over its target, a card already at hard and a loan payment already past its due date aren't offered; and logging asks for at most the days still reachable, so there's always an offer to pick (§5.22).
147. **A first month that's only partly tracked counts in full** (the owner's choice, 2026-11-20). A user who starts mid-month gets Cashflow, Debt and Savings from that month, so Debt can read low while the month's loan payments are measured against part of a month's pay. It corrects itself after the first full month, and leaving such months out would show "—" for a month longer (§5.20).

Answered on 2026-11-20 (Phase 5, the goal's two monthly figures):

148. **The monthly room is named as the most there is, and `monthly_needed` as what the date needs** (GOL-3, GOL-6). The dialog's "set aside about ₱X a month" and the too-soon message's "saving about ₱X a month" read as what the goal needs, so for one goal and one date the card's smaller "Save ₱Y/month" looked like it disagreed. The too-soon message now says "Even setting aside the most you could, about ₱X a month, the earliest you can reach this is …", the dialog's line says "The most you could set aside …", and the card adds "on time". Only the wording changed: both figures are worked out as before (§5.7, §5.17, items 84, 85, 98; UI_FLOWS §11 item 96).
