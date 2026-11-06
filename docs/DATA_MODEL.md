# Data Model: Jobert's Vault

| | |
|---|---|
| **Status** | Approved v1.1 |
| **Last updated** | 2026-11-06 |
| **Database** | SQLite (bundled with the app through `rusqlite`) |
| **Related docs** | [PRD.md](PRD.md), [ARCHITECTURE.md](ARCHITECTURE.md) |

This document describes every table and field, plus the rules for calculating balances, totals and periods. **Any change to the database schema must update this doc first, then add a new migration.**

---

## 1. Global conventions

| Topic | Rule |
|---|---|
| **Money** | Stored as `INTEGER` in **minor units** (e.g. cents), as an `i64` in Rust. Floating-point numbers are never used for money, anywhere. |
| **Amount sign** | `transactions.amount` is always **positive**. The transaction's `kind` decides whether money goes in or out. |
| **Decimal places** | `currency_decimals` (0 or 2) decides how minor units are displayed. It **can only be changed while no amount is stored**: no transactions, and no account with an opening balance other than 0 or a credit limit. Changing it afterwards would change the meaning of every stored amount. |
| **Dates** | `TEXT` in `YYYY-MM-DD` format (ISO 8601), using the user's local calendar. This format sorts correctly as plain text. No time of day is stored. |
| **Timestamps** | `created_at` / `updated_at` are `TEXT` in ISO 8601 UTC (`2026-10-26T09:30:00Z`). They're for record-keeping only and are never used in calculations. Services receive the current time as a `now` value, the same way they receive `today`. |
| **IDs** | `INTEGER PRIMARY KEY`, assigned by SQLite. |
| **Booleans** | `INTEGER` 0 or 1, with a `CHECK` constraint. |
| **Enums** | `TEXT` with a `CHECK (... IN (...))` constraint. The values are lowercase `snake_case`. |
| **Table mode** | Every table is declared `STRICT`, so SQLite rejects a value of the wrong type. |
| **Connection settings** | `PRAGMA foreign_keys = ON`, `PRAGMA journal_mode = WAL`, `PRAGMA busy_timeout = 5000`, `PRAGMA synchronous = NORMAL` (as in ARCHITECTURE §4.3) |
| **Deleting records** | Accounts, categories and payment methods that are in use are **archived instead of deleted**. Unused ones can be deleted. Foreign keys use `ON DELETE RESTRICT`, so the database itself blocks a delete that would break history. |
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

**Merging (CAT-4):** in one database transaction, move every transaction from category A to active category B (both must be the same kind), then delete A.

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

**Payees** don't have their own table. Suggestions (TXN-6, Phase 2) come from the distinct `payee` values on rows of the form's type (expense or income), sorted by most recent use (`date DESC, id DESC`). Each payee is suggested once, ignoring ASCII case and spelled as it was last used, and only when it starts with what's typed (ASCII case ignored, wildcards taken literally). At most 8 are suggested. The suggested category is the one on the latest row of that type with that payee. If that category is archived now, the payee is still suggested but fills in no category.

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
| `backup_folder` | absolute path | `<app data>\backups` | 2 | SET-3 |
| `auto_backup_enabled` | `true` \| `false` | `true` | 2 | SET-3 |
| `last_backup_at` | ISO timestamp | (none) | 2 | BAK-7 |
| `last_auto_backup_date` | `YYYY-MM-DD`, the local date of the last successful automatic backup | (none) | 2 | BAK-2 |
| `theme` | `system` \| `light` \| `dark` | `system` | 4 | SET-5 |
| `reduce_transparency` | `true` \| `false` | `false` | 4 | SET-8 |

`month_start_day` is capped at 28 so that every month contains that day.

`backup_folder`, `auto_backup_enabled`, `last_backup_at` and `last_auto_backup_date` describe this computer's backups rather than the data, so a restore keeps the current values (§9).

`onboarding_completed` becomes `true` when the user chooses **Start using the app** or **Set up later**. Closing the app before that shows the form again on the next launch.

---

## 4. Tables: Phase 3 (migration `0003_planning.sql`)

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

**Editing a rule (REC-5):**
- Template fields (amount, category, and so on) affect future occurrences only.
- If the schedule changes, `start_date` becomes the next unprocessed date and `next_index` is reset to 0.

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

- **Auto mode:** a transaction is created and the row is inserted with status `posted`, both in one database transaction.
- **Confirm mode:** the row is inserted as `pending`. When the user approves it, it becomes `posted` and gets a transaction. When the user skips it, it becomes `skipped`.
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

- The linked account can't be a credit card. This is checked in Rust.
- `achieved_at` is set the first time the account's balance reaches the target amount (GOL-4).

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
- **Future-dated transactions are never included** (TXN-8, Phase 3). Every total, budget and "recent" list counts only `date ≤ today`, matching balances, so the numbers on a screen always agree. Future-dated rows appear only under Upcoming.
- If the report is filtered by account (RPT-5), only transactions with `account_id = account` are counted.
- **Refunds** (CC-5) use an income category, so a refund increases *income*, not lowers *expense*. This is intentional: refunds offset net cashflow, not the expense total or the category breakdown (decided 2026-10-26).

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
- **Reminder (CC-7, Phase 3):** shown when the next due date is 5 days away or closer and `amount_owed > 0`.

