#!/usr/bin/env bash
# The words and contracts at the pin (shared-pin.json), into shared/. Never into git.
set -euo pipefail
cd "$(dirname "$0")/.."
repo=$(python3 -c 'import json;print(json.load(open("shared-pin.json"))["repo"])')
commit=$(python3 -c 'import json;print(json.load(open("shared-pin.json"))["commit"])')
paths=$(python3 -c 'import json;print(" ".join(json.load(open("shared-pin.json"))["paths"]))')
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
git -C "$work" init -q
git -C "$work" remote add origin "https://github.com/$repo"
git -C "$work" fetch -q --depth 1 origin "$commit"
git -C "$work" -c advice.detachedHead=false checkout -q FETCH_HEAD -- $paths
rm -rf shared && mkdir -p shared
for p in $paths; do
  mkdir -p "shared/$(dirname "$p")"
  cp -R "$work/$p" "shared/$p"
done
# contracts/ lands at shared/contracts, words at shared/app/strings.
echo "$commit" > shared/PIN
echo "shared/ at $repo@$commit"
