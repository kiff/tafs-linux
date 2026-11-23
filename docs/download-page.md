# kiff.dev's Linux download page: draft text

The words for tafs for Linux's download page on kiff.dev, in English. The site makes the page;
this file is its text and what each part links to, kept beside the app so the two say the same.
Nothing here is published: the page goes up, and a release is published, on the owner's yes.
Every fact below is the app's own as built today (`docs/app.md` › The AppImage, Signing; the
README's Install; `docs/approval.md`; `docs/account.md`). When one of those changes, this changes
with it.

Placeholders: `{version}` is the latest published release (as `v1.0.1`), `{release}` its page,
`https://github.com/kiff/tafs-linux/releases/tag/{version}`.

---

## tafs for Linux

**Your tafs on your Linux computer.** The same account and the same tafs as on your phone, with
what needs you, your brief, Coach, the Record and Plan & usage.

[Download the AppImage ({version}, x86_64)] → `{release}`'s `tafs-{version}-x86_64.AppImage`

### What it runs on

- Ubuntu 24.04 or newer, or another distribution as new (glibc 2.39 or later), on x86_64.
- One file, the AppImage: nothing to install, and it doesn't need administrator rights.
- To approve on this computer, a FIDO2 security key with a PIN, and the system's `libfido2`
  (on Ubuntu: `sudo apt install libfido2-1`). Without a key, this computer shows everything and
  your phone approves.

### Check it before you run it

Download these three files into one folder, from `{release}`:

- `tafs-{version}-x86_64.AppImage`
- `SHA256SUMS`, the AppImage's SHA-256
- `SHA256SUMS.asc`, its signature by tafs for Linux's release key

And the release key, [`tafs-release-key.asc`] →
`https://github.com/kiff/tafs-linux/blob/main/packaging/tafs-release-key.asc`

**The key's fingerprint:** `73BC 9445 0F18 1366 FE44  3DCB 2845 162C 93DB BCA1`
(OpenPGP ed25519, signing only, expires 2028-11-16, `tafs for Linux (release signing)
<support@kiff.dev>`).

```
gpg --import tafs-release-key.asc
gpg --fingerprint support@kiff.dev        # the fingerprint above
gpg --verify SHA256SUMS.asc SHA256SUMS     # "Good signature"
sha256sum -c SHA256SUMS                    # the AppImage: OK
```

gpg says the key isn't certified with a trusted signature unless you've signed it yourself.
That's expected: what counts is the fingerprint, compared with the one on this page.

### Run it

```
chmod +x tafs-{version}-x86_64.AppImage
./tafs-{version}-x86_64.AppImage
```

Then sign in with KIFF, or add this computer from your phone: open tafs on your phone and scan
the code this computer shows. With a security key plugged in when you add it, this computer can
approve with the key: its touch and your PIN, each time.

### Updates

An AppImage doesn't update itself. tafs checks for a newer release when it opens, and once a day
while it stays open, and tells you in its sidebar, with a link to the release. It never downloads
or runs anything on its own. Download the new one, check it the same way, and use it in the old
one's place.

### Removing it

Sign out first (Account › Sign out): that removes this computer's session from your keyring and
from your account. Then delete the AppImage. tafs also keeps your theme, language and a few
choices in `~/.config/tafs`, and files you opened in `~/.cache/tafs`: delete those folders too.

### More

- Every release, with its notes: `https://github.com/kiff/tafs-linux/releases`
- The source: `https://github.com/kiff/tafs-linux`
- Help: support@kiff.dev
