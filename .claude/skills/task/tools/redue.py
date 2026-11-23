"""Make a check-in due again in the DEV database, by deleting its newest checkins rows.

Close the dev app first (stopdev.ps1). Never touches the real jobertsvault.db.

    python -I .claude/skills/task/tools/redue.py week    # a weekly check-in (the month stays done)
    python -I .claude/skills/task/tools/redue.py month   # a monthly one (the week stays done)
    python -I .claude/skills/task/tools/redue.py both    # a merged one (titled the month's)

A check-in is due when the latest completed week or month has no row (DATA_MODEL 5.19), so
deleting the newest row of a kind brings that kind back, as long as that row was the latest
period's. Its focus and grade go with it: dev data only.
"""

import os
import sqlite3
import subprocess
import sys

DB = os.path.join(os.environ["APPDATA"], "com.jobertsvault.app", "jobertsvault-dev.db")
assert DB.endswith("jobertsvault-dev.db"), DB

kinds = {"week": ["week"], "month": ["month"], "both": ["week", "month"]}.get(
    sys.argv[1] if len(sys.argv) > 1 else ""
)
if not kinds:
    sys.exit(__doc__)

running = subprocess.run(
    ["powershell", "-NoProfile", "-Command",
     "Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -like '*\\src-tauri\\target\\debug\\jobertsvault.exe' }"],
    capture_output=True, text=True,
).stdout.strip()
if running:
    sys.exit("The dev app is running. Close it first (.claude/skills/task/tools/stopdev.ps1).")

conn = sqlite3.connect(DB)
with conn:
    for kind in kinds:
        row = conn.execute(
            "SELECT id, period_start, period_end FROM checkins WHERE kind = ? "
            "ORDER BY period_start DESC LIMIT 1",
            (kind,),
        ).fetchone()
        if row is None:
            print(f"no {kind} row: it's already due if a transaction is old enough")
            continue
        conn.execute("DELETE FROM checkins WHERE id = ?", (row[0],))
        print(f"deleted the {kind} row for {row[1]} to {row[2]}")
conn.close()
