#!/usr/bin/env bash
# tafs as the AppImage for Ubuntu: the release binary, the words at the pin, the desktop entry
# and its icon, with GTK 4, libadwaita and what they need bundled by linuxdeploy and its GTK
# plugin, both pinned by checksum. Writes tafs-<version>-x86_64.AppImage at the repo's root.
# Run after `tools/sync.sh` and `cargo build --release -p tafs`. The system it's built on sets
# the floor: its glibc (Ubuntu 24.04's 2.39 in CI), since GTK 4.14 and libadwaita 1.5 come from it.
set -euo pipefail
cd "$(dirname "$0")/.."
version=${1:-dev}
tools=target/appimage-tools
appdir=target/AppDir

fetch() { # url, file, sha256: downloaded once, and refused unless its checksum is the pinned one.
  if [ ! -f "$tools/$2" ] || ! echo "$3  $tools/$2" | sha256sum -c --quiet; then
    curl -sSLf -o "$tools/$2" "$1"
    echo "$3  $tools/$2" | sha256sum -c --quiet
  fi
  chmod +x "$tools/$2"
}
mkdir -p "$tools"
fetch https://github.com/linuxdeploy/linuxdeploy/releases/download/1-alpha-20250213-2/linuxdeploy-x86_64.AppImage \
  linuxdeploy-x86_64.AppImage 4648f278ab3ef31f819e67c30d50f462640e5365a77637d7e6f2ad9fd0b4522a
fetch https://raw.githubusercontent.com/linuxdeploy/linuxdeploy-plugin-gtk/7a3fbc31a9e5075073ff8790f26effbac5f84453/linuxdeploy-plugin-gtk.sh \
  linuxdeploy-plugin-gtk.sh b0f4cbc684a0103a9651f0955b635eaea0096b3a66c0f5a2c2aa337960375171

rm -rf "$appdir"
install -Dm755 target/release/tafs "$appdir/usr/bin/tafs"
# The words: the app reads them from ../share/tafs/strings beside its binary.
mkdir -p "$appdir/usr/share/tafs"
cp -R shared/app/strings "$appdir/usr/share/tafs/strings"
install -Dm644 packaging/dev.kiff.tafs.desktop "$appdir/usr/share/applications/dev.kiff.tafs.desktop"
install -Dm644 crates/tafs-app/assets/mascot.png "$appdir/usr/share/icons/hicolor/192x192/apps/dev.kiff.tafs.png"
# The symbolic icons the screens take from the desktop's theme (a chevron, the halt's hand), so
# a desktop without Adwaita draws them too. Its cursors are left to the desktop.
mkdir -p "$appdir/usr/share/icons/Adwaita"
cp -R /usr/share/icons/Adwaita/index.theme /usr/share/icons/Adwaita/symbolic /usr/share/icons/Adwaita/scalable \
  "$appdir/usr/share/icons/Adwaita/"

# Without FUSE, as CI runners and containers have none.
export APPIMAGE_EXTRACT_AND_RUN=1 DEPLOY_GTK_VERSION=4 LINUXDEPLOY_OUTPUT_VERSION="$version"
PATH="$PWD/$tools:$PATH" "$tools/linuxdeploy-x86_64.AppImage" \
  --appdir "$appdir" \
  --plugin gtk \
  --desktop-file "$appdir/usr/share/applications/dev.kiff.tafs.desktop" \
  --icon-file "$appdir/usr/share/icons/hicolor/192x192/apps/dev.kiff.tafs.png" \
  --output appimage
ls -la tafs-*-x86_64.AppImage
