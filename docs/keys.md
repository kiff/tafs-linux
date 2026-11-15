# The device key

The device key signs with no prompt (KIFF's desktop client contract §2, §4): P-256, ECDSA over
SHA-256, signatures in ASN.1 DER (never raw r‖s), standard base64 (`tafs-core/src/keys.rs`).

- **Made in software.** Linux has no hardware key store every desktop offers, so the key is made
  from the system's randomness, and its private half is kept in the person's keyring with the
  session. The start says `"key_store": "software"`, and the session is **view-only**: KIFF makes a
  software-keyed session `view`. Every approval goes to the phone.
- **One per sign-in.** A sign-in makes its own key; the session keeps it.
- **Its fingerprint** is KIFF's: the first 8 bytes of SHA-256 of its SPKI DER, in hex, in four
  groups.

Tests: KIFF's encoding (SPKI DER, at most 512 bytes), a DER signature its public half verifies,
the key back from the keyring, and the fingerprint.
