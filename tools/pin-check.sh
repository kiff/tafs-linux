#!/usr/bin/env bash
# The chat pin in the window (the iPad's ChatPinUITests, its 845 and 859 tests), in a taf's chat
# and in Coach: a message sent goes to the top within a second, stays there while the answer's
# work and words grow below it, and doesn't move when tafs cloud's copy takes its place. Run under
# a display (xvfb-run); it fails on the first chat whose pin didn't hold.
set -euo pipefail
cd "$(dirname "$0")/.."
app=${TAFS:-target/release/tafs}
run() { # a name, then the app's arguments
  local name=$1
  shift
  local log status=0
  log=$(mktemp)
  timeout 120 "$app" "$@" --pin-check > "$log" 2>&1 || status=$?
  echo "::group::The pin in $name"
  grep "pin-check" "$log" || cat "$log"
  echo "::endgroup::"
  rm -f "$log"
  if [ "$status" -ne 0 ]; then
    echo "The pin didn't hold in $name." >&2
    exit 1
  fi
}
run "a taf's chat" --home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json \
  --chat crates/tafs-app/fixtures/taf-chat.json --chat-stream crates/tafs-app/fixtures/taf-chat-stream.txt \
  --page taf:taf-aaaa0000000000000000
run "Coach" --home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json \
  --coach shared/contracts/coach-conversation.json --coach-stream crates/tafs-app/fixtures/coach-stream.txt \
  --page coach
echo "The pin held in a taf's chat and in Coach."
