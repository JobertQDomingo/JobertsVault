-- Phase 5: the HMO payslip deduction category (PAY-3, DATA_MODEL §7), as 0003 added its own.
-- Only a database with no expense category of that name gets it (`name` is COLLATE NOCASE, so
-- this ignores case): one the user made is kept as it is. It goes after the existing expense
-- categories and takes the least-used color of the active ones, the lowest number on a tie.
-- The palette is written out here so a later change to it can't change what this does.
INSERT INTO categories (name, kind, color, icon, sort_order, created_at, updated_at)
SELECT 'HMO', 'expense',
       (SELECT p.column2
          FROM (VALUES (1, 'cat-1'), (2, 'cat-2'), (3, 'cat-3'), (4, 'cat-4'), (5, 'cat-5'),
                       (6, 'cat-6'), (7, 'cat-7'), (8, 'cat-8'), (9, 'cat-9'), (10, 'cat-10'),
                       (11, 'cat-11'), (12, 'cat-12')) AS p
         ORDER BY (SELECT count(*) FROM categories c
                    WHERE c.kind = 'expense' AND c.archived = 0 AND c.color = p.column2),
                  p.column1
         LIMIT 1),
       'pill',
       (SELECT coalesce(max(sort_order) + 1, 0) FROM categories WHERE kind = 'expense'),
       strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
       strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
WHERE NOT EXISTS (SELECT 1 FROM categories WHERE kind = 'expense' AND name = 'HMO');
