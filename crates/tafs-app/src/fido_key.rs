//! This computer's USB security key, over libfido2 (tafs-macos `Sources/App/approvals/FidoKey.swift`):
//! CTAP2 over USB HID. It asks the one key plugged in for exactly a `security_key::AssertionAsk` or
//! a `security_key::CredentialAsk`, with the PIN, and returns the key's raw answer; core checks
//! that answer whatever libfido2 says (`passkey::refusal`). Only `fido_dev_make_cred` and
//! `fido_dev_get_assert` are asked of the key, and its retry count read: it's never reset, and its
//! PIN is never set or changed here.
//!
//! macOS 12 builds libfido2 from pinned source; Linux loads the system's `libfido2.so.1`
//! (Debian and Ubuntu's `libfido2-1`, Fedora's `libfido2`) when a key is first asked, so the app
//! builds and runs without it, and a computer without it says so. Switched off with the rest
//! (`passkey::ENABLED`): nothing calls this yet.
#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_uchar, c_void, CStr, CString};
use std::sync::{Mutex, OnceLock};

use tafs_core::passkey::RP_ID;
use tafs_core::security_key::{
    fido_code, AssertionAsk, CredentialAsk, Failure, FidoOption, Pin, Unreadable, WAIT_MS,
};

type Dev = *mut c_void;
type Info = *mut c_void;
type Assert = *mut c_void;
type Cred = *mut c_void;

/// `FIDO_DISABLE_U2F_FALLBACK`: a U2F key can't verify the person.
const DISABLE_U2F_FALLBACK: c_int = 0x02;
/// The most keys looked for: one is asked, so more than one is refused.
const MOST: usize = 8;

/// libfido2's calls this app makes, and no others.
struct Fido {
    _lib: libloading::Library,
    init: unsafe extern "C" fn(c_int),
    set_log_handler: unsafe extern "C" fn(Option<unsafe extern "C" fn(*const c_char)>),
    info_new: unsafe extern "C" fn(usize) -> Info,
    info_free: unsafe extern "C" fn(*mut Info, usize),
    info_manifest: unsafe extern "C" fn(Info, usize, *mut usize) -> c_int,
    info_ptr: unsafe extern "C" fn(Info, usize) -> Info,
    info_path: unsafe extern "C" fn(Info) -> *const c_char,
    dev_new: unsafe extern "C" fn() -> Dev,
    dev_free: unsafe extern "C" fn(*mut Dev),
    dev_open: unsafe extern "C" fn(Dev, *const c_char) -> c_int,
    dev_close: unsafe extern "C" fn(Dev) -> c_int,
    dev_is_fido2: unsafe extern "C" fn(Dev) -> bool,
    dev_has_pin: unsafe extern "C" fn(Dev) -> bool,
    dev_set_timeout: unsafe extern "C" fn(Dev, c_int) -> c_int,
    dev_cancel: unsafe extern "C" fn(Dev) -> c_int,
    dev_get_retry_count: unsafe extern "C" fn(Dev, *mut c_int) -> c_int,
    dev_get_assert: unsafe extern "C" fn(Dev, Assert, *const c_char) -> c_int,
    dev_make_cred: unsafe extern "C" fn(Dev, Cred, *const c_char) -> c_int,
    assert_new: unsafe extern "C" fn() -> Assert,
    assert_free: unsafe extern "C" fn(*mut Assert),
    assert_set_rp: unsafe extern "C" fn(Assert, *const c_char) -> c_int,
    assert_set_clientdata_hash: unsafe extern "C" fn(Assert, *const c_uchar, usize) -> c_int,
    assert_allow_cred: unsafe extern "C" fn(Assert, *const c_uchar, usize) -> c_int,
    assert_set_up: unsafe extern "C" fn(Assert, c_int) -> c_int,
    assert_set_uv: unsafe extern "C" fn(Assert, c_int) -> c_int,
    assert_count: unsafe extern "C" fn(Assert) -> usize,
    assert_id_ptr: unsafe extern "C" fn(Assert, usize) -> *const c_uchar,
    assert_id_len: unsafe extern "C" fn(Assert, usize) -> usize,
    assert_authdata_raw_ptr: unsafe extern "C" fn(Assert, usize) -> *const c_uchar,
    assert_authdata_raw_len: unsafe extern "C" fn(Assert, usize) -> usize,
    assert_sig_ptr: unsafe extern "C" fn(Assert, usize) -> *const c_uchar,
    assert_sig_len: unsafe extern "C" fn(Assert, usize) -> usize,
    cred_new: unsafe extern "C" fn() -> Cred,
    cred_free: unsafe extern "C" fn(*mut Cred),
    cred_set_type: unsafe extern "C" fn(Cred, c_int) -> c_int,
    cred_set_clientdata_hash: unsafe extern "C" fn(Cred, *const c_uchar, usize) -> c_int,
    cred_set_rp: unsafe extern "C" fn(Cred, *const c_char, *const c_char) -> c_int,
    cred_set_user: unsafe extern "C" fn(
        Cred,
        *const c_uchar,
        usize,
        *const c_char,
        *const c_char,
        *const c_char,
    ) -> c_int,
    cred_set_rk: unsafe extern "C" fn(Cred, c_int) -> c_int,
    cred_set_uv: unsafe extern "C" fn(Cred, c_int) -> c_int,
    cred_authdata_raw_ptr: unsafe extern "C" fn(Cred) -> *const c_uchar,
    cred_authdata_raw_len: unsafe extern "C" fn(Cred) -> usize,
    cred_attstmt_ptr: unsafe extern "C" fn(Cred) -> *const c_uchar,
    cred_attstmt_len: unsafe extern "C" fn(Cred) -> usize,
    cred_fmt: unsafe extern "C" fn(Cred) -> *const c_char,
}

