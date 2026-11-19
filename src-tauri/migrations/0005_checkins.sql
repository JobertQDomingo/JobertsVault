-- Phase 5: the check-ins (CHK-1 to CHK-8, DATA_MODEL §4.9) and the last GOL-7 move of a goal's
-- target date (CHK-3, DATA_MODEL §4.4). A check-in row is never edited, except to grade its
-- focus, and never deleted.
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

ALTER TABLE goals ADD COLUMN moved_on   TEXT CHECK (moved_on GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]');  -- the local date of the move
-- the target date it had before; a column's CHECK may name an earlier column, so both are set or neither is
ALTER TABLE goals ADD COLUMN moved_from TEXT CHECK ((moved_from IS NULL) = (moved_on IS NULL)
  AND (moved_from IS NULL OR moved_from GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'));
