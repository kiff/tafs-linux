#!/usr/bin/env bash
# Each screen built so far in en, fr and pt, light and dark, at 1280×800 (saved at 2×), into
# screenshots/. Fails if any is missing: a crash at launch, or a word missing at the pin, stops
# the app before it saves. Needs a display: run it under xvfb-run on a headless machine.
set -uo pipefail
cd "$(dirname "$0")/.."
app=${TAFS_APP:-target/release/tafs}
export GSK_RENDERER=${GSK_RENDERER:-cairo} NO_AT_BRIDGE=1 GTK_A11Y=none
mkdir -p screenshots
# The desktop's font is the person's. The screenshots take Inter, the nearest to the iPad's face,
# so every machine saves the same pictures.
export XDG_CONFIG_HOME=$(mktemp -d)
mkdir -p "$XDG_CONFIG_HOME/gtk-4.0"
printf '[Settings]\ngtk-font-name=Inter 11\ngtk-hint-font-metrics=1\n' > "$XDG_CONFIG_HOME/gtk-4.0/settings.ini"
missing=0
for screen in welcome sign-in sign-in-code sign-in-narrow pair connected overview overview-home overview-empty needs-you needs-you-empty needs-you-waits needs-you-happened record taf taf-card taf-home taf-home-card taf-card-lines taf-work taf-run taf-knocks taf-setup taf-units usage usage-kiff usage-kiff-member connect-apps connect-apps-narrow devices people kiff-account account-kiff consent time-zone language alerts calendar calendar-week calendar-compact account sidebar-sample sidebar-others sidebar-setting-up sidebar-plan view-only view-only-paired recipes recipe-sheet gallery-strip gallery halt-all halt-all-halted halt-one record-search record-export record-receipt record-on delete-account pin-sheet coach coach-wider coach-asks coach-chats; do
  case $screen in
    welcome) opts=(--page welcome) ;;
    sign-in) opts=(--page sign-in) ;;
    sign-in-code) opts=(--page sign-in-code) ;;
    sign-in-narrow) opts=(--size 700x900 --page sign-in) ;;
    connected) opts=(--home shared/contracts/home.json --page connected) ;;
    pair) opts=(--page pair) ;;
    overview) opts=(--sample --signed-in) ;;
    overview-home) opts=(--home shared/contracts/home.json) ;;
    # No taf and no recipes listed: the Overview's "No tafs yet", under board 2.
    overview-empty) opts=(--home crates/tafs-app/fixtures/home-empty.json) ;;
    needs-you) opts=(--sample --signed-in --page needs-you) ;;
    # Nothing waiting: board 6 over its words.
    needs-you-empty) opts=(--home shared/contracts/home.json --page needs-you) ;;
    needs-you-waits) opts=(--home crates/tafs-app/fixtures/home-waits.json --page needs-you) ;;
    needs-you-happened) opts=(--home shared/contracts/home.json --unproven crates/tafs-app/fixtures/unproven.json --page needs-you) ;;
    account) opts=(--sample --signed-in --page account) ;;
    calendar) opts=(--sample --signed-in --page calendar) ;;
    calendar-week) opts=(--sample --signed-in --page calendar-week) ;;
    # An upright iPad's size: the compact layout, as the reference set's calendar.png.
    calendar-compact) opts=(--sample --signed-in --page calendar --size 1032x1376) ;;
    devices) opts=(--sample --signed-in --page devices) ;;
    account-kiff) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --account-reads crates/tafs-app/fixtures/account-reads.json --page account) ;;
    consent) opts=(--home shared/contracts/home.json --account-reads crates/tafs-app/fixtures/account-reads-not-given.json --page consent) ;;
    kiff-account) opts=(--home shared/contracts/home.json --people crates/tafs-app/fixtures/people-kiff.json --page kiff-account) ;;
    time-zone) opts=(--home shared/contracts/home.json --people crates/tafs-app/fixtures/people-kiff.json --page time-zone) ;;
    people) opts=(--home shared/contracts/home.json --people crates/tafs-app/fixtures/people-kiff.json --page people) ;;
    connect-apps-narrow) opts=(--sample --signed-in --size 900x800 --page connect-apps) ;;
    connect-apps) opts=(--sample --signed-in --page connect-apps) ;;
    usage) opts=(--sample --signed-in --page usage) ;;
    usage-kiff) opts=(--home shared/contracts/home.json --usage crates/tafs-app/fixtures/usage-kiff.json --page usage) ;;
    usage-kiff-member) opts=(--home shared/contracts/home.json --usage crates/tafs-app/fixtures/usage-kiff-member.json --page usage) ;;
    record) opts=(--sample --signed-in --page record) ;;
    taf) opts=(--sample --signed-in --page taf) ;;
    taf-home) opts=(--home shared/contracts/home.json --page taf:taf-aaaa0000000000000000) ;;
    taf-work) opts=(--home shared/contracts/home.json --work crates/tafs-app/fixtures/taf-work.json --runs crates/tafs-app/fixtures/taf-runs-page.json --page taf:taf-aaaa0000000000000000) ;;
    taf-setup) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --memory crates/tafs-app/fixtures/taf-memory.json --page taf-setup:taf-aaaa0000000000000000) ;;
    taf-knocks) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --doors crates/tafs-app/fixtures/taf-doors.json --page taf:taf-aaaa0000000000000000) ;;
    taf-units) opts=(--home crates/tafs-app/fixtures/home-waits.json --page taf:taf-aaaa0000000000000000) ;;
    taf-run) opts=(--home shared/contracts/home.json --work crates/tafs-app/fixtures/taf-work.json --runs crates/tafs-app/fixtures/taf-runs-page.json --request crates/tafs-app/fixtures/taf-run-request.json --page run:taf-aaaa0000000000000000) ;;
    taf-card-lines) opts=(--home crates/tafs-app/fixtures/home-card-lines.json --page taf-card:taf-aaaa0000000000000000) ;;
    taf-card) opts=(--sample --signed-in --page taf-card) ;;
    taf-home-card) opts=(--home shared/contracts/home.json --page taf-card:taf-aaaa0000000000000000) ;;
    sidebar-plan) opts=(--home shared/contracts/home.json --balance crates/tafs-app/fixtures/balance-home-line.json --page overview) ;;
    sidebar-setting-up) opts=(--home crates/tafs-app/fixtures/home-setting-up.json --page overview) ;;
    sidebar-others) opts=(--home crates/tafs-app/fixtures/home-others.json --page overview) ;;
    sidebar-sample) opts=(--sample) ;;
    view-only) opts=(--view-only) ;;
    view-only-paired) opts=(--view-only --paired) ;;
    recipes) opts=(--home crates/tafs-app/fixtures/home-empty.json --recipes crates/tafs-app/fixtures/recipes.json) ;;
    pin-sheet) opts=(--home shared/contracts/home.json --page pin-sheet) ;;
    delete-account) opts=(--home shared/contracts/home.json --people crates/tafs-app/fixtures/people-kiff.json --page delete-account) ;;
    record-export) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access.json --record-search crates/tafs-app/fixtures/record-search.json --receipts crates/tafs-app/fixtures/receipts.json --page record-export) ;;
    alerts) opts=(--home shared/contracts/home.json --page alerts) ;;
    language) opts=(--sample --signed-in --page language) ;;
    record-receipt) opts=(--home shared/contracts/home.json --activity crates/tafs-app/fixtures/activity.json --receipt crates/tafs-app/fixtures/receipt.json --page receipt:prop_receipt_1) ;;
    record-on) opts=(--home shared/contracts/home.json --activity crates/tafs-app/fixtures/activity.json --calls crates/tafs-app/fixtures/calls.json --page record) ;;
    record-search) opts=(--home shared/contracts/home.json --record-search crates/tafs-app/fixtures/record-search.json --page record-search:1042) ;;
    halt-all) opts=(--home shared/contracts/home.json --halt crates/tafs-app/fixtures/halt-halting.json --page halt-all) ;;
    # Halt all once every part holds it: board 4 over the screen.
    halt-all-halted) opts=(--home shared/contracts/home.json --halt crates/tafs-app/fixtures/halt-halted.json --page halt-all) ;;
    halt-one) opts=(--home shared/contracts/home.json --halt crates/tafs-app/fixtures/halt-halted.json --page halt:taf-aaaa0000000000000000) ;;
    gallery-strip) opts=(--home shared/contracts/home.json --gallery crates/tafs-app/fixtures/gallery.json) ;;
    gallery) opts=(--home shared/contracts/home.json --gallery crates/tafs-app/fixtures/gallery.json --recipes crates/tafs-app/fixtures/recipes.json --page gallery) ;;
    # Coach's conversation from tafs cloud's contract, and one with an approval waiting under its wider line.
    coach) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --coach shared/contracts/coach-conversation.json --page coach) ;;
    coach-wider) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --coach shared/contracts/coach-wider.json --page coach) ;;
    coach-asks) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --coach crates/tafs-app/fixtures/coach-asks.json --page coach) ;;
    coach-chats) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --coach shared/contracts/coach-conversation.json --coach-chats shared/contracts/coach-chats.json --page coach) ;;
    recipe-sheet) opts=(--home crates/tafs-app/fixtures/home-empty.json --recipes crates/tafs-app/fixtures/recipes.json --page recipe:refunds) ;;
  esac
  for lang in en fr pt; do
    for theme in light dark; do
      out="screenshots/$screen-$lang-$theme.png"
      rm -f "$out"
      "$app" "${opts[@]}" --lang "$lang" --theme "$theme" --screenshot "$out" >/dev/null 2>&1
      if [ ! -s "$out" ]; then echo "missing: $out"; missing=$((missing + 1)); fi
    done
  done
done
echo "$(ls screenshots/*.png | wc -l) screenshots, $missing missing"
[ "$missing" -eq 0 ]
