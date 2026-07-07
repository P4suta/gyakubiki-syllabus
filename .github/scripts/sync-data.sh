#!/usr/bin/env bash
# Overlay the living data (raw/ + raw-details/) from the `data` branch into the
# working tree, if that branch exists. Read-only — used by the deploy build and
# by the fetch workflows (so incremental crawling sees the latest data). On the
# very first run the branch does not exist yet and this is a no-op (main's seed
# data is used).
set -euo pipefail

if git ls-remote --exit-code --heads origin data >/dev/null 2>&1; then
  git fetch origin data --depth=1 -q
  git checkout FETCH_HEAD -- raw raw-details 2>/dev/null || true
  echo "Overlaid raw/ + raw-details/ from the data branch."
else
  echo "data branch does not exist yet; using the seed data on this branch."
fi