/// Why a key can't be asked on this computer before any is looked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Missing {
    /// No `libfido2.so.1` (the `libfido2-1` package).
    Library,
    /// A libfido2 older than 1.14, without the raw authenticator data's getters.
    TooOld,
}

impl Fido {
    fn load() -> Result<Fido, Missing> {
        // SAFETY: libfido2's own constructor only; every symbol is read with its C signature
        // from `fido.h` (1.14).
        unsafe {
            let lib = libloading::Library::new("libfido2.so.1").map_err(|_| Missing::Library)?;
            macro_rules! f {
                ($name:literal) => {
                    *lib.get(concat!($name, "\0").as_bytes())
                        .map_err(|_| Missing::TooOld)?
                };
            }
            let fido = Fido {
                init: f!("fido_init"),
                set_log_handler: f!("fido_set_log_handler"),
                info_new: f!("fido_dev_info_new"),
                info_free: f!("fido_dev_info_free"),
                info_manifest: f!("fido_dev_info_manifest"),
                info_ptr: f!("fido_dev_info_ptr"),
                info_path: f!("fido_dev_info_path"),
                dev_new: f!("fido_dev_new"),
                dev_free: f!("fido_dev_free"),
                dev_open: f!("fido_dev_open"),
                dev_close: f!("fido_dev_close"),
                dev_is_fido2: f!("fido_dev_is_fido2"),
                dev_has_pin: f!("fido_dev_has_pin"),
                dev_set_timeout: f!("fido_dev_set_timeout"),
                dev_cancel: f!("fido_dev_cancel"),
                dev_get_retry_count: f!("fido_dev_get_retry_count"),
                dev_get_assert: f!("fido_dev_get_assert"),
                dev_make_cred: f!("fido_dev_make_cred"),
                assert_new: f!("fido_assert_new"),
                assert_free: f!("fido_assert_free"),
                assert_set_rp: f!("fido_assert_set_rp"),
                assert_set_clientdata_hash: f!("fido_assert_set_clientdata_hash"),
                assert_allow_cred: f!("fido_assert_allow_cred"),
                assert_set_up: f!("fido_assert_set_up"),
                assert_set_uv: f!("fido_assert_set_uv"),
                assert_count: f!("fido_assert_count"),
                assert_id_ptr: f!("fido_assert_id_ptr"),
                assert_id_len: f!("fido_assert_id_len"),
                assert_authdata_raw_ptr: f!("fido_assert_authdata_raw_ptr"),
                assert_authdata_raw_len: f!("fido_assert_authdata_raw_len"),
                assert_sig_ptr: f!("fido_assert_sig_ptr"),
                assert_sig_len: f!("fido_assert_sig_len"),
                cred_new: f!("fido_cred_new"),
                cred_free: f!("fido_cred_free"),
                cred_set_type: f!("fido_cred_set_type"),
                cred_set_clientdata_hash: f!("fido_cred_set_clientdata_hash"),
                cred_set_rp: f!("fido_cred_set_rp"),
                cred_set_user: f!("fido_cred_set_user"),
                cred_set_rk: f!("fido_cred_set_rk"),
                cred_set_uv: f!("fido_cred_set_uv"),
                cred_authdata_raw_ptr: f!("fido_cred_authdata_raw_ptr"),
                cred_authdata_raw_len: f!("fido_cred_authdata_raw_len"),
                cred_attstmt_ptr: f!("fido_cred_attstmt_ptr"),
                cred_attstmt_len: f!("fido_cred_attstmt_len"),
                cred_fmt: f!("fido_cred_fmt"),
                _lib: lib,
            };
            // No U2F fallback; and whatever `FIDO_DEBUG` says, its debug output goes nowhere (it
            // would print the frames sent to the key, the PIN protocol's included). The
            // environment is left as it is: changing it while other threads read it isn't safe.
            // After `fido_init`, which sets its own handler when `FIDO_DEBUG` is set.
            (fido.init)(DISABLE_U2F_FALLBACK);
            (fido.set_log_handler)(Some(Self::quiet));
            Ok(fido)
        }
    }

