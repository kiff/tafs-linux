#!/usr/bin/env bash
# Each screen built so far in en, fr, ar (right to left) and ja, light and dark (TAFS_LANGS to change them), at 1280×800 (saved at 2×), into
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
# A held call from KIFF, held a moment ago and lapsing in ten minutes, as the iPad's card counts it.
held=$(mktemp)
sed -e "s/__HELD_AT__/$(date -u -d '-30 sec' +%FT%TZ)/" -e "s/__EXPIRES__/$(date -u -d '+10 min' +%FT%TZ)/" \
  crates/tafs-app/fixtures/held-kiff.json > "$held"
# The same hold, over a total the Card keeps across calls: Approve once can't pass it, Extra room can.
held_total=$(mktemp)
sed -e 's/"can_approve_once": true/"can_approve_once": false/' "$held" > "$held_total"
for screen in welcome welcome-ticked welcome-terms-changed cards-1 cards-2 cards-3 how-it-works sign-in sign-in-code sign-in-narrow pair connected overview overview-home overview-empty first-home overview-unread needs-you needs-you-kiff needs-you-key needs-you-key-room needs-you-empty needs-you-waits needs-you-set-all-aside needs-you-dismiss needs-you-hold needs-you-happened record taf taf-card taf-home taf-home-card taf-card-lines taf-work taf-work-manage taf-next taf-receipt-report taf-report taf-card-manage taf-run taf-knocks doors doors-door doors-api doors-api-new-key doors-api-instructions taf-dial taf-setup-apps taf-starts taf-results taf-setup taf-units usage usage-kiff usage-kiff-member usage-ask usage-decline connect-apps connect-apps-narrow connect-app connect-connection connect-remove connect-kiff connect-which-taf connect-approve devices devices-sign-out people people-rename people-add files files-file chrome share share-step2 kiff-account account-kiff consent time-zone time-zone-use language alerts calendar calendar-week calendar-compact account sidebar-sample sidebar-others sidebar-setting-up discard-setup sidebar-unreachable sidebar-new-version sidebar-plan view-only view-only-paired recipes recipe-sheet gallery-strip gallery new-taf coach-hand halt-all halt-all-halted halt-one halt-resume record-search record-export record-receipt record-on delete-account pin-sheet pin-sheet-add app-event coach coach-attach coach-wider coach-stream coach-stream-healed coach-asks coach-read-back coach-improve coach-improve-files coach-chats coach-watch coach-watch-day coach-empty coach-start coach-draft coach-draft-refused coach-tools coach-full coach-drive taf-chat taf-chat-outcomes taf-chat-knocks taf-chat-attach taf-versions taf-versions-trial taf-trial taf-chat-working taf-chat-stream taf-report brief brief-answer brief-clear overview-labels overview-label mission-log; do
  case $screen in
    welcome) opts=(--page welcome) ;;
    # The welcome's tick (K5): ticked, Continue on; and as KIFF leaves it when the terms changed.
    welcome-ticked) opts=(--page welcome-ticked) ;;
    welcome-terms-changed) opts=(--page welcome-terms-changed) ;;
    # The three cards on how tafs works, over Home, and How tafs works from Account.
    cards-1) opts=(--home shared/contracts/home.json --page cards:1) ;;
    cards-2) opts=(--home shared/contracts/home.json --page cards:2) ;;
    cards-3) opts=(--home shared/contracts/home.json --page cards:3) ;;
    how-it-works) opts=(--home shared/contracts/home.json --page how-it-works) ;;
    sign-in) opts=(--page sign-in) ;;
    sign-in-code) opts=(--page sign-in-code) ;;
    sign-in-narrow) opts=(--size 700x900 --page sign-in) ;;
    connected) opts=(--home shared/contracts/home.json --page connected) ;;
    pair) opts=(--page pair) ;;
    overview) opts=(--sample --signed-in) ;;
    overview-home) opts=(--home shared/contracts/home.json) ;;
    # No taf and no recipes listed: the Overview's "No tafs yet", under board 2.
    overview-empty) opts=(--home crates/tafs-app/fixtures/home-empty.json) ;;
    # The first Home: no taf, for someone who may use Coach (Ask Coach, How tafs works).
    first-home) opts=(--home crates/tafs-app/fixtures/home-empty.json --me crates/tafs-app/fixtures/me-manager.json) ;;
    # Home couldn't be read yet: the mascot and "Couldn't load your tafs.", the Unreachable line.
    overview-unread) opts=(--view-only --unreachable network) ;;
    needs-you) opts=(--sample --signed-in --page needs-you) ;;
    # A held call from KIFF, as this computer answers it: Answer in KIFF.
    needs-you-kiff) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --held "$held" --page needs-you) ;;
    # A held call from KIFF on a computer paired with a security key: Approve once here, signed
    # in the answer (kiff.answer.v1), above "answer it in KIFF". Nothing is signed.
    needs-you-key) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --held "$held" --page needs-you-key) ;;
    # Over a total, for someone who manages Cards, with the security key: Extra room (change-card),
    # signed in the answer, and why. Nothing is signed.
    needs-you-key-room) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --held "$held_total" --page needs-you-key-room) ;;
    # Nothing waiting: board 6 over its words.
    needs-you-empty) opts=(--home shared/contracts/home.json --page needs-you) ;;
    needs-you-waits) opts=(--home crates/tafs-app/fixtures/home-waits.json --page needs-you) ;;
    # "Set all aside" tapped: tafs cloud's one ask, its yes and no. A Card unit request's Dismiss
    # tapped: it's signed like a refusal, so the can't-approve line under it, nothing sent.
    needs-you-set-all-aside) opts=(--home crates/tafs-app/fixtures/home-waits.json --page needs-you-set-all-aside) ;;
    # A hold's notification clicked: Needs you with that request brought to the top.
    needs-you-hold) opts=(--home crates/tafs-app/fixtures/home-waits.json --page needs-you-hold:exc-example) ;;
    needs-you-dismiss) opts=(--home crates/tafs-app/fixtures/home-waits.json --page needs-you-dismiss) ;;
    needs-you-happened) opts=(--home shared/contracts/home.json --unproven crates/tafs-app/fixtures/unproven.json --page needs-you) ;;
    account) opts=(--sample --signed-in --page account) ;;
    calendar) opts=(--sample --signed-in --page calendar) ;;
    calendar-week) opts=(--sample --signed-in --page calendar-week) ;;
    # An upright iPad's size: the compact layout, as the reference set's calendar.png.
    calendar-compact) opts=(--sample --signed-in --page calendar --size 1032x1376) ;;
    devices) opts=(--sample --signed-in --page devices) ;;
    devices-sign-out) opts=(--sample --signed-in --page devices-sign-out) ;;
    account-kiff) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --account-reads crates/tafs-app/fixtures/account-reads.json --balance crates/tafs-app/fixtures/balance-home-line.json --page account) ;;
    consent) opts=(--home shared/contracts/home.json --account-reads crates/tafs-app/fixtures/account-reads-not-given.json --page consent) ;;
    kiff-account) opts=(--home shared/contracts/home.json --people crates/tafs-app/fixtures/people-kiff.json --page kiff-account) ;;
    time-zone) opts=(--home shared/contracts/home.json --people crates/tafs-app/fixtures/people-kiff.json --page time-zone) ;;
    time-zone-use) opts=(--home shared/contracts/home.json --people crates/tafs-app/fixtures/people-kiff.json --page time-zone-use) ;;
    people) opts=(--home shared/contracts/home.json --people crates/tafs-app/fixtures/people-kiff.json --page people) ;;
    # Files from tafs cloud's contracts: the account's, and its first file open.
    files) opts=(--home shared/contracts/home.json --files shared/contracts/files-list.json --page files) ;;
    files-file) opts=(--home shared/contracts/home.json --files shared/contracts/files-list.json --page files-file) ;;
    # Account › Chrome: the Chromes planque lists (a fixture), Unpair, and Pair a Chrome (the phone's).
    chrome) opts=(--home shared/contracts/home.json --chromes crates/tafs-app/fixtures/chromes.json --page chrome) ;;
    # Renaming a group with Save tapped: this computer can't approve, and sends nothing.
    people-rename) opts=(--home shared/contracts/home.json --people crates/tafs-app/fixtures/people-kiff.json --page people-rename) ;;
    people-add) opts=(--home shared/contracts/home.json --people crates/tafs-app/fixtures/people-kiff.json --page people-add) ;;
    connect-apps-narrow) opts=(--sample --signed-in --size 900x800 --page connect-apps) ;;
    connect-apps) opts=(--sample --signed-in --page connect-apps) ;;
    # The reference set's connections: Stripe's, one connection's detail, and its Remove access.
    connect-app) opts=(--sample --signed-in --page connect-app) ;;
    connect-connection) opts=(--sample --signed-in --page connect-connection) ;;
    connect-remove) opts=(--sample --signed-in --page connect-remove) ;;
    # Signed in with KIFF, read from a fixture: Connect, "Which taf?", and a taf picked (can't approve here).
    connect-kiff) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --connect crates/tafs-app/fixtures/connect-kiff.json --page connect-apps) ;;
    connect-which-taf) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --connect crates/tafs-app/fixtures/connect-kiff.json --page connect-which-taf) ;;
    connect-approve) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --connect crates/tafs-app/fixtures/connect-kiff.json --page connect-approve) ;;
    usage) opts=(--sample --signed-in --page usage) ;;
    usage-kiff) opts=(--home shared/contracts/home.json --usage crates/tafs-app/fixtures/usage-kiff.json --page usage) ;;
    usage-kiff-member) opts=(--home shared/contracts/home.json --usage crates/tafs-app/fixtures/usage-kiff-member.json --page usage) ;;
    usage-ask) opts=(--home shared/contracts/home.json --usage crates/tafs-app/fixtures/usage-kiff-member.json --page usage-ask) ;;
    # Declining a request with Decline tapped: this computer can't approve, and sends nothing.
    usage-decline) opts=(--home shared/contracts/home.json --usage crates/tafs-app/fixtures/usage-kiff.json --page usage-decline) ;;
    record) opts=(--sample --signed-in --page record) ;;
    taf) opts=(--sample --signed-in --page taf) ;;
    taf-home) opts=(--home shared/contracts/home.json --page taf:taf-aaaa0000000000000000) ;;
    taf-work) opts=(--home shared/contracts/home.json --work crates/tafs-app/fixtures/taf-work.json --runs crates/tafs-app/fixtures/taf-runs-page.json --page taf:taf-aaaa0000000000000000) ;;
    # Its work for whoever manages it (Answer within, Send again), and its Card for a view-only
    # session's manager (Revoke's line). Nothing is sent.
    # A payment's receipt with "Report this payment", and its sheet (`taf-activity-report.json`).
    taf-receipt-report) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --me crates/tafs-app/fixtures/me-manager.json --taf-activity crates/tafs-app/fixtures/taf-activity-report.json --page taf-receipt:taf-aaaa0000000000000000:pay-1) ;;
    taf-report) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --me crates/tafs-app/fixtures/me-manager.json --taf-activity crates/tafs-app/fixtures/taf-activity-report.json --page taf-report:taf-aaaa0000000000000000:pay-1) ;;
    taf-next) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --me crates/tafs-app/fixtures/me-manager.json --taf-activity crates/tafs-app/fixtures/taf-activity-next.json --page taf:taf-aaaa0000000000000000) ;;
    taf-work-manage) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --me crates/tafs-app/fixtures/me-manager.json --work crates/tafs-app/fixtures/taf-work.json --runs crates/tafs-app/fixtures/taf-runs-page.json --page taf:taf-aaaa0000000000000000) ;;
    taf-card-manage) opts=(--home crates/tafs-app/fixtures/home-card-lines.json --access crates/tafs-app/fixtures/access-manage.json --me crates/tafs-app/fixtures/me-manager.json --page taf-card:taf-aaaa0000000000000000) ;;
    taf-setup) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --memory crates/tafs-app/fixtures/taf-memory.json --page taf-setup:taf-aaaa0000000000000000) ;;
    taf-knocks) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --doors crates/tafs-app/fixtures/taf-doors.json --page taf:taf-aaaa0000000000000000) ;;
    # A taf's doors (`DoorsView`): the doors' own dial over the door at the top, and one door's
    # screen with its keys and knocks. Read from a file (no contract at the pin); nothing is sent.
    doors) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --me crates/tafs-app/fixtures/me-manager.json --doors crates/tafs-app/fixtures/taf-doors-dial.json --page doors:taf-aaaa0000000000000000) ;;
    # The taf's dial in the doors' own dial's place, over its doors (synthetic: the iPad's
    # `Screenshots.knob` with four services). Nothing is sent.
    taf-dial) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --me crates/tafs-app/fixtures/me-manager.json --doors crates/tafs-app/fixtures/taf-doors-dial.json --knob crates/tafs-app/fixtures/taf-knob-dial.json --work crates/tafs-app/fixtures/taf-work.json --page doors:taf-aaaa0000000000000000) ;;
    # A door "From another application" (tafs cloud's contract): today against its limit, the last
    # request in tafs cloud's words, its keys (never a key). Paused with one key, New key tapped:
    # signed, so this computer says it can't, with "Approve on your phone instead". And its API
    # instructions.
    doors-api) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --me crates/tafs-app/fixtures/me-manager.json --doors crates/tafs-app/fixtures/taf-doors-api.json --page doors:taf-aaaa0000000000000000:door_ord7) ;;
    doors-api-new-key) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --me crates/tafs-app/fixtures/me-manager.json --doors crates/tafs-app/fixtures/taf-doors-api-paused.json --size 1280x1000 --page doors:taf-aaaa0000000000000000:door_ord7:door.api.new_key) ;;
    doors-api-instructions) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --me crates/tafs-app/fixtures/me-manager.json --doors crates/tafs-app/fixtures/taf-doors-api.json --page doors:taf-aaaa0000000000000000:door_ord7:door.api.instructions) ;;
    doors-door) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --me crates/tafs-app/fixtures/me-manager.json --doors crates/tafs-app/fixtures/taf-doors-dial.json --page doors:taf-aaaa0000000000000000:door_email) ;;
    # Setup with its own Connect again and a connected app on the ring (tafs cloud's contract);
    # Starts when and Results go to over its dial from a file (synthetic, the iPad's
    # `Screenshots.setupKnob` with a version waiting). Nothing is sent.
    taf-setup-apps) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --me crates/tafs-app/fixtures/me-manager.json --setup shared/contracts/taf-setup-reconnect.json --knob crates/tafs-app/fixtures/taf-knob-setup.json --page taf-setup:taf-aaaa0000000000000000) ;;
    # Taller than the rest, so the app events' card shows whole, Approve's note under it included.
    taf-starts) opts=(--size 1280x1000 --home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --me crates/tafs-app/fixtures/me-manager.json --knob crates/tafs-app/fixtures/taf-knob-setup.json --doors crates/tafs-app/fixtures/taf-doors-dial.json --page starts:taf-aaaa0000000000000000) ;;
    taf-results) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --me crates/tafs-app/fixtures/me-manager.json --knob crates/tafs-app/fixtures/taf-knob-setup.json --work crates/tafs-app/fixtures/taf-work.json --page results:taf-aaaa0000000000000000) ;;
    # A taf's chat from a file (synthetic: no contract for a taf's own thread at the pin): at rest, and at work.
    # Files attached in a chat's box (synthetic): one ready, one going up, one that didn't go, each
    # with its remove control; they go with the next message.
    taf-chat-attach) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --chat crates/tafs-app/fixtures/taf-chat.json --attach crates/tafs-app/fixtures/attach-tray.json --page taf:taf-aaaa0000000000000000) ;;
    # A taf's versions (synthetic): newest first, the live one marked, an ended trial's line; and
    # with a trial open, Resume; then the trial itself, its bar, guide and chat. What the phone
    # signs is drawn up to the signature.
    taf-versions) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --chat crates/tafs-app/fixtures/taf-chat.json --versions crates/tafs-app/fixtures/versions.json --page versions:taf-aaaa0000000000000000) ;;
    taf-versions-trial) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --chat crates/tafs-app/fixtures/taf-chat.json --versions crates/tafs-app/fixtures/versions.json --trial crates/tafs-app/fixtures/trial.json --page versions:taf-aaaa0000000000000000) ;;
    taf-trial) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --chat crates/tafs-app/fixtures/taf-chat-working.json --versions crates/tafs-app/fixtures/versions.json --trial crates/tafs-app/fixtures/trial.json --page trial:taf-aaaa0000000000000000) ;;
    taf-chat) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --chat crates/tafs-app/fixtures/taf-chat.json --page taf:taf-aaaa0000000000000000) ;;
    # What became of its calls, from KIFF's activity (`OutcomeLine`): one approved by you and done,
    # where it was answered; one refused, when it was made.
    taf-chat-outcomes) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --activity crates/tafs-app/fixtures/taf-chat-activity.json --chat crates/tafs-app/fixtures/taf-chat.json --page taf:taf-aaaa0000000000000000) ;;
    # A door's knocks in its chat, at their times (`KnockCard`), from the doors' fixture.
    taf-chat-knocks) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --doors crates/tafs-app/fixtures/taf-doors.json --chat crates/tafs-app/fixtures/taf-chat.json --page taf:taf-aaaa0000000000000000) ;;
    taf-chat-working) opts=(--home shared/contracts/home.json --chat crates/tafs-app/fixtures/taf-chat-working.json --page taf:taf-aaaa0000000000000000) ;;
    taf-chat-stream) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --chat crates/tafs-app/fixtures/taf-chat.json --chat-stream crates/tafs-app/fixtures/taf-chat-stream.txt --page taf:taf-aaaa0000000000000000) ;;
    # Report this answer, open over the chat's last answer: its Send sends nothing.
    taf-report) opts=(--home shared/contracts/home.json --chat crates/tafs-app/fixtures/taf-chat.json --report --page taf:taf-aaaa0000000000000000) ;;
    taf-units) opts=(--home crates/tafs-app/fixtures/home-waits.json --page taf:taf-aaaa0000000000000000) ;;
    taf-run) opts=(--home shared/contracts/home.json --work crates/tafs-app/fixtures/taf-work.json --runs crates/tafs-app/fixtures/taf-runs-page.json --request crates/tafs-app/fixtures/taf-run-request.json --page run:taf-aaaa0000000000000000) ;;
    taf-card-lines) opts=(--home crates/tafs-app/fixtures/home-card-lines.json --page taf-card:taf-aaaa0000000000000000) ;;
    taf-card) opts=(--sample --signed-in --page taf-card) ;;
    taf-home-card) opts=(--home shared/contracts/home.json --page taf-card:taf-aaaa0000000000000000) ;;
    sidebar-plan) opts=(--home shared/contracts/home.json --balance crates/tafs-app/fixtures/balance-home-line.json --page overview) ;;
    # Discard tapped on a taf being set up, then its yes: tafs cloud's ask, then the can't-approve
    # line under it (its yes is signed, as a removal is), nothing revoked or sent.
    discard-setup) opts=(--home crates/tafs-app/fixtures/home-setting-up.json --me crates/tafs-app/fixtures/me-manager.json --coach-setup crates/tafs-app/fixtures/setup-coach.json --page discard-setup) ;;
    sidebar-setting-up) opts=(--home crates/tafs-app/fixtures/home-setting-up.json --me crates/tafs-app/fixtures/me-manager.json --coach-setup crates/tafs-app/fixtures/setup-coach.json --page overview) ;;
    # Home couldn't be read: the Unreachable line at the foot, as the app tries again (nothing is read).
    sidebar-unreachable) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --unreachable network) ;;
    sidebar-new-version) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --new-version v1.0.2) ;;
    sidebar-others) opts=(--home crates/tafs-app/fixtures/home-others.json --page overview) ;;
    sidebar-sample) opts=(--sample) ;;
    view-only) opts=(--view-only) ;;
    view-only-paired) opts=(--view-only --paired) ;;
    recipes) opts=(--home crates/tafs-app/fixtures/home-empty.json --recipes crates/tafs-app/fixtures/recipes.json) ;;
    pin-sheet) opts=(--home shared/contracts/home.json --page pin-sheet) ;;
    # The same sheet when the computer is paired, adding the key ("Pair without" in Cancel's place).
    pin-sheet-add) opts=(--home shared/contracts/home.json --page pin-sheet-add) ;;
    # An app event's filters before signing (synthetic, the iPad's test's stop): Approve is the gate's line.
    app-event) opts=(--home shared/contracts/home.json --app-event crates/tafs-app/fixtures/app-event.json) ;;
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
    # Resume tapped on a halted taf: resuming is confirmed and signed by the device, so the
    # can't-approve line under it, nothing sent.
    halt-resume) opts=(--home shared/contracts/home.json --halt crates/tafs-app/fixtures/halt-halted.json --page halt-resume:taf-aaaa0000000000000000) ;;
    halt-one) opts=(--home shared/contracts/home.json --halt crates/tafs-app/fixtures/halt-halted.json --page halt:taf-aaaa0000000000000000) ;;
    gallery-strip) opts=(--home shared/contracts/home.json --gallery crates/tafs-app/fixtures/gallery.json) ;;
    gallery) opts=(--home shared/contracts/home.json --gallery crates/tafs-app/fixtures/gallery.json --recipes crates/tafs-app/fixtures/recipes.json --page gallery) ;;
    # Coach's conversation from tafs cloud's contract, and one with an approval waiting under its wider line.
    coach-attach) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --coach shared/contracts/coach-conversation.json --attach crates/tafs-app/fixtures/attach-tray.json --page coach) ;;
    coach) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --coach shared/contracts/coach-conversation.json --page coach) ;;
    coach-wider) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --coach shared/contracts/coach-wider.json --page coach) ;;
    coach-stream) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --coach shared/contracts/coach-conversation.json --coach-stream crates/tafs-app/fixtures/coach-stream.txt --page coach) ;;
    # An answer cut off mid-Markdown (an open bold, a link without its address yet): what it's in
    # the middle of closed, so no symbol shows while it streams (SteadyMarkdown).
    coach-stream-healed) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --coach shared/contracts/coach-conversation.json --coach-stream crates/tafs-app/fixtures/coach-stream-healed.txt --page coach) ;;
    # A new conversation on a taf (`improveTaf`): New taf or Improve Rae, and three first messages.
    coach-improve) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --coach crates/tafs-app/fixtures/coach-empty.json --page coach-improve:taf-aaaa0000000000000000) ;;
    # The same, with files picked before: they wait for the next message, and the box says so.
    coach-improve-files) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --coach crates/tafs-app/fixtures/coach-empty.json --attach crates/tafs-app/fixtures/attach-tray.json --page coach-improve:taf-aaaa0000000000000000) ;;
    # The same, read back to its top: ↓ to the latest over the conversation.
    coach-read-back) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --coach crates/tafs-app/fixtures/coach-asks.json --page coach-read-back) ;;
    coach-asks) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --coach crates/tafs-app/fixtures/coach-asks.json --page coach) ;;
    coach-chats) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --coach shared/contracts/coach-conversation.json --coach-chats shared/contracts/coach-chats.json --page coach) ;;
    # W11, From your watch, under Previous conversations (the fixture's two days).
    coach-watch) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --coach shared/contracts/coach-conversation.json --coach-chats shared/contracts/coach-chats.json --watch-days crates/tafs-app/fixtures/watch-days.json --page coach) ;;
    coach-watch-day) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --coach shared/contracts/coach-conversation.json --coach-chats shared/contracts/coach-chats.json --watch-days crates/tafs-app/fixtures/watch-days.json --page coach-watch-day) ;;
    coach-empty) opts=(--home crates/tafs-app/fixtures/home-empty.json --me crates/tafs-app/fixtures/me-manager.json --coach crates/tafs-app/fixtures/coach-empty.json --page coach) ;;
    # An approval naming KIFF's draft, read from a file (--coach-drafts, nothing is read): the Card drawn, and KIFF's refusal with Try again.
    coach-draft) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --coach crates/tafs-app/fixtures/coach-card.json --coach-drafts crates/tafs-app/fixtures/coach-drafts.json --page coach) ;;
    coach-draft-refused) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --coach crates/tafs-app/fixtures/coach-card.json --coach-drafts crates/tafs-app/fixtures/coach-drafts-refused.json --page coach) ;;
    # Coach's tools still to give (tafs cloud's contract): the row over the conversation, and the step open, the gate's line in Give's place.
    coach-tools) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --coach crates/tafs-app/fixtures/coach-asks.json --coach-setup shared/contracts/setup-coach.json --page coach-tools) ;;
    # Coach mode full screen: the conversation and, beside it, what it has made so far, with KIFF's draft of the Card.
    coach-full) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --coach crates/tafs-app/fixtures/coach-card.json --coach-drafts crates/tafs-app/fixtures/coach-drafts.json --page coach-full) ;;
    # Coach asks to drive the app (approving it is the phone's), after a drive that ended: what it did and why it ended.
    coach-drive) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --coach crates/tafs-app/fixtures/coach-drive.json --page coach-full) ;;
    coach-start) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --coach shared/contracts/setup-start.json --page coach) ;;
    # Your brief in Needs you's place, from tafs cloud's answer in a file (nothing is sent): with a
    # signed answer tapped (ApprovalGate's words), and with nothing to decide (board 3).
    brief) opts=(--home crates/tafs-app/fixtures/home-missions.json --me crates/tafs-app/fixtures/me-manager.json --brief crates/tafs-app/fixtures/brief.json --page needs-you) ;;
    brief-answer) opts=(--home crates/tafs-app/fixtures/home-missions.json --me crates/tafs-app/fixtures/me-manager.json --brief crates/tafs-app/fixtures/brief.json --size 1280x1500 --page brief-answer:hold:ce_pay) ;;
    brief-clear) opts=(--home crates/tafs-app/fixtures/home-missions.json --brief crates/tafs-app/fixtures/brief-clear.json --page needs-you) ;;
    # A mission's log, opened from Done's Evidence over the brief.
    mission-log) opts=(--home crates/tafs-app/fixtures/home-missions.json --me crates/tafs-app/fixtures/me-manager.json --brief crates/tafs-app/fixtures/brief.json --mission-log crates/tafs-app/fixtures/mission-log.json --page mission-log:m_money) ;;
    # Home's labels as chips over the Overview's grid and on each taf's line, and filtered to one.
    overview-labels) opts=(--home crates/tafs-app/fixtures/home-missions.json) ;;
    overview-label) opts=(--home crates/tafs-app/fixtures/home-missions.json --page overview-label:l_clients) ;;
    # + New's gallery: "Say it in your own words" with board 8 beside it, then every job.
    new-taf) opts=(--home shared/contracts/home.json --me crates/tafs-app/fixtures/me-manager.json --gallery crates/tafs-app/fixtures/gallery.json --recipes crates/tafs-app/fixtures/recipes.json --page new-taf) ;;
    # A job handed to Coach from its sheet: a new conversation, the job's words in the box, nothing sent.
    coach-hand) opts=(--home crates/tafs-app/fixtures/home-empty.json --me crates/tafs-app/fixtures/me-manager.json --recipes crates/tafs-app/fixtures/recipes.json --coach crates/tafs-app/fixtures/coach-empty.json --page coach-hand:refunds) ;;
    # Share on a taf's page and its sheet's two steps (from --people's grants), Share tapped on the
    # second: the can't-approve line, nothing sent.
    share) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --people crates/tafs-app/fixtures/people-kiff.json --page share:taf-aaaa0000000000000000) ;;
    share-step2) opts=(--home shared/contracts/home.json --access crates/tafs-app/fixtures/access-manage.json --people crates/tafs-app/fixtures/people-kiff.json --page share-step2:taf-aaaa0000000000000000) ;;
    recipe-sheet) opts=(--home crates/tafs-app/fixtures/home-empty.json --recipes crates/tafs-app/fixtures/recipes.json --page recipe:refunds) ;;
  esac
  for lang in ${TAFS_LANGS:-en fr ar ja}; do
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
