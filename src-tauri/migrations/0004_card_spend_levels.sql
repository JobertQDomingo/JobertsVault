-- Phase 4: credit card spending levels per statement cycle (CC-10, DATA_MODEL §2, §5.18).
-- ADD COLUMN can't add a table CHECK, but a column's CHECK may name the columns before it, so
-- the whole rule sits on the last one: all three NULL, or a card with 0 < soft < mid < hard.
-- The CHECK is tested on existing rows when the column is added, where all three are NULL.
ALTER TABLE accounts ADD COLUMN spend_soft INTEGER;
ALTER TABLE accounts ADD COLUMN spend_mid INTEGER;
ALTER TABLE accounts ADD COLUMN spend_hard INTEGER CHECK (
  (spend_soft IS NULL AND spend_mid IS NULL AND spend_hard IS NULL)
  OR (kind = 'credit_card' AND spend_soft IS NOT NULL AND spend_mid IS NOT NULL
      AND spend_hard IS NOT NULL AND 0 < spend_soft AND spend_soft < spend_mid
      AND spend_mid < spend_hard)
);

-- 3,000, 4,000 and 5,000 in the main currency unit; a missing setting is the default 2 decimals.
UPDATE accounts
SET spend_soft = 3000 * f.unit, spend_mid = 4000 * f.unit, spend_hard = 5000 * f.unit
FROM (SELECT CASE (SELECT value FROM settings WHERE key = 'currency_decimals')
               WHEN '0' THEN 1 ELSE 100 END AS unit) AS f
WHERE kind = 'credit_card';