    /// libfido2's log handler: says nothing.
    unsafe extern "C" fn quiet(_: *const c_char) {}

    /// libfido2, loaded and started once.
    fn get() -> Result<&'static Fido, Missing> {
        static FIDO: OnceLock<Result<Fido, Missing>> = OnceLock::new();
        FIDO.get_or_init(Fido::load).as_ref().map_err(|m| *m)
    }
}

/// What asking the key came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyError {
    /// libfido2 isn't usable here.
    Missing(Missing),
    /// The key's own failure.
    Key(Failure),
    /// What the key answered can't be read.
    Unreadable,
    /// Cancelled while the key was asked.
    Cancelled,
}

impl From<Failure> for KeyError {
    fn from(f: Failure) -> Self {
        KeyError::Key(f)
    }
}

impl From<Unreadable> for KeyError {
    fn from(_: Unreadable) -> Self {
        KeyError::Unreadable
    }
}

/// The key's raw answer to one assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyAssertion {
    pub authenticator_data: Vec<u8>,
    pub signature: Vec<u8>,
    /// The credential the key used, when it says.
    pub credential_id: Option<Vec<u8>>,
}

/// The key's raw answer to a new credential.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyCredential {
    pub authenticator_data: Vec<u8>,
    pub attestation_object: Vec<u8>,
}

fn opt(o: FidoOption) -> c_int {
    match o {
        FidoOption::Omit => 0,
        FidoOption::No => 1,
        FidoOption::Yes => 2,
    }
}

fn bytes(p: *const c_uchar, n: usize) -> Vec<u8> {
    if p.is_null() || n == 0 {
        return vec![];
    }
    // SAFETY: libfido2 gives `n` bytes at `p`, valid while the object they come from lives.
    unsafe { std::slice::from_raw_parts(p, n).to_vec() }
}

fn checked(codes: &[c_int]) -> Result<(), KeyError> {
    match codes.iter().find(|c| **c != fido_code::OK) {
        Some(bad) => Err(KeyError::Key(Failure::Failed { code: *bad })),
        None => Ok(()),
    }
}

