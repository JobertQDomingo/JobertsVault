# Seeds 50,000 transactions into the PERF build's database only (com.jobertsvault.perf).
# Usage: python -I .claude/skills/task/tools/passes/seedperf.py [auto_update true|false]
import datetime, os, sqlite3, sys

db = os.path.join(os.environ["APPDATA"], "com.jobertsvault.perf", "jobertsvault.db")
assert os.path.exists(db), db
c = sqlite3.connect(db)
c.execute("PRAGMA foreign_keys=ON")
auto = sys.argv[1] if len(sys.argv) > 1 else "false"
with c:
    c.execute("INSERT OR REPLACE INTO settings VALUES ('onboarding_completed','true')")
    c.execute("INSERT OR REPLACE INTO settings VALUES ('auto_update_enabled',?)", (auto,))
    if c.execute("SELECT COUNT(*) FROM transactions").fetchone()[0] == 0:
        c.executescript("""INSERT INTO accounts (name, kind, opening_balance, credit_limit, statement_day, due_day, created_at, updated_at) VALUES
          ('P Cash','cash',500000,NULL,NULL,NULL,'x','x'),('P Bank','bank',10000000,NULL,NULL,NULL,'x','x'),
          ('P Wallet','ewallet',0,NULL,NULL,NULL,'x','x'),('P Visa','credit_card',0,5000000,15,5,'x','x');""")
        ids = lambda q: [r[0] for r in c.execute(q)]
        acc = ids("SELECT id FROM accounts")
        inc = ids("SELECT id FROM categories WHERE kind='income'")
        exp = ids("SELECT id FROM categories WHERE kind='expense'")
        pm = ids("SELECT id FROM payment_methods")
        start, today = datetime.date(2022, 1, 1), datetime.date.today()
        days, n = (today - start).days + 1, 50000
        rows = []
        for i in range(n):
            d = (start + datetime.timedelta(days=i * days // n)).isoformat()
            pick = lambda l, s: l[(i * 7 + s) % len(l)]
            a, amt = pick(acc, 0), 100 + (i * 7919) % 500000
            note = f"note {i % 97}" if i % 5 == 0 else None
            if i % 10 == 0:
                to = pick(acc, 1)
                to = to if to != a else acc[(acc.index(a) + 1) % len(acc)]
                rows.append(("transfer", amt, d, a, to, None, None, None, note))
            elif i % 10 in (1, 2):
                rows.append(("income", amt, d, a, None, pick(inc, 3), f"Employer {i % 20}", None, note))
            else:
                rows.append(("expense", amt, d, a, None, pick(exp, 5), f"Store {i % 300}", pick(pm, 2), note))
        c.executemany(
            "INSERT INTO transactions (kind,amount,date,account_id,to_account_id,category_id,payee,payment_method_id,note,created_at,updated_at)"
            " VALUES (?,?,?,?,?,?,?,?,?,'x','x')",
            rows,
        )
print(c.execute("SELECT COUNT(*), MIN(date), MAX(date) FROM transactions").fetchone(), dict(c.execute("SELECT key,value FROM settings")))
