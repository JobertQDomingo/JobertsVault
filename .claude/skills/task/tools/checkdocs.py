"""Cross-checks the docs against each other. Run from the repo root:

    python -I .claude/skills/task/tools/checkdocs.py

It reads only. It prints one line per problem and exits 1 if there is any.
It catches mechanical drift (ids, links, counts, numbering). It can't tell whether
two docs describe the same behavior: read them for that (the /task skill's stale-docs check).
"""
import glob
import io
import os
import re
import sys

problems = []


def rd(path):
    return io.open(path, encoding="utf-8", newline="").read()


files = ["ROADMAP.md", "CLAUDE.md", "README.md", "RELEASING.md"]
files += glob.glob("docs/*.md") + glob.glob("docs/decisions/*.md")
txt = {f.replace(os.sep, "/"): rd(f) for f in files if os.path.exists(f)}
prd = txt["docs/PRD.md"]
road = txt["ROADMAP.md"]

# 1. Requirement ids: every id a ROADMAP task names exists in the PRD,
#    and every id of the current phase's priority has a task.
prd_ids = set(re.findall(r"(?m)^\| ([A-Z]+-\d+a?) \|", prd))
road_ids = set(re.findall(r"\[([A-Z]+-\d+a?)\]", road))
for missing in sorted(road_ids - prd_ids):
    problems.append("ROADMAP names %s, which has no row in the PRD" % missing)
current = max(int(n) for n in re.findall(r"(?m)^## Phase (\d+)", road))
start = road.index("## Phase %d" % current)
section = road[start:].split("\n## ", 1)[0]
cur_ids = set(re.findall(r"(?m)^\| ([A-Z]+-\d+a?) \|.*\| P%d \|" % current, prd))
for missing in sorted(cur_ids - set(re.findall(r"\[([A-Z]+-\d+a?)\]", section))):
    problems.append("PRD %s is P%d but no Phase %d task names it" % (missing, current, current))
if not re.search(r"(?m)^\| \*\*%d · " % current, road):
    problems.append("Phase %d has no row in ROADMAP's Overview table" % current)

# 2. Each phase's progress in the Overview matches its boxes.
sections = re.split(r"(?m)^## Phase (\d+)", road)
for i in range(1, len(sections), 2):
    number, body = sections[i], sections[i + 1].split("\n## ")[0]
    boxes = re.findall(r"(?m)^- \[( |x)\] ", body)
    row = re.search(r"(?m)^\| \*\*%s · .*\| (\d+) / (\d+) \|" % number, road)
    if row and (int(row.group(1)), int(row.group(2))) != (boxes.count("x"), len(boxes)):
        problems.append("Phase %s Overview says %s / %s, its boxes say %d / %d"
                        % (number, row.group(1), row.group(2), boxes.count("x"), len(boxes)))

# 3. Relative links resolve.
for name, text in txt.items():
    for link in re.findall(r"\]\(([^)#\s]+)(?:#[^)]*)?\)", text):
        if link.startswith(("http", "mailto:")):
            continue
        if not os.path.exists(os.path.normpath(os.path.join(os.path.dirname(name), link))):
            problems.append("%s links to %s, which doesn't exist" % (name, link))

# 4. Numbered decision lists have no gap or repeat.
for name, heading in (("docs/DATA_MODEL.md", "## 9. Open questions"), ("docs/UI_FLOWS.md", "## 11. Open questions")):
    tail = txt[name][txt[name].index(heading):]
    nums = [int(n) for n in re.findall(r"(?m)^(\d+)\. \*\*", tail)]
    if nums != list(range(1, len(nums) + 1)):
        problems.append("%s: the answered items under '%s' aren't numbered 1..%d in order" % (name, heading, len(nums)))

# 5. Every decision record is in every list of them.
records = sorted(os.path.basename(f)[:4] for f in glob.glob("docs/decisions/[0-9]*.md"))
for name in ("docs/decisions/README.md", "docs/ARCHITECTURE.md"):
    for number in records:
        if "[%s](" % number not in txt[name]:
            problems.append("%s doesn't list decision record %s" % (name, number))
span = "0001–%s" % records[-1]
for name in ("README.md", "docs/README.md", "docs/ARCHITECTURE.md"):
    if span not in txt[name]:
        problems.append("%s doesn't say 'decision records %s'" % (name, span))

# 6. Line endings and encoding.
for name, text in txt.items():
    if "\r" in text:
        problems.append("%s has CRLF line endings" % name)
    if text.startswith("﻿"):
        problems.append("%s starts with a BOM" % name)

for line in problems:
    print(line)
print("checkdocs: %d problem(s)" % len(problems))
sys.exit(1 if problems else 0)