/// How many FIDO keys are plugged in, without opening any: the PIN sheet says to plug one in, or
/// to leave only one.
pub fn plugged() -> Result<usize, Missing> {
    let f = Fido::get()?;
    // SAFETY: a list of `MOST` made and freed here; `found` is written by libfido2.
    unsafe {
        let mut list = (f.info_new)(MOST);
        if list.is_null() {
            return Ok(0);
        }
        let mut found = 0usize;
        let code = (f.info_manifest)(list, MOST, &mut found);
        (f.info_free)(&mut list, MOST);
        Ok(if code == fido_code::OK { found } else { 0 })
    }
}

/// One request to the key, so a cancel reaches it from another thread. As macOS 12's `run`
/// makes a new `OpenKey` for each request, the PIN sheet makes a new one for each ask: once
/// cancelled it stays cancelled, so a cancel that arrives before the key is opened still stops
/// that request.
#[derive(Default)]
pub struct OpenKey {
    state: Mutex<(usize, bool)>,
}

impl OpenKey {
    /// Cancels the request at the key, if one runs; one about to start won't.
    pub fn cancel(&self) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        s.1 = true;
        if s.0 != 0 {
            if let Ok(f) = Fido::get() {
                // SAFETY: the device is open until `run` clears it under this lock.
                unsafe { (f.dev_cancel)(s.0 as Dev) };
            }
        }
    }

    fn run<T>(&self, body: impl FnOnce(&Fido, Dev) -> Result<T, KeyError>) -> Result<T, KeyError> {
        let f = Fido::get().map_err(KeyError::Missing)?;
        let device = open_only_key(f)?;
        {
            let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
            if s.1 {
                drop(s);
                close(f, device);
                return Err(KeyError::Cancelled);
            }
            s.0 = device as usize;
        }
        let out = body(f, device);
        let cancelled = {
            let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
            s.0 = 0;
            s.1
        };
        close(f, device);
        match out {
            Err(_) if cancelled => Err(KeyError::Cancelled),
            other => other,
        }
    }

    /// One assertion, over the client data hash this app built, with this computer's credential.
    /// The PIN is handed to the key once, and cleared whatever happens.
    pub fn get_assertion(
        &self,
        ask: &AssertionAsk,
        pin: Option<&mut Pin>,
    ) -> Result<KeyAssertion, KeyError> {
        let Some(pin) = pin.filter(|p| p.is_held()) else {
            return Err(Failure::PinMissing.into());
        };
        let out = self.run(|f, device| {
            // SAFETY: the assertion is made, used and freed here; its setters copy what they take.
            unsafe {
                let mut a = (f.assert_new)();
                if a.is_null() {
                    return Err(Failure::Failed {
                        code: fido_code::INTERNAL,
                    }
                    .into());
                }
                let rp = CString::new(RP_ID).expect("no NUL");
                let result = (|| {
                    checked(&[
                        (f.assert_set_rp)(a, rp.as_ptr()),
                        (f.assert_set_clientdata_hash)(a, ask.client_data_hash.as_ptr(), 32),
                        (f.assert_allow_cred)(
                            a,
                            ask.credential_id.as_ptr(),
                            ask.credential_id.len(),
                        ),
                        (f.assert_set_up)(a, opt(ask.user_presence)),
                        (f.assert_set_uv)(a, opt(ask.user_verification)),
                    ])?;
                    let code = pin.use_once(|c| (f.dev_get_assert)(device, a, c.as_ptr()))?;
                    check(f, device, code)?;
                    if (f.assert_count)(a) != 1 {
                        return Err(KeyError::Unreadable);
                    }
                    let used = bytes((f.assert_id_ptr)(a, 0), (f.assert_id_len)(a, 0));
                    Ok(KeyAssertion {
                        authenticator_data: bytes(
                            (f.assert_authdata_raw_ptr)(a, 0),
                            (f.assert_authdata_raw_len)(a, 0),
                        ),
                        signature: bytes((f.assert_sig_ptr)(a, 0), (f.assert_sig_len)(a, 0)),
                        credential_id: (!used.is_empty()).then_some(used),
                    })
                })();
                (f.assert_free)(&mut a);
                result
            }
        });
        pin.release();
        out
    }

    /// One new credential for kiff.dev, ES256, the person verified by the PIN. The attestation
    /// object is written from libfido2's three parts.
    pub fn make_credential(
        &self,
        ask: &CredentialAsk,
        pin: Option<&mut Pin>,
    ) -> Result<KeyCredential, KeyError> {
        let Some(pin) = pin.filter(|p| p.is_held()) else {
            return Err(Failure::PinMissing.into());
        };
        let name = CString::new(ask.user_name.as_str()).map_err(|_| KeyError::Unreadable);
        let out = name.and_then(|name| {
            self.run(|f, device| {
                // SAFETY: the credential is made, used and freed here; its setters copy what they take.
                unsafe {
                    let mut c = (f.cred_new)();
                    if c.is_null() {
                        return Err(Failure::Failed {
                            code: fido_code::INTERNAL,
                        }
                        .into());
                    }
                    let rp = CString::new(RP_ID).expect("no NUL");
                    let rp_name = CString::new(CredentialAsk::RP_NAME).expect("no NUL");
                    let result = (|| {
                        checked(&[
                            (f.cred_set_type)(c, CredentialAsk::ES256),
                            (f.cred_set_clientdata_hash)(c, ask.client_data_hash.as_ptr(), 32),
                            (f.cred_set_rp)(c, rp.as_ptr(), rp_name.as_ptr()),
                            (f.cred_set_user)(
                                c,
                                ask.user_id.as_ptr(),
                                ask.user_id.len(),
                                name.as_ptr(),
                                name.as_ptr(),
                                std::ptr::null(),
                            ),
                            (f.cred_set_rk)(c, opt(ask.resident_key)),
                            (f.cred_set_uv)(c, opt(ask.user_verification)),
                        ])?;
                        let code = pin.use_once(|p| (f.dev_make_cred)(device, c, p.as_ptr()))?;
                        check(f, device, code)?;
                        let auth =
                            bytes((f.cred_authdata_raw_ptr)(c), (f.cred_authdata_raw_len)(c));
                        let statement = bytes((f.cred_attstmt_ptr)(c), (f.cred_attstmt_len)(c));
                        let fmt = (f.cred_fmt)(c);
                        if fmt.is_null() || auth.is_empty() || statement.is_empty() {
                            return Err(KeyError::Unreadable);
                        }
                        let format = CStr::from_ptr(fmt).to_string_lossy().into_owned();
                        Ok(KeyCredential {
                            attestation_object: tafs_core::security_key::attestation_object(
                                &format, &statement, &auth,
                            ),
                            authenticator_data: auth,
                        })
                    })();
                    (f.cred_free)(&mut c);
                    result
                }
            })
        });
        pin.release();
        out
    }

    /// The PIN tries the key has left, for the PIN sheet; a key with none left is blocked.
    pub fn tries_left(&self) -> Result<i32, KeyError> {
        self.run(|f, device| {
            let mut left: c_int = 0;
            // SAFETY: an open device, and `left` written by libfido2.
            let code = unsafe { (f.dev_get_retry_count)(device, &mut left) };
            if let Some(failure) = Failure::of(code, None) {
                return Err(failure.into());
            }
            if left <= 0 {
                return Err(Failure::PinBlocked.into());
            }
            Ok(left)
        })
    }
}

