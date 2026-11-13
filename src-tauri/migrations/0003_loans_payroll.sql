-- Phase 4: loans, payslips and paydays (DATA_MODEL §4.5–4.8).
-- The Loan and payslip deduction categories are added by this migration's hook (db/seed.rs, §7).

-- A loan is a list of scheduled payments, not an account (ADR 0011).
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

-- A payment is paid when it has a transaction, paid before when its flag is set, else unpaid.
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

-- Every row of a payslip points at its gross row, the gross row included.
ALTER TABLE transactions ADD COLUMN payslip_id INTEGER REFERENCES transactions(id) ON DELETE CASCADE;
CREATE INDEX idx_txn_payslip ON transactions(payslip_id) WHERE payslip_id IS NOT NULL;

ALTER TABLE recurring_rules ADD COLUMN is_payday INTEGER NOT NULL DEFAULT 0 CHECK (is_payday IN (0,1));
