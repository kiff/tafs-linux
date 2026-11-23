#!/usr/bin/env bash
# This repository is public: its code, docs and tests say each rule as itself, never where it came
# from. No coordination thread, team lane, PM time or dated decision. Fails on any, with where.
# CODEOWNERS names logins because it works by them, and this script names what it looks for.
set -uo pipefail
cd "$(dirname "$0")/.."
pattern='kiff/work|\b(qa-kiff|desktop-(windows|macos|linux)|parity-(linux|mac|steward)|legal-i18n|dev-android|mac12|mac-local)\b|\bthe PM\b|\bPM'"'"'s\b|\(PM[ ,)]|owner'"'"'s [0-9]+ [A-Z][a-z]+ rule'
found=$(git grep -nIE "$pattern" -- . ':!.github/CODEOWNERS' ':!tools/public-text.sh')
if [ -n "$found" ]; then
  echo "Public text names a thread, a lane or the PM; say the rule itself:"
  echo "$found"
  exit 1
fi
echo "Public text: clean"