### 5.6 Budget progress (Phase 3)

```
spent = Σ expense amounts in the budget's current period (in its category, or in all categories for an overall budget)
used% = spent ÷ amount        →  amber when ≥ 80%, red when ≥ 100%
```

Refunds don't reduce `spent`, because they're income (§5.2).

### 5.7 Goal progress (Phase 3)

```
progress = max(0, current balance of the linked account)
% done   = min(100, progress ÷ target_amount × 100)
monthly_needed = (target_amount − progress) ÷ months left until target_date   (rounded up, if there is a target date)
```

### 5.8 Default accounts

| Used for | Rule |
|---|---|
| **Quick-add (TXN-1)** | `last_used_account_id`, which is updated whenever a transaction is created. If it's missing or the account is archived or deleted, use the first active account by `sort_order`. |
| **Pay bill's From account (CC-9)** | The active non-card account on the latest transaction (`date DESC, id DESC`), going by each transaction's source `account_id`. If there is none, the first active non-card account by `sort_order`. If there's no such account, the field starts empty. |
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

---

## 6. Validation rules (checked in Rust)

The database's CHECK constraints are a last line of defence. The Rust service layer checks everything **first** and returns clear, per-field errors that the forms can display.

| Rule | Error shown on |
|---|---|
| Amount is > 0 and ≤ 999,999,999,999 minor units | amount |
| The date is a real calendar date | date |
| The date isn't in the future: "Date can't be in the future" (Phases 1–2 only; Phase 3 removes this rule for TXN-8) | date |
| An opening balance is 0 or more and ≤ 999,999,999,999 minor units, and a credit limit is > 0 and ≤ the same maximum | that field |
| A transfer has no payee or payment method | payee / payment method |
| The last active account, or the last active category of a kind, can't be archived or deleted (`Conflict`: "Keep at least one active account") | shown as a toast |
| A category's color is a palette key and its icon is on the curated list (DESIGN_SYSTEM §3.4, §6) | color / icon |
| A transaction's category kind matches the transaction kind | category |
| A transfer's destination differs from its source | to_account |
| A **new** transaction can't use an archived account, category or payment method. Existing transactions keep theirs. | account / category |
| Name lengths (account ≤ 50, category ≤ 40, payment method ≤ 40, payee ≤ 100, note ≤ 500) | that field |
| Names are unique within their scope | name |
| Credit card fields are present if and only if the account is a credit card | the card fields |
| `currency_decimals` can't change once any amount is stored: a transaction, or an account's non-zero opening balance or credit limit (§1) | settings |
| A budget's category is an expense category, and a goal's account isn't a credit card | category / account |

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
| `0002_*.sql` | 2 | (only if Phase 2 needs schema changes; backups mostly need settings keys) |
| `0003_planning.sql` | 3 | recurring_rules, recurring_occurrences, budgets, goals |

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

Answered on 2026-11-04:

3. **Payment method names** are 1–40 characters, the same as categories (§6).
4. **A new category's default color** counts only active categories of its kind, so archived ones don't hold a color, and a tie goes to the lowest palette number (§7).

Answered on 2026-11-01:

3. **Future dates** are rejected until Phase 3 (§6).
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

12. **The decimals lock** covers every stored amount, not only transactions: an account's opening balance other than 0, or a credit limit, locks `currency_decimals` too, because changing it would reinterpret those amounts (§1). The seeded Cash account holds 0, so a new database starts unlocked. ADR 0002's rule still holds, since a transaction still locks it.
13. **Pay bill's From account** goes by each transaction's source `account_id`: the latest transaction whose source is an active non-card account. A transfer's destination doesn't count as using that account (§5.8).
14. **Today on a Custom period** jumps to the range in the same sequence that contains today, which is where ▶ stops (§5.3).

Answered on 2026-11-06:

15. **Chart bars (RPT-1)** are calendar-aligned: a Day period is one bar, a Year is 12 calendar months even when `month_start_day` > 1, and Custom weeks follow `week_start` and Custom months are calendar months, cut to the range. "Up to 6 months" means ending before the same date 6 months after the start (§5.9).
16. **Payee suggestions (TXN-6)** match the form's type: an expense suggests only payees used on expenses, with the category of the latest expense to that payee, and the same for income. A payee whose last category is archived is still suggested, but choosing it leaves the category as it is (§3.4).

Answered on 2026-11-07:

17. **A restore keeps this computer's backup settings** (BAK-4): `backup_folder`, `auto_backup_enabled`, `last_backup_at` and `last_auto_backup_date` keep their current values, and are deleted where the current data has none, while every other setting comes from the backup. Restoring on a second Windows account then doesn't point its backups at the first account's folder, and the last backup date doesn't go back in time (§3.5, ARCHITECTURE §7).
