"""T25: a full year of Phase 3-5 data in the PERF build's database only (com.jobertsvault.perf),
as services/perf.rs seeds it: 24 payslips, a loan with 12 payments paid and 12 to come, payday and
recurring rules starting after today, a budget, and the check-ins finished each week and month
with graded focuses. Nothing is left due, so the app opens on the dashboard.

Run after the perf build has started once (so its migrations ran):
    python -I .claude/skills/task/tools/passes/seedperf5.py
"""

import datetime as dt
import os
import sqlite3

DB = os.path.join(os.environ["APPDATA"], "com.jobertsvault.perf", "jobertsvault.db")
assert DB.endswith(os.path.join("com.jobertsvault.perf", "jobertsvault.db")) and os.path.exists(DB), DB
c = sqlite3.connect(DB)
c.execute("PRAGMA foreign_keys=ON")
today = dt.date.today()
assert c.execute("SELECT COUNT(*) FROM checkins").fetchone()[0] == 0, "already seeded"
cat = lambda name: c.execute("SELECT id FROM categories WHERE name = ?", (name,)).fetchone()[0]
acct = lambda name: c.execute("SELECT id FROM accounts WHERE name = ?", (name,)).fetchone()[0]
bank, cash, card = acct("P Bank"), acct("P Cash"), acct("P Visa")


def add_months(d, n, day=None):
    y, m = divmod(d.month - 1 + n, 12)
    y, m = d.year + y, m + 1
    last = (dt.date(y + (m == 12), m % 12 + 1, 1) - dt.timedelta(days=1)).day
    return dt.date(y, m, min(day or d.day, last))


def txn(kind, amount, date, account, category, payslip=None):
    cur = c.execute(
        "INSERT INTO transactions (kind, amount, date, account_id, category_id, payee, payslip_id, created_at, updated_at)"
        " VALUES (?, ?, ?, ?, ?, 'Perf employer', ?, 'x', 'x')",
        (kind, amount, date.isoformat(), account, category, payslip),
    )
    return cur.lastrowid


with c:
    first = dt.date(today.year - 1, today.month, 1)
    salary, loan_cat = cat("Salary"), cat("Loan")
    deductions = [(cat("SSS"), 60000), (cat("PhilHealth"), 30000), (cat("Withholding tax"), 100000)]
    c.execute(
        "INSERT INTO loans (name, account_id, category_id, is_new, added_on, created_at, updated_at)"
        " VALUES ('Perf loan', ?, ?, 0, ?, 'x', 'x')", (bank, loan_cat, first.isoformat()))
    loan = c.execute("SELECT id FROM loans WHERE name = 'Perf loan'").fetchone()[0]
    for n in range(24):
        month = add_months(first, n)
        for day in (15, 30):
            d = add_months(month, 0, day)
            if n < 12 and d <= today:
                gross = txn("income", 1250000, d, bank, salary)
                c.execute("UPDATE transactions SET payslip_id = id WHERE id = ?", (gross,))
                for category, amount in deductions:
                    txn("expense", amount, d, bank, category, gross)
        due = add_months(month, 0, 20)
        paid = txn("expense", 100000, due, bank, loan_cat) if due < today else None
        c.execute(
            "INSERT INTO loan_payments (loan_id, due_date, amount, transaction_id, created_at, updated_at) VALUES (?, ?, 100000, ?, 'x', 'x')",
            (loan, due.isoformat(), paid))
    # Rules start after today, so nothing posts or waits at startup.
    nxt = lambda day: min(d for d in (add_months(today, 0, day), add_months(today, 1, day)) if d > today).isoformat()
    for kind, amount, account, category, freq, start, mode, payday in [
        ("income", 1050000, bank, salary, "monthly", nxt(15), "confirm", 1),
        ("income", 1050000, bank, salary, "monthly", nxt(30), "confirm", 1),
        ("expense", 15000, cash, cat("Food & Groceries"), "daily", (today + dt.timedelta(days=1)).isoformat(), "auto", 0),
        ("expense", 80000, card, cat("Shopping"), "weekly", (today + dt.timedelta(days=2)).isoformat(), "auto", 0),
        ("expense", 260000, bank, cat("Utilities"), "monthly", nxt(10), "confirm", 0),
    ]:
        c.execute(
            "INSERT INTO recurring_rules (kind, amount, account_id, category_id, frequency, interval, start_date, mode, is_payday, created_at, updated_at)"
            " VALUES (?, ?, ?, ?, ?, 1, ?, ?, ?, 'x', 'x')", (kind, amount, account, category, freq, start, mode, payday))
    c.execute("INSERT INTO budgets (category_id, period, amount, created_at, updated_at) VALUES (NULL, 'monthly', 6000000, 'x', 'x')")
    c.execute("INSERT INTO budgets (category_id, period, amount, created_at, updated_at) VALUES (?, 'weekly', 500000, 'x', 'x')", (cat("Dining Out"),))

    # Check-ins: every week up to the newest completed one, every month up to the last one.
    monday = today - dt.timedelta(days=today.weekday())
    last_week = monday - dt.timedelta(days=7)
    rows = []
    w = last_week - dt.timedelta(weeks=51)
    while w <= last_week:
        rows.append(("week", w, w + dt.timedelta(days=6), w + dt.timedelta(days=13)))
        w += dt.timedelta(weeks=1)
    m = add_months(first, 0, 1)
    while add_months(m, 1, 1) <= today:
        end = add_months(m, 1, 1) - dt.timedelta(days=1)
        rows.append(("month", m, end, add_months(end, 1, 31)))
        m = add_months(m, 1, 1)
    for i, (kind, start, end, next_end) in enumerate(rows):
        newest = end in (last_week + dt.timedelta(days=6), m - dt.timedelta(days=1))
        c.execute(
            "INSERT INTO checkins (kind, period_start, period_end, completed_on, level, stat_cashflow, stat_bills, stat_card, stat_debt,"
            " stat_savings, stat_habit, debt_free_on, focus_kind, focus_target, focus_start, focus_end, focus_result, focus_actual, created_at, updated_at)"
            " VALUES (?, ?, ?, ?, ?, ?, 90, ?, 60, ?, 80, NULL, 'spending_cap', ?, ?, ?, ?, ?, 'x', 'x')",
            (kind, start.isoformat(), end.isoformat(), (end + dt.timedelta(days=1)).isoformat(), 2 + i % 4, 40 + i % 50, 50 + i % 40, 20 + i % 30,
             500000 if kind == "week" else 2000000, (end + dt.timedelta(days=1)).isoformat(), next_end.isoformat(),
             None if newest else ("missed" if i % 3 == 0 else "kept"), None if newest else 400000 + (i % 3) * 60000))
print(dict(c.execute("SELECT kind, COUNT(*) FROM checkins GROUP BY kind").fetchall()),
      c.execute("SELECT COUNT(*) FROM transactions").fetchone(), c.execute("SELECT COUNT(*) FROM loan_payments").fetchone())
