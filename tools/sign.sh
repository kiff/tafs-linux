#!/usr/bin/env bash
# Signs a release's SHA256SUMS with tafs for Linux's signing key: writes SHA256SUMS.asc beside it,
# an armored detached OpenPGP signature, then checks it. Whoever downloads the AppImage checks the
# signature with the public key, then the AppImage against SHA256SUMS.
#
# The key comes from the environment, never from a file in the repository:
#   TAFS_SIGNING_KEY         the armored secret key
#   TAFS_SIGNING_PASSPHRASE  its passphrase (may be empty for a key without one)
# With no key, it says so and signs nothing (exit 0): the release goes out unsigned until a key
# exists, and adding one is a value, not a change here.
#
# Usage: tools/sign.sh SHA256SUMS
set -euo pipefail
sums=${1:?usage: tools/sign.sh SHA256SUMS}
[ -f "$sums" ] || { echo "No $sums to sign." >&2; exit 1; }

if [ -z "${TAFS_SIGNING_KEY:-}" ]; then
  echo "No signing key yet: $sums is left unsigned."
  exit 0
fi

# A keyring of its own, removed on the way out, so nothing of the key stays on the machine.
GNUPGHOME=$(mktemp -d)
export GNUPGHOME
trap 'gpgconf --kill gpg-agent >/dev/null 2>&1 || true; rm -rf "$GNUPGHOME"' EXIT
chmod 700 "$GNUPGHOME"

# Imported from stdin: the key never reaches a file or the command line.
printf '%s\n' "$TAFS_SIGNING_KEY" | gpg --batch --quiet --import
fingerprint=$(gpg --batch --with-colons --list-secret-keys | awk -F: '$1 == "fpr" { print $10; exit }')
[ -n "$fingerprint" ] || { echo "The signing key couldn't be read." >&2; exit 1; }

rm -f "$sums.asc"
printf '%s' "${TAFS_SIGNING_PASSPHRASE:-}" |
  gpg --batch --quiet --yes --pinentry-mode loopback --passphrase-fd 0 \
    --local-user "$fingerprint" --armor --detach-sign --output "$sums.asc" "$sums"

gpg --batch --quiet --verify "$sums.asc" "$sums"
echo "$sums signed by $fingerprint."
