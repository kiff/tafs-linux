//! The USB security key's side of an approval, without the key (tafs-macos
//! `Sources/Core/approvals/SecurityKey.swift`, from kiff/tafs `docs/computer-passkeys.md`): what
//! is asked of the key, its PIN, its answers' failures, and a new credential's attestation
//! object. The app will ask the key through libfido2 with exactly these, and nothing else.
//! Switched off with the rest (`passkey::ENABLED`).

use std::ffi::CStr;

/// A CTAP2 option as libfido2 takes it (`fido_opt_t`): left out, false or true.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FidoOption {
    Omit,
    No,
    Yes,
}

/// The key is always asked with its PIN, and the PIN verifies the person: `uv` is left out, as
/// some CTAP2.0 keys refuse it alongside a PIN token. `passkey::refusal` still refuses
/// authenticator data without the UV flag.
pub const UV_OPTION: FidoOption = FidoOption::Omit;

/// Something asked of the key that it can't be asked: refused before it's asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unreadable;

/// One assertion asked of the key: kiff.dev, the hash of the client data this app built, and the
/// one credential this computer approves with. The person present, and verified by the PIN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertionAsk {
    pub client_data_hash: [u8; 32],
    pub credential_id: Vec<u8>,
    pub user_presence: FidoOption,
    pub user_verification: FidoOption,
}

impl AssertionAsk {
    /// Refuses a hash that isn't SHA-256's 32 bytes, and an empty or oversized credential id
    /// (CTAP2 takes at most 1023 bytes).
    pub fn new(client_data_hash: &[u8], credential_id: &[u8]) -> Result<AssertionAsk, Unreadable> {
        let hash: [u8; 32] = client_data_hash.try_into().map_err(|_| Unreadable)?;
        if credential_id.is_empty() || credential_id.len() > 1023 {
            return Err(Unreadable);
        }
        Ok(AssertionAsk {
            client_data_hash: hash,
            credential_id: credential_id.to_vec(),
            user_presence: FidoOption::Yes,
            user_verification: UV_OPTION,
        })
    }
}

/// One new credential asked of the key: kiff.dev ("KIFF"), ES256 only, the person verified by the
/// PIN, kept as a credential id this computer holds (not discoverable).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialAsk {
    pub client_data_hash: [u8; 32],
    pub user_id: Vec<u8>,
    pub user_name: String,
    pub resident_key: FidoOption,
    pub user_verification: FidoOption,
}

impl CredentialAsk {
    /// The relying party's name, as Windows gives it.
    pub const RP_NAME: &'static str = "KIFF";
    /// COSE's ES256, the only algorithm KIFF takes.
    pub const ES256: i32 = -7;

    /// Refuses a hash that isn't 32 bytes, and a user id outside CTAP2's 1 to 64 bytes.
    pub fn new(
        client_data_hash: &[u8],
        user_id: &[u8],
        user_name: &str,
    ) -> Result<CredentialAsk, Unreadable> {
        let hash: [u8; 32] = client_data_hash.try_into().map_err(|_| Unreadable)?;
        if user_id.is_empty() || user_id.len() > 64 {
            return Err(Unreadable);
        }
        Ok(CredentialAsk {
            client_data_hash: hash,
            user_id: user_id.to_vec(),
            user_name: user_name.to_string(),
            resident_key: FidoOption::No,
            user_verification: UV_OPTION,
        })
    }
}

/// How long the key waits for its touch, as Windows' two minutes for Windows Hello.
pub const WAIT_MS: i32 = 120_000;

/// Why the key gave no answer. The PIN sheet says each in its own words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// No security key is plugged in.
    NoKey,
    /// More than one is plugged in: the app asks none of them.
    SeveralKeys,
    /// A key that only speaks U2F: it has no PIN, so it can't verify the person.
    NotFido2,
    /// No PIN was given: the key is asked only with its PIN, each time.
    PinMissing,
    /// A PIN shorter than 4 characters, which no key takes.
    PinTooShort,
    /// A PIN longer than 63 bytes, or with a NUL in it, which no key takes.
    PinTooLong,
    /// The PIN was already handed to the key once, and cleared.
    PinUsed,
    /// The key has no PIN set, so it can't verify the person.
    PinNotSet,
    /// The PIN was wrong; the key says how many tries are left, when it says.
    WrongPin { tries_left: Option<i32> },
    /// No PIN tries are left: the key is blocked until it's reset.
    PinBlocked,
    /// Three wrong PINs in a row: no more until it's unplugged and plugged in again.
    PinPaused,
    /// The key doesn't hold this computer's credential: it's another key.
    NotThisKey,
    /// The key wasn't touched in time.
    TimedOut,
    /// The key refused, or the request was cancelled at the key.
    Denied,
    /// The key was unplugged, or stopped answering.
    Removed,
    /// Anything else, by libfido2's code.
    Failed { code: i32 },
}