/// The key's failure for `code`; a wrong PIN reads the tries left, so the sheet can say.
fn check(f: &Fido, device: Dev, code: c_int) -> Result<(), KeyError> {
    if code == fido_code::OK {
        return Ok(());
    }
    let mut left: c_int = 0;
    // SAFETY: an open device, and `left` written by libfido2.
    let tries = (code == fido_code::PIN_INVALID
        && unsafe { (f.dev_get_retry_count)(device, &mut left) } == fido_code::OK)
        .then_some(left);
    Err(Failure::of(code, tries)
        .unwrap_or(Failure::Failed { code })
        .into())
}

/// Opens the one FIDO2 key plugged in, with a PIN set: none, several, a U2F-only key or a key
/// without a PIN is refused before anything is asked of it.
fn open_only_key(f: &Fido) -> Result<Dev, KeyError> {
    // SAFETY: the list and device are made, used and freed here, or the device handed back open.
    unsafe {
        let mut list = (f.info_new)(MOST);
        if list.is_null() {
            return Err(Failure::Failed {
                code: fido_code::INTERNAL,
            }
            .into());
        }
        let mut found = 0usize;
        let code = (f.info_manifest)(list, MOST, &mut found);
        let opened = (|| {
            if let Some(failure) = Failure::of(code, None) {
                return Err(KeyError::Key(failure));
            }
            match found {
                0 => return Err(Failure::NoKey.into()),
                1 => {}
                _ => return Err(Failure::SeveralKeys.into()),
            }
            let info = (f.info_ptr)(list, 0);
            let path = if info.is_null() {
                std::ptr::null()
            } else {
                (f.info_path)(info)
            };
            let device = (f.dev_new)();
            if path.is_null() || device.is_null() {
                return Err(Failure::Failed {
                    code: fido_code::INTERNAL,
                }
                .into());
            }
            if let Some(failure) = Failure::of((f.dev_open)(device, path), None) {
                let mut d = device;
                (f.dev_free)(&mut d);
                return Err(failure.into());
            }
            let refusal = if !(f.dev_is_fido2)(device) {
                Some(Failure::NotFido2)
            } else if !(f.dev_has_pin)(device) {
                Some(Failure::PinNotSet)
            } else {
                None
            };
            if let Some(r) = refusal {
                close(f, device);
                return Err(r.into());
            }
            if (f.dev_set_timeout)(device, WAIT_MS) != fido_code::OK {
                // A fixed line, nothing from the key: it then waits libfido2's own time.
                eprintln!("the security key refused its timeout");
            }
            Ok(device)
        })();
        (f.info_free)(&mut list, MOST);
        opened
    }
}

