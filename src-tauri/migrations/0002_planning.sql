-- Phase 3 schema, exactly as in DATA_MODEL §4. Released migrations are never edited.

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