/// libfido2's codes this app tells apart (`fido/err.h`, 1.17.0).
pub mod fido_code {
    pub const OK: i32 = 0x00;
    pub const INVALID_CREDENTIAL: i32 = 0x22;
    pub const OPERATION_DENIED: i32 = 0x27;
    pub const KEEPALIVE_CANCEL: i32 = 0x2d;
    pub const NO_CREDENTIALS: i32 = 0x2e;
    pub const USER_ACTION_TIMEOUT: i32 = 0x2f;
    pub const PIN_INVALID: i32 = 0x31;
    pub const PIN_BLOCKED: i32 = 0x32;
    pub const PIN_AUTH_BLOCKED: i32 = 0x34;
    pub const PIN_NOT_SET: i32 = 0x35;
    pub const ACTION_TIMEOUT: i32 = 0x3a;
    pub const TX: i32 = -1;
    pub const RX: i32 = -2;
    pub const INTERNAL: i32 = -9;
}

impl Failure {
    /// The failure for libfido2's code, or none for `FIDO_OK`. A wrong PIN with no tries left
    /// is a blocked key.
    pub fn of(code: i32, tries_left: Option<i32>) -> Option<Failure> {
        use fido_code::*;
        Some(match code {
            OK => return None,
            PIN_INVALID => match tries_left {
                Some(left) if left <= 0 => Failure::PinBlocked,
                _ => Failure::WrongPin { tries_left },
            },
            PIN_BLOCKED => Failure::PinBlocked,
            PIN_AUTH_BLOCKED => Failure::PinPaused,
            PIN_NOT_SET => Failure::PinNotSet,
            NO_CREDENTIALS | INVALID_CREDENTIAL => Failure::NotThisKey,
            ACTION_TIMEOUT | USER_ACTION_TIMEOUT => Failure::TimedOut,
            OPERATION_DENIED | KEEPALIVE_CANCEL => Failure::Denied,
            TX | RX => Failure::Removed,
            other => Failure::Failed { code: other },
        })
    }
}

/// Zeroes bytes in a way the compiler keeps.
fn clear(bytes: &mut [u8]) {
    for b in bytes.iter_mut() {
        // SAFETY: a valid, aligned, exclusive pointer to one byte of `bytes`.
        unsafe { std::ptr::write_volatile(b, 0) };
    }
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
}

/// The security key's PIN for one approval, in one buffer the app owns: made from the bytes
/// typed, which it clears, handed to the key once as a C string, then cleared. It's never kept
/// past that, written anywhere, logged or sent to a server. A PIN that isn't handed over is
/// cleared when it's dropped.
pub struct Pin {
    /// The PIN and its terminating NUL; none once handed over or released.
    buffer: Option<Vec<u8>>,
}

impl std::fmt::Debug for Pin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never the PIN itself.
        f.debug_struct("Pin")
            .field("held", &self.is_held())
            .finish()
    }
}

impl Pin {
    /// CTAP2: at least 4 Unicode code points, at most 63 bytes of UTF-8.
    pub const FEWEST_CHARACTERS: usize = 4;
    pub const MOST_BYTES: usize = 63;

    /// The PIN from its UTF-8 bytes. The bytes given are cleared in place, whether the PIN is
    /// taken or refused.
    pub fn new(bytes: &mut [u8]) -> Result<Pin, Failure> {
        let taken = Self::take(bytes);
        clear(bytes);
        taken
    }

