#!/usr/bin/env bash
# The AppImage on a clean Ubuntu 24.04 desktop base, run inside an `ubuntu:24.04` container as
# root: tools/check-appimage.sh <the .AppImage> <a directory for the screenshots>.
#
# The container gets what every Ubuntu 24.04 desktop has and the AppImage counts on (linuxdeploy
# leaves those base libraries out on purpose: fribidi, HarfBuzz, fontconfig, FreeType, GL/EGL,
# X11, xkbcommon, the Wayland client), plus a virtual display and a font; never GTK 4 or
# libadwaita, which the AppImage must bring itself. Then it fails on:
# - a library the app finds in neither the AppImage nor that base;
# - an image loader that can't find its own libraries. GTK opens those at run time with no
#   LD_LIBRARY_PATH, so each must find them by its own RUNPATH; the SVG one draws every icon;
# - an icon GTK can't load, in any of the three screens it saves: the first screen, the sign-in,
#   and the sample signed in, whose sidebar draws the app's own icons.
set -euo pipefail
app=$(realpath "$1")
out=$(realpath "$2")
work=$(mktemp -d)
cd "$work"

apt-get update -qq
DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends \
  xvfb xauth fonts-dejavu-core \
  libfribidi0 libharfbuzz0b libfontconfig1 libfreetype6 libexpat1 zlib1g \
  libgl1 libegl1 libglx0 libgbm1 libdrm2 \
  libx11-6 libx11-xcb1 libxcb1 libxext6 libxrender1 libxi6 libxrandr2 libxfixes3 \
  libxcursor1 libxinerama1 libxdamage1 libxcomposite1 libxkbcommon0 \
  libwayland-client0 libwayland-cursor0 libwayland-egl1 > /dev/null
if dpkg -l | grep -qE "libgtk-4|libgtk-3|libadwaita"; then
  echo "Not clean: GTK or libadwaita is installed." >&2
  exit 1
fi
chmod +x "$app"

"$app" --appimage-extract > /dev/null
libs=$(find squashfs-root/usr/lib -type d | tr "\n" ":")
missing=$(LD_LIBRARY_PATH="$libs" ldd squashfs-root/usr/bin/tafs | grep "not found" || true)
if [ -n "$missing" ]; then
  echo "Not found on a clean Ubuntu 24.04 desktop base:" >&2
  echo "$missing" >&2
  exit 1
fi

echo "::group::Image loaders in the AppImage, and what each finds by itself"
loaders=$(find squashfs-root/usr/lib/gdk-pixbuf-2.0 -name "libpixbufloader-*.so" -type f | sort)
if ! grep -q "libpixbufloader-svg.so" <<< "$loaders"; then
  echo "No SVG image loader in the AppImage." >&2
  exit 1
fi
broken=""
for loader in $loaders; do
  gone=$(ldd "$loader" | grep "not found" || true)
  echo "$loader: ${gone:-every library found}"
  if [ -n "$gone" ]; then broken="$broken$loader"$'\n'"$gone"$'\n'; fi
done
grep -v "^#" squashfs-root/usr/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache | grep "\.so" || true
echo "::endgroup::"
if [ -n "$broken" ]; then
  echo "An image loader can't find its libraries by itself, so GTK can't open it:" >&2
  echo "$broken" >&2
  exit 1
fi
rm -rf squashfs-root

export APPIMAGE_EXTRACT_AND_RUN=1 GSK_RENDERER=cairo NO_AT_BRIDGE=1
shot() { # name, then the app's arguments: its screenshot, and what GTK said while drawing it.
  local name=$1
  shift
  timeout 120 xvfb-run -a "$app" "$@" --screenshot "$out/$name.png" 2>&1 | tee "$work/$name.log"
  test -s "$out/$name.png"
}
shot first-screen
shot sign-in --page sign-in
shot sample --sample --signed-in
if grep -h "Failed to load icon" "$work"/*.log | sort -u | grep . >&2; then
  echo "GTK couldn't load those icons." >&2
  exit 1
fi
echo "Three screens drawn, every icon loaded."
