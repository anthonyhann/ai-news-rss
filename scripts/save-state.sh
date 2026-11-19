#!/usr/bin/env bash
set -euo pipefail

if [[ ! -f data/state.sqlite3 ]]; then
  echo "no data/state.sqlite3 to save"
  exit 0
fi
if [[ -z "${GITHUB_REPOSITORY:-}" || -z "${GITHUB_TOKEN:-}" ]]; then
  echo "GITHUB_REPOSITORY/GITHUB_TOKEN not set; skip state save"
  exit 0
fi

tmp="$(mktemp -d)"
cp data/state.sqlite3 "$tmp/state.sqlite3"
git -C "$tmp" init -q
git -C "$tmp" config user.name "github-actions[bot]"
git -C "$tmp" config user.email "41898282+github-actions[bot]@users.noreply.github.com"
git -C "$tmp" add state.sqlite3
git -C "$tmp" commit -q -m "state: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
git -C "$tmp" remote add origin "https://x-access-token:${GITHUB_TOKEN}@github.com/${GITHUB_REPOSITORY}.git"
git -C "$tmp" push --quiet --force origin HEAD:state
echo "saved state branch ($(wc -c < data/state.sqlite3) bytes)"