    fn take(bytes: &[u8]) -> Result<Pin, Failure> {
        if bytes.len() > Self::MOST_BYTES || bytes.contains(&0) {
            return Err(Failure::PinTooLong);
        }
        let characters = std::str::from_utf8(bytes)
            .map_err(|_| Failure::PinTooShort)?
            .chars()
            .count();
        if characters < Self::FEWEST_CHARACTERS {
            return Err(Failure::PinTooShort);
        }
        // Its full size at once, so the bytes are never copied by a grown buffer.
        let mut buffer = Vec::with_capacity(bytes.len() + 1);
        buffer.extend_from_slice(bytes);
        buffer.push(0);
        Ok(Pin {
            buffer: Some(buffer),
        })
    }

    /// Whether the PIN can still be handed over.
    pub fn is_held(&self) -> bool {
        self.buffer.is_some()
    }

    /// Hands the PIN to `body` as a C string, once; then clears it, whatever `body` does.
    pub fn use_once<T>(&mut self, body: impl FnOnce(&CStr) -> T) -> Result<T, Failure> {
        let Some(buffer) = self.buffer.as_ref() else {
            return Err(Failure::PinUsed);
        };
        // Clears even if `body` panics.
        struct Release<'a>(&'a mut Pin);
        impl Drop for Release<'_> {
            fn drop(&mut self) {
                self.0.release();
            }
        }
        let c = CStr::from_bytes_with_nul(buffer).map_err(|_| Failure::PinTooLong)?;
        let c: *const CStr = c;
        let release = Release(self);
        // SAFETY: the buffer isn't touched until `release` drops after `body` returns.
        let out = body(unsafe { &*c });
        drop(release);
        Ok(out)
    }

    /// Clears the PIN now, if it's still held.
    pub fn release(&mut self) {
        if let Some(mut held) = self.buffer.take() {
            clear(&mut held);
        }
    }

    #[cfg(test)]
    fn held_bytes(&self) -> Option<Vec<u8>> {
        self.buffer.as_ref().map(|b| b[..b.len() - 1].to_vec())
    }
}

impl Drop for Pin {
    fn drop(&mut self) {
        self.release();
    }
}

/// A CBOR head: the major type and a length, in the shortest form.
pub fn cbor_head(major: u8, count: usize) -> Vec<u8> {
    let t = major << 5;
    match count {
        0..=23 => vec![t | count as u8],
        24..=0xff => vec![t | 24, count as u8],
        0x100..=0xffff => vec![t | 25, (count >> 8) as u8, count as u8],
        _ => {
            let c = count as u32;
            let mut v = vec![t | 26];
            v.extend_from_slice(&c.to_be_bytes());
            v
        }
    }
}

fn cbor_text(s: &str) -> Vec<u8> {
    let mut v = cbor_head(3, s.len());
    v.extend_from_slice(s.as_bytes());
    v
}

/// A new credential's attestation object, as WebAuthn writes it: a CBOR map of `fmt`, `attStmt`
/// and `authData`, in CTAP2's canonical order. libfido2 gives the three apart.
pub fn attestation_object(format: &str, statement: &[u8], authenticator_data: &[u8]) -> Vec<u8> {
    let mut out = vec![0xa3];
    out.extend(cbor_text("fmt"));
    out.extend(cbor_text(format));
    out.extend(cbor_text("attStmt"));
    out.extend_from_slice(statement);
    out.extend(cbor_text("authData"));
    out.extend(cbor_head(2, authenticator_data.len()));
    out.extend_from_slice(authenticator_data);
    out
}

/// What the PIN sheet says for each of the key's answers (tafs-macos `SecurityKeyWords.swift`), in
/// the shared words (`approval.security_key.*`). Every failure says nothing was approved; where
/// nothing was refused, the request is kept.
pub mod words {
    use super::Failure;
    use crate::words::{WordBook, WordsError};

