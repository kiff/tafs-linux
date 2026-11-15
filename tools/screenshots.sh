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
for screen in welcome sign-in pair overview overview-home needs-you needs-you-waits record taf taf-card taf-home taf-home-card usage usage-kiff usage-kiff-member connect-apps devices people kiff-account time-zone calendar calendar-week calendar-compact account sidebar-sample view-only view-only-paired; do
  case $screen in
    welcome) opts=(--page welcome) ;;
    sign-in) opts=(--page sign-in) ;;
    pair) opts=(--page pair) ;;
    overview) opts=(--sample --signed-in) ;;
    overview-home) opts=(--home shared/contracts/home.json) ;;
    needs-you) opts=(--sample --signed-in --page needs-you) ;;
    needs-you-waits) opts=(--home crates/tafs-app/fixtures/home-waits.json --page needs-you) ;;
    account) opts=(--sample --signed-in --page account) ;;
    calendar) opts=(--sample --signed-in --page calendar) ;;
    calendar-week) opts=(--sample --signed-in --page calendar-week) ;;
    # An upright iPad's size: the compact layout, as the reference set's calendar.png.
    calendar-compact) opts=(--sample --signed-in --page calendar --size 1032x1376) ;;
    devices) opts=(--sample --signed-in --page devices) ;;
    kiff-account) opts=(--home shared/contracts/home.json --people crates/tafs-app/fixtures/people-kiff.json --page kiff-account) ;;
    time-zone) opts=(--home shared/contracts/home.json --people crates/tafs-app/fixtures/people-kiff.json --page time-zone) ;;
    people) opts=(--home shared/contracts/home.json --people crates/tafs-app/fixtures/people-kiff.json --page people) ;;
    connect-apps) opts=(--sample --signed-in --page connect-apps) ;;
    usage) opts=(--sample --signed-in --page usage) ;;
    usage-kiff) opts=(--home shared/contracts/home.json --usage crates/tafs-app/fixtures/usage-kiff.json --page usage) ;;
    usage-kiff-member) opts=(--home shared/contracts/home.json --usage crates/tafs-app/fixtures/usage-kiff-member.json --page usage) ;;
    record) opts=(--sample --signed-in --page record) ;;
    taf) opts=(--sample --signed-in --page taf) ;;
    taf-home) opts=(--home shared/contracts/home.json --page taf:taf-aaaa0000000000000000) ;;
    taf-card) opts=(--sample --signed-in --page taf-card) ;;
    taf-home-card) opts=(--home shared/contracts/home.json --page taf-card:taf-aaaa0000000000000000) ;;
    sidebar-sample) opts=(--sample) ;;
    view-only) opts=(--view-only) ;;
    view-only-paired) opts=(--view-only --paired) ;;
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