fn close(f: &Fido, device: Dev) {
    // SAFETY: an open device, closed and freed once.
    unsafe {
        (f.dev_close)(device);
        let mut d = device;
        (f.dev_free)(&mut d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Without a key plugged in (CI, this machine), libfido2 is loaded where it's installed and
    /// finds none, and every ask says so before any PIN is handed over.
    #[test]
    fn with_no_key_nothing_is_asked() {
        let key = OpenKey::default();
        let ask = AssertionAsk::new(&[1; 32], &[7]).unwrap();
        // No PIN: refused before the key is looked for.
        assert_eq!(
            key.get_assertion(&ask, None),
            Err(KeyError::Key(Failure::PinMissing))
        );
        let mut bytes = b"1234".to_vec();
        let mut pin = Pin::new(&mut bytes).unwrap();
        match plugged() {
            Ok(0) => {
                assert_eq!(
                    key.get_assertion(&ask, Some(&mut pin)),
                    Err(KeyError::Key(Failure::NoKey))
                );
            }
            Ok(_) => return, // a real key: not asked from a test
            Err(missing) => {
                assert_eq!(
                    key.get_assertion(&ask, Some(&mut pin)),
                    Err(KeyError::Missing(missing))
                );
            }
        }
        assert!(
            !pin.is_held(),
            "the PIN is cleared even when no key was asked"
        );
    }

    /// One request each: a cancelled one stays cancelled, and the next ask is a new one.
    #[test]
    fn a_cancel_stops_its_own_request_only() {
        let first = OpenKey::default();
        first.cancel();
        assert!(first.state.lock().unwrap().1);
        assert!(!OpenKey::default().state.lock().unwrap().1);
    }

    #[test]
    fn the_options_are_libfido2s() {
        assert_eq!(opt(FidoOption::Omit), 0);
        assert_eq!(opt(FidoOption::No), 1);
        assert_eq!(opt(FidoOption::Yes), 2);
    }
}