    /// The line for why the key gave no approval.
    pub fn line(w: &WordBook, failure: Failure) -> Result<String, WordsError> {
        let key = match failure {
            Failure::NoKey => "approval.security_key.no_key",
            Failure::SeveralKeys => "approval.security_key.several_keys",
            Failure::NotFido2 => "approval.security_key.not_fido2",
            Failure::PinTooShort => "approval.security_key.pin_too_short",
            Failure::PinTooLong => "approval.security_key.pin_too_long",
            Failure::PinNotSet => "approval.security_key.no_pin_set",
            Failure::WrongPin {
                tries_left: Some(left),
            } if left > 0 => {
                return w.count("approval.security_key.wrong_pin", left as i64, &[]);
            }
            Failure::WrongPin { .. } => "approval.security_key.wrong_pin_no_count",
            Failure::PinBlocked => "approval.security_key.locked",
            Failure::PinPaused => "approval.security_key.pin_paused",
            Failure::NotThisKey => "approval.security_key.not_this_key",
            Failure::TimedOut => "approval.security_key.timed_out",
            Failure::Denied => "approval.security_key.not_allowed",
            Failure::Removed => "approval.security_key.unplugged",
            Failure::PinMissing | Failure::PinUsed | Failure::Failed { .. } => {
                "approval.security_key.failed"
            }
        };
        w.line(key)
    }

    /// Anything else that stopped the approval: an answer KIFF would refuse (no user
    /// verification among them) is the key failing to sign, never the key's own decline.
    pub fn failed(w: &WordBook) -> Result<String, WordsError> {
        w.line("approval.security_key.failed")
    }

    /// The tries left before the key locks, read before the PIN is asked, when the key says.
    pub fn tries_left(w: &WordBook, count: i32) -> Result<String, WordsError> {
        w.count("approval.security_key.tries_left", count as i64, &[])
    }

    /// Whether the sheet stays open to try again: anything but a locked key.
    pub fn may_try_again(failure: Option<Failure>) -> bool {
        failure != Some(Failure::PinBlocked)
    }

