#!/usr/bin/env bash
# Checks a release's SHA256SUMS.asc with tafs for Linux's release key, and only that key: the key
# in packaging/tafs-release-key.asc must be the one whose fingerprint is pinned here, and the
# signature must be a good one made by it. Any other key, or a bad signature, fails.
#
# With no SHA256SUMS.asc, it says the release is unsigned: that fails with --require, and is only
# said without it.
#
# Usage: tools/verify-signature.sh DIR [--require]
#   DIR holds the release's SHA256SUMS (and SHA256SUMS.asc).
set -euo pipefail
dir=${1:?usage: tools/verify-signature.sh DIR [--require]}
require=${2:-}
root=$(cd "$(dirname "$0")/.." && pwd)
# The release key's fingerprint, as the README and docs/app.md give it.
FINGERPRINT=73BC94450F181366FE443DCB2845162C93DBBCA1

fail() {
  echo "$1" >&2
  exit 1
}

[ -f "$dir/SHA256SUMS" ] || fail "No SHA256SUMS in $dir."
if [ ! -f "$dir/SHA256SUMS.asc" ]; then
  [ "$require" = --require ] && fail "No SHA256SUMS.asc: this release is unsigned."
  echo "No SHA256SUMS.asc: this release is unsigned."
  exit 0
fi

# A keyring of its own, holding only the release key, removed on the way out.
GNUPGHOME=$(mktemp -d)
export GNUPGHOME
trap 'gpgconf --kill gpg-agent >/dev/null 2>&1 || true; rm -rf "$GNUPGHOME"' EXIT
chmod 700 "$GNUPGHOME"
gpg --batch --quiet --import "$root/packaging/tafs-release-key.asc"
key=$(gpg --batch --with-colons --list-keys | awk -F: '$1 == "fpr" { print $10; exit }')
[ "$key" = "$FINGERPRINT" ] ||
  fail "packaging/tafs-release-key.asc isn't the release key: its fingerprint is ${key:-none}."

# gpg's machine-readable status: VALIDSIG names the key that made a good signature.
status=$(gpg --batch --status-fd 1 --verify "$dir/SHA256SUMS.asc" "$dir/SHA256SUMS" 2>/dev/null || true)
grep -q "^\[GNUPG:\] VALIDSIG $FINGERPRINT " <<<"$status" ||
  fail "SHA256SUMS.asc isn't a good signature by the release key $FINGERPRINT."
echo "SHA256SUMS is signed by the release key $FINGERPRINT."
