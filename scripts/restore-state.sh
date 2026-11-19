#!/usr/bin/env bash
set -euo pipefail

mkdir -p data
if git ls-remote --exit-code origin state >/dev/null 2>&1; then
  git fetch --depth=1 origin state
  if git show FETCH_HEAD:state.sqlite3 >/tmp/ainews-state.sqlite3 2>/dev/null; then
    cp /tmp/ainews-state.sqlite3 data/state.sqlite3
    echo "restored state branch"
  else
    echo "state branch has no state.sqlite3; starting fresh"
  fi
else
  echo "no state branch; starting fresh"
fi