    /// The sheet's own words: its title, what it asks, the field, the touch, and its buttons.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SheetLines {
        pub title: String,
        pub enter_pin: String,
        pub pin: String,
        pub touch: String,
        pub cancel: String,
        pub approve: String,
    }

    pub fn sheet(w: &WordBook) -> Result<SheetLines, WordsError> {
        Ok(SheetLines {
            title: w.line("approval.security_key.title")?,
            enter_pin: w.line("approval.security_key.enter_pin")?,
            pin: w.line("approval.security_key.pin")?,
            touch: w.line("approval.security_key.touch")?,
            cancel: w.line("common.cancel")?,
            approve: w.line("approval.approve")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin(text: &str) -> Pin {
        let mut bytes = text.as_bytes().to_vec();
        Pin::new(&mut bytes).unwrap()
    }

    fn refused(text: &str, expected: Failure) {
        let mut bytes = text.as_bytes().to_vec();
        assert_eq!(
            Pin::new(&mut bytes).unwrap_err(),
            expected,
            "{} bytes",
            text.len()
        );
        assert!(
            bytes.iter().all(|b| *b == 0),
            "the refused PIN's bytes are cleared"
        );
    }

    #[test]
    fn libfido2s_codes_are_the_keys_failures() {
        assert_eq!(Failure::of(0x00, None), None);
        assert_eq!(
            Failure::of(0x31, Some(5)),
            Some(Failure::WrongPin {
                tries_left: Some(5)
            })
        );
        assert_eq!(
            Failure::of(0x31, None),
            Some(Failure::WrongPin { tries_left: None })
        );
        assert_eq!(Failure::of(0x31, Some(0)), Some(Failure::PinBlocked));
        assert_eq!(Failure::of(0x32, None), Some(Failure::PinBlocked));
        assert_eq!(Failure::of(0x34, None), Some(Failure::PinPaused));
        assert_eq!(Failure::of(0x35, None), Some(Failure::PinNotSet));
        assert_eq!(Failure::of(0x2e, None), Some(Failure::NotThisKey));
        assert_eq!(Failure::of(0x22, None), Some(Failure::NotThisKey));
        assert_eq!(Failure::of(0x2f, None), Some(Failure::TimedOut));
        assert_eq!(Failure::of(0x3a, None), Some(Failure::TimedOut));
        assert_eq!(Failure::of(0x27, None), Some(Failure::Denied));
        assert_eq!(Failure::of(0x2d, None), Some(Failure::Denied));
        assert_eq!(Failure::of(-1, None), Some(Failure::Removed));
        assert_eq!(Failure::of(-2, None), Some(Failure::Removed));
        assert_eq!(
            Failure::of(0x2b, None),
            Some(Failure::Failed { code: 0x2b })
        );
    }

    #[test]
    fn a_pin_is_taken_cleared_and_handed_over_once() {
        let mut bytes = b"1234".to_vec();
        let mut p = Pin::new(&mut bytes).unwrap();
        assert_eq!(bytes, [0, 0, 0, 0], "the bytes typed are cleared");
        assert_eq!(p.held_bytes().unwrap(), b"1234");
        let handed = p.use_once(|c| c.to_str().unwrap().to_string()).unwrap();
        assert_eq!(handed, "1234");
        assert!(!p.is_held(), "released once handed over");
        assert_eq!(p.use_once(|_| 0).unwrap_err(), Failure::PinUsed);
        assert!(!format!("{p:?}").contains("1234"));
    }

    #[test]
    fn a_pin_is_released_when_its_use_panics_and_when_let_go() {
        let mut p = pin("246810");
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = p.use_once(|_| panic!("broke"));
        }));
        assert!(r.is_err());
        assert!(!p.is_held(), "released after a panic");
        let mut unused = pin("1357");
        unused.release();
        assert!(!unused.is_held());
    }

    #[test]
    fn a_pin_no_key_takes_is_refused_and_cleared() {
        refused("123", Failure::PinTooShort);
        refused("", Failure::PinTooShort);
        // Three characters of 4 bytes each: 12 bytes, still 3 characters.
        refused("\u{1F511}\u{1F511}\u{1F511}", Failure::PinTooShort);
        refused(&"7".repeat(64), Failure::PinTooLong);
        refused("12\u{0}34", Failure::PinTooLong);
        let mut bad = vec![0xff, 0xfe, 0xfd, 0xfc];
        assert_eq!(Pin::new(&mut bad).unwrap_err(), Failure::PinTooShort);
        assert_eq!(bad, [0, 0, 0, 0]);
        // Four characters of 2 bytes, and 63 bytes: taken.
        assert_eq!(pin("éééé").held_bytes().unwrap().len(), 8);
        assert_eq!(pin(&"7".repeat(63)).held_bytes().unwrap().len(), 63);
    }

    #[test]
    fn whats_asked_of_the_key_is_refused_before_its_asked() {
        let hash = [1u8; 32];
        let ask = AssertionAsk::new(&hash, &[7]).unwrap();
        assert_eq!(ask.client_data_hash, hash);
        for (h, id) in [
            (&[1u8; 31][..], &[7u8][..]),
            (&hash[..], &[][..]),
            (&hash[..], &[7u8; 1024][..]),
        ] {
            assert_eq!(AssertionAsk::new(h, id), Err(Unreadable));
        }
        CredentialAsk::new(&hash, &[2; 64], "rae@example.com").unwrap();
        for id in [&[][..], &[2u8; 65][..]] {
            assert_eq!(
                CredentialAsk::new(&hash, id, "rae@example.com"),
                Err(Unreadable)
            );
        }
        assert_eq!(CredentialAsk::ES256, -7);
        assert_eq!(CredentialAsk::RP_NAME, "KIFF");
        assert_eq!(WAIT_MS, 120_000);
    }

    #[test]
    fn with_the_pin_uv_is_left_out_presence_asked_and_not_discoverable() {
        let a = AssertionAsk::new(&[1; 32], &[7]).unwrap();
        assert_eq!(a.user_verification, FidoOption::Omit);
        assert_eq!(a.user_presence, FidoOption::Yes);
        let c = CredentialAsk::new(&[1; 32], &[2], "rae@example.com").unwrap();
        assert_eq!(c.user_verification, FidoOption::Omit);
        assert_eq!(c.resident_key, FidoOption::No);
        // Without the UV flag, the answer is still refused.
        use sha2::Digest;
        let mut auth = sha2::Sha256::digest(b"kiff.dev").to_vec();
        auth.extend_from_slice(&[0x01, 0, 0, 0, 1]);
        assert_eq!(
            crate::passkey::refusal(&auth, b"c", b"c"),
            Some(crate::passkey::Refusal::NotVerified)
        );
    }

    #[test]
    fn the_attestation_object_is_in_ctap2s_canonical_order() {
        let auth = [0xabu8; 37];
        let object = attestation_object("none", &[0xa0], &auth);
        let mut want = vec![0xa3, 0x63];
        want.extend_from_slice(b"fmt");
        want.push(0x64);
        want.extend_from_slice(b"none");
        want.push(0x67);
        want.extend_from_slice(b"attStmt");
        want.push(0xa0);
        want.push(0x68);
        want.extend_from_slice(b"authData");
        want.extend_from_slice(&[0x58, 37]);
        want.extend_from_slice(&auth);
        assert_eq!(object, want);
        assert_eq!(cbor_head(2, 23), [0x57]);
        assert_eq!(cbor_head(2, 300), [0x59, 0x01, 0x2c]);
        assert_eq!(cbor_head(2, 70_000), [0x5a, 0x00, 0x01, 0x11, 0x70]);
    }

    fn book(lang: &str) -> crate::words::WordBook {
        crate::words::WordBook::load(&crate::words::WordBook::default_root(), lang).unwrap()
    }

    /// macOS 12's `SecurityKeyWordsTests`.
    #[test]
    fn each_failure_says_its_own_shared_line() {
        let w = book("en");
        let cases = [
            (Failure::NoKey, "approval.security_key.no_key"),
            (Failure::SeveralKeys, "approval.security_key.several_keys"),
            (Failure::NotFido2, "approval.security_key.not_fido2"),
            (Failure::PinTooShort, "approval.security_key.pin_too_short"),
            (Failure::PinTooLong, "approval.security_key.pin_too_long"),
            (Failure::PinNotSet, "approval.security_key.no_pin_set"),
            (
                Failure::WrongPin { tries_left: None },
                "approval.security_key.wrong_pin_no_count",
            ),
            (Failure::PinBlocked, "approval.security_key.locked"),
            (Failure::PinPaused, "approval.security_key.pin_paused"),
            (Failure::NotThisKey, "approval.security_key.not_this_key"),
            (Failure::TimedOut, "approval.security_key.timed_out"),
            (Failure::Denied, "approval.security_key.not_allowed"),
            (Failure::Removed, "approval.security_key.unplugged"),
            (Failure::Failed { code: -9 }, "approval.security_key.failed"),
            (Failure::PinMissing, "approval.security_key.failed"),
            (Failure::PinUsed, "approval.security_key.failed"),
        ];
        for (failure, key) in cases {
            assert_eq!(
                words::line(&w, failure).unwrap(),
                w.line(key).unwrap(),
                "{key}"
            );
        }
        assert_eq!(
            words::failed(&w).unwrap(),
            w.line("approval.security_key.failed").unwrap()
        );
        assert_ne!(
            words::failed(&w).unwrap(),
            w.line("approval.security_key.not_allowed").unwrap()
        );
    }

    #[test]
    fn a_wrong_pin_says_the_tries_left_in_each_languages_plural() {
        let w = book("en");
        let wrong = |n| {
            words::line(
                &w,
                Failure::WrongPin {
                    tries_left: Some(n),
                },
            )
            .unwrap()
        };
        assert_eq!(wrong(1), "Wrong PIN. 1 try left before the key locks.");
        assert_eq!(wrong(5), "Wrong PIN. 5 tries left before the key locks.");
        assert_eq!(
            wrong(0),
            w.line("approval.security_key.wrong_pin_no_count").unwrap()
        );
        let pl = book("pl");
        assert_eq!(
            words::tries_left(&pl, 3).unwrap(),
            "Zostały 3 próby, zanim klucz się zablokuje."
        );
        assert_eq!(
            words::tries_left(&pl, 5).unwrap(),
            "Zostało 5 prób, zanim klucz się zablokuje."
        );
        for lang in crate::words::SHIPPED {
            let w = book(lang);
            words::sheet(&w).unwrap();
            for n in [1, 2, 5, 8] {
                words::tries_left(&w, n).unwrap();
                words::line(
                    &w,
                    Failure::WrongPin {
                        tries_left: Some(n),
                    },
                )
                .unwrap();
            }
        }
    }

    #[test]
    fn a_locked_key_closes_the_sheet() {
        assert!(!words::may_try_again(Some(Failure::PinBlocked)));
        assert!(words::may_try_again(Some(Failure::WrongPin {
            tries_left: Some(2)
        })));
        assert!(words::may_try_again(Some(Failure::NoKey)));
        assert!(words::may_try_again(None));
    }
}
