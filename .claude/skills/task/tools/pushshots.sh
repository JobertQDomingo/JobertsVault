#!/usr/bin/env bash
# Adds screenshots to the pr-screenshots branch under <folder>/ without touching the working tree.
# Usage: pushshots.sh <folder> <png>...   Prints the raw URL of each file.
set -euo pipefail
cd /c/Projects/JobertsVault
folder=$1; shift
export GIT_INDEX_FILE=$(mktemp)
rm -f "$GIT_INDEX_FILE"
parent=""
if git fetch -q origin pr-screenshots 2>/dev/null; then
  parent=$(git rev-parse FETCH_HEAD)
  git read-tree "$parent"
fi
for f in "$@"; do
  blob=$(git hash-object -w "$f")
  git update-index --add --cacheinfo 100644,"$blob","$folder/$(basename "$f")"
done
tree=$(git write-tree)
commit=$(git commit-tree "$tree" ${parent:+-p "$parent"} -m "Screenshots for $folder")
git push -q origin "$commit:refs/heads/pr-screenshots"
rm -f "$GIT_INDEX_FILE"
for f in "$@"; do
  echo "https://raw.githubusercontent.com/JobertQDomingo/JobertsVault/pr-screenshots/$folder/$(basename "$f")"
done
