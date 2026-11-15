//! Signing this computer in with KIFF: the device authorization flow the iPad uses
//! (`KIFFClient.startDeviceSignIn`, `pollDeviceToken`, `AppModel.startSignIn`, `finishSignIn`),
//! and KIFF's desktop client contract §1.1.
//!
//! This is the flow's logic only: what is sent, how each answer reads, and when to poll again.
//! The app does the HTTP, through [`Transport`], so the whole flow is tested here without KIFF.

use std::time::Duration;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::words::{WordBook, WordsError};

/// KIFF's API (contract §1).
pub const KIFF_API: &str = "https://api.kiff.dev";

/// The platform KIFF records for this computer's key (`ThisDevice.platform`): `linux`.
pub const PLATFORM: &str = "linux";

/// What KIFF records the device as (`ThisDevice.kiffKind`): a computer is `other`.
pub const KIFF_KIND: &str = "other";

/// The answer to `POST /v1/auth/device/start` (`KIFFClient.DeviceStart`).
#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct DeviceStart {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: i64,
    pub interval: i64,
}

impl std::fmt::Debug for DeviceStart {
    // The device code is a secret until it's used: never printed.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceStart")
            .field("user_code", &self.user_code)
            .field("verification_uri_complete", &self.verification_uri_complete)
            .field("expires_in", &self.expires_in)
            .field("interval", &self.interval)
            .finish_non_exhaustive()
    }
}

impl DeviceStart {
    /// The approval page's link in the app's language (`lang`), so KIFF's page speaks it
    /// whatever the browser's; a link that already names one keeps it (`inAppLanguage`).
    pub fn in_app_language(mut self, language: &str) -> DeviceStart {
        let link = &self.verification_uri_complete;
        let names_one = link.contains("?lang=") || link.contains("&lang=");
        if !link.is_empty() && !names_one {
            let sep = if link.contains('?') { '&' } else { '?' };
            self.verification_uri_complete = format!("{link}{sep}lang={language}");
        }
        self
    }
}

/// The answer to `POST /v1/auth/device/token` once approved (`KIFFClient.DeviceToken`).
#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct DeviceToken {
    pub access_token: String,
    #[serde(default)]
    pub expires_in: i64,
    #[serde(default)]
    pub tenant: Option<String>,
    #[serde(default)]
    pub subject: Option<String>,
    /// How the session approves (contract §1.1): `view`, `key`, `passkey`, or `device_key`.
    #[serde(default)]
    pub approval_mode: Option<String>,
}

impl std::fmt::Debug for DeviceToken {
    // The token is handed over once and never printed.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceToken")
            .field("tenant", &self.tenant)
            .field("approval_mode", &self.approval_mode)
            .finish_non_exhaustive()
    }
}

/// What can go wrong (`KIFFClient.KIFFError`, the cases sign-in meets).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KiffError {
    AuthorizationPending,
    SlowDown,
    Expired,
    Unauthorized,
    /// KIFF refused: its status and its machine-readable error, never shown as it is.
    Server {
        status: u16,
        message: String,
    },
    /// It didn't reach KIFF (the iPad's `URLError`s that `transient` lists).
    Offline,
    /// The answer couldn't be read, or its link isn't KIFF's.
    Unreadable,
}

impl KiffError {
    /// The line the person reads (`errorDescription`, and `PlainError.words` for a refusal).
    pub fn words(&self, w: &WordBook) -> Result<String, WordsError> {
        match self {
            KiffError::AuthorizationPending => w.line("error.sign_in_pending"),
            KiffError::SlowDown => w.line("error.slow_down"),
            KiffError::Expired => w.line("error.sign_in_code_expired"),
            KiffError::Unauthorized => w.line("error.signed_out"),
            KiffError::Server { status, .. } => plain_words(w, *status),
            // Linux has no system line for it, as Windows: the iPad's offline line.
            KiffError::Offline => w.line("error.offline"),
            KiffError::Unreadable => plain_words(w, 200),
        }
    }

    /// Worth asking again while the code lasts (`AppModel.transient`): it didn't reach KIFF, or
    /// KIFF answered 5xx or 429.
    pub fn transient(&self) -> bool {
        match self {
            KiffError::Offline => true,
            KiffError::Server { status, .. } => *status >= 500 || *status == 429,
            _ => false,
        }
    }
}

/// `PlainError.words(status:)`: a refusal in plain words, by its status alone.
pub fn plain_words(w: &WordBook, status: u16) -> Result<String, WordsError> {
    w.line(match status {
        0 => "error.offline",
        401 => "error.youre_signed_out",
        403 => "error.not_allowed",
        409 => "error.changed",
        429 => "error.busy",
        500..=599 => "error.down",
        _ => "error.ours",
    })
}

/// One HTTP answer: its status and body.
#[derive(Debug, Clone)]
pub struct Answer {
    pub status: u16,
    pub body: Vec<u8>,
}

/// The app's HTTP: a JSON POST to KIFF with no session. `Err(Offline)` when it didn't get there.
pub trait Transport {
    fn post(&self, path: &str, body: &[u8]) -> Result<Answer, KiffError>;
}

/// What a refusal says (`send`'s refusal branch): 401 is signed out; otherwise KIFF's `error`,
/// or "KIFF answered 400" when it gave none.
pub fn refusal(answer: &Answer, w: Option<&WordBook>) -> KiffError {
    if answer.status == 401 {
        return KiffError::Unauthorized;
    }
    let fields: Option<Value> = serde_json::from_slice(&answer.body).ok();
    let error = fields
        .as_ref()
        .and_then(|f| f.get("error"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let message = error.unwrap_or_else(|| {
        w.and_then(|w| {
            w.get(
                "error.kiff_answered",
                &[("status", &answer.status.to_string())],
            )
            .ok()
        })
        .unwrap_or_else(|| format!("KIFF answered {}", answer.status))
    });
    KiffError::Server {
        status: answer.status,
        message,
    }
}

/// The start's body (`startDeviceSignIn`, contract §1.1). Linux holds no approval key: the
/// session is passkey-only (`"approval": "passkey"`), and its device key is in software, so it
/// says `"key_store": "software"`.
pub fn start_body(device_key_spki_b64: &str, device_name: &str) -> Value {
    json!({
        "public_key": device_key_spki_b64,
        "approval": "passkey",
        "key_store": "software",
        "device_name": device_name,
        "platform": PLATFORM,
        "kind": KIFF_KIND,
    })
}

/// The device's name in KIFF's list: "tafs on this computer", in the person's language.
pub fn device_name(w: &WordBook) -> Result<String, WordsError> {
    let device = w.line("device.linux")?;
    w.get("pair.device_name", &[("device", &device)])
}

/// Whether a link may be opened: https, on KIFF's own domain (`kiff.dev` or a host under it),
/// on the default port, with no user in it. Anything else refuses the whole answer.
pub fn is_kiff_link(link: &str) -> bool {
    let Ok(url) = url::Url::parse(link) else {
        return false;
    };
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
    {
        return false;
    }
    match url.host_str() {
        Some(host) => host == "kiff.dev" || host.ends_with(".kiff.dev"),
        None => false,
    }
}

/// `POST /v1/auth/device/start`, read: the code, its link in the app's language.
pub fn start(
    t: &dyn Transport,
    body: &Value,
    language: &str,
    w: Option<&WordBook>,
) -> Result<DeviceStart, KiffError> {
    let bytes = serde_json::to_vec(body).map_err(|_| KiffError::Unreadable)?;
    let answer = t.post("/v1/auth/device/start", &bytes)?;
    if !(200..300).contains(&answer.status) {
        return Err(refusal(&answer, w));
    }
    let start: DeviceStart =
        serde_json::from_slice(&answer.body).map_err(|_| KiffError::Unreadable)?;
    if !is_kiff_link(&start.verification_uri_complete) || !is_kiff_link(&start.verification_uri) {
        return Err(KiffError::Unreadable);
    }
    Ok(start.in_app_language(language))
}

/// One poll (`pollDeviceToken`): the session, or why not yet.
pub fn poll(
    t: &dyn Transport,
    device_code: &str,
    w: Option<&WordBook>,
) -> Result<DeviceToken, KiffError> {
    let bytes = serde_json::to_vec(&json!({ "device_code": device_code }))
        .map_err(|_| KiffError::Unreadable)?;
    let answer = t.post("/v1/auth/device/token", &bytes)?;
    if (200..300).contains(&answer.status) {
        return serde_json::from_slice(&answer.body).map_err(|_| KiffError::Unreadable);
    }
    match refusal(&answer, w) {
        KiffError::Server {
            status: 400,
            message,
        } => match message.as_str() {
            "authorization_pending" => Err(KiffError::AuthorizationPending),
            "slow_down" => Err(KiffError::SlowDown),
            "expired_token" => Err(KiffError::Expired),
            _ => Err(KiffError::Server {
                status: 400,
                message,
            }),
        },
        other => Err(other),
    }
}

/// The least time between two polls, whatever KIFF advertises (`signInMinimumInterval`).
pub const MINIMUM_INTERVAL: Duration = Duration::from_secs(2);

/// What `slow_down` adds to the wait (`finishSignIn`).
pub const SLOW_DOWN: Duration = Duration::from_secs(5);

/// The poll's clock and its wait, so tests run it at once.
pub trait Clock {
    /// Seconds since the sign-in started.
    fn elapsed(&self) -> Duration;
    /// Waits `d`, or less when the person comes back from the browser (`nudgeSignIn`).
    /// `Err` when the sign-in was cancelled.
    fn wait(&self, d: Duration) -> Result<(), Cancelled>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cancelled;

/// Why polling stopped without a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stopped {
    Cancelled,
    Failed(KiffError),
}

/// `finishSignIn`: polls until the person approves, waiting the interval (at least 2 s) before
/// each try and 5 s more on `slow_down`. A dropped connection or a 5xx/429 asks again; the code
/// running out, or any other refusal, stops it.
pub fn finish(
    t: &dyn Transport,
    clock: &dyn Clock,
    start: &DeviceStart,
    w: Option<&WordBook>,
) -> Result<DeviceToken, Stopped> {
    let mut interval = Duration::from_secs(start.interval.max(0) as u64).max(MINIMUM_INTERVAL);
    let deadline = Duration::from_secs(start.expires_in.max(0) as u64);
    while clock.elapsed() < deadline {
        clock.wait(interval).map_err(|_| Stopped::Cancelled)?;
        match poll(t, &start.device_code, w) {
            Ok(token) => return Ok(token),
            Err(KiffError::AuthorizationPending) => continue,
            Err(KiffError::SlowDown) => interval += SLOW_DOWN,
            Err(e) if e.transient() => continue,
            Err(e) => return Err(Stopped::Failed(e)),
        }
    }
    Err(Stopped::Failed(KiffError::Expired))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    struct Script {
        answers: RefCell<Vec<Result<Answer, KiffError>>>,
        sent: RefCell<Vec<(String, Value)>>,
    }

    impl Script {
        fn new(answers: Vec<Result<Answer, KiffError>>) -> Script {
            Script {
                answers: RefCell::new(answers),
                sent: RefCell::new(vec![]),
            }
        }
    }

    impl Transport for Script {
        fn post(&self, path: &str, body: &[u8]) -> Result<Answer, KiffError> {
            self.sent
                .borrow_mut()
                .push((path.to_string(), serde_json::from_slice(body).unwrap()));
            self.answers.borrow_mut().remove(0)
        }
    }

    struct Fake {
        now: Cell<u64>,
        waits: RefCell<Vec<u64>>,
    }

    impl Clock for Fake {
        fn elapsed(&self) -> Duration {
            Duration::from_secs(self.now.get())
        }
        fn wait(&self, d: Duration) -> Result<(), Cancelled> {
            self.now.set(self.now.get() + d.as_secs());
            self.waits.borrow_mut().push(d.as_secs());
            Ok(())
        }
    }

    fn ok(v: Value) -> Result<Answer, KiffError> {
        Ok(Answer {
            status: 200,
            body: serde_json::to_vec(&v).unwrap(),
        })
    }

    fn err(status: u16, error: &str) -> Result<Answer, KiffError> {
        Ok(Answer {
            status,
            body: serde_json::to_vec(&json!({ "error": error })).unwrap(),
        })
    }

    fn started() -> Value {
        json!({
            "device_code": "secret-device-code",
            "user_code": "ABCD-EFGH",
            "verification_uri": "https://app.kiff.dev/device",
            "verification_uri_complete": "https://app.kiff.dev/device?code=ABCD-EFGH",
            "expires_in": 600,
            "interval": 5
        })
    }

    #[test]
    fn start_sends_a_passkey_only_software_key_linux_start() {
        let body = start_body("KEY", "tafs on this computer");
        assert_eq!(body["platform"], "linux");
        assert_eq!(body["kind"], "other");
        assert_eq!(body["approval"], "passkey");
        assert_eq!(body["key_store"], "software");
        assert!(body.get("approval_public_key").is_none());
        let t = Script::new(vec![ok(started())]);
        let s = start(&t, &body, "fr", None).unwrap();
        assert_eq!(t.sent.borrow()[0].0, "/v1/auth/device/start");
        assert_eq!(
            s.verification_uri_complete,
            "https://app.kiff.dev/device?code=ABCD-EFGH&lang=fr"
        );
        assert!(!format!("{s:?}").contains("secret-device-code"));
    }

    #[test]
    fn a_link_that_names_a_language_keeps_it() {
        let mut v = started();
        v["verification_uri_complete"] = json!("https://app.kiff.dev/device?code=X&lang=de");
        let s: DeviceStart = serde_json::from_value(v).unwrap();
        assert_eq!(
            s.in_app_language("fr").verification_uri_complete,
            "https://app.kiff.dev/device?code=X&lang=de"
        );
    }

    #[test]
    fn only_kiffs_own_https_links_open() {
        assert!(is_kiff_link("https://app.kiff.dev/device?code=A"));
        assert!(is_kiff_link("https://kiff.dev/x"));
        for bad in [
            "http://app.kiff.dev/device",
            "https://evilkiff.dev/device",
            "https://app.kiff.dev.evil.com/",
            "https://user@app.kiff.dev/",
            "https://app.kiff.dev:8443/",
            "file:///etc/passwd",
            "tafs://device",
        ] {
            assert!(!is_kiff_link(bad), "{bad}");
        }
        let mut v = started();
        v["verification_uri_complete"] = json!("https://evilkiff.dev/device");
        let t = Script::new(vec![ok(v)]);
        assert_eq!(
            start(&t, &start_body("K", "n"), "en", None),
            Err(KiffError::Unreadable)
        );
    }

    #[test]
    fn a_refused_start_is_kiffs_status_and_code() {
        let t = Script::new(vec![err(400, "invalid_device_name")]);
        assert_eq!(
            start(&t, &start_body("K", ""), "en", None),
            Err(KiffError::Server {
                status: 400,
                message: "invalid_device_name".into()
            })
        );
    }

    #[test]
    fn polls_until_approved_slowing_down_when_asked() {
        let token = json!({ "access_token": "tok", "token_type": "Bearer", "expires_in": 7776000, "tenant": "t1", "subject": "s", "approval_mode": "view" });
        let t = Script::new(vec![
            err(400, "authorization_pending"),
            err(400, "slow_down"),
            err(503, "unavailable"),
            Err(KiffError::Offline),
            ok(token),
        ]);
        let clock = Fake {
            now: Cell::new(0),
            waits: RefCell::new(vec![]),
        };
        let s: DeviceStart = serde_json::from_value(started()).unwrap();
        let got = finish(&t, &clock, &s, None).unwrap();
        assert_eq!(got.approval_mode.as_deref(), Some("view"));
        assert_eq!(*clock.waits.borrow(), vec![5, 5, 10, 10, 10]);
        assert!(
            t.sent
                .borrow()
                .iter()
                .all(|(p, b)| p == "/v1/auth/device/token"
                    && b["device_code"] == "secret-device-code")
        );
        assert!(!format!("{got:?}").contains("tok"));
    }

    #[test]
    fn the_interval_is_at_least_two_seconds() {
        let mut v = started();
        v["interval"] = json!(0);
        let s: DeviceStart = serde_json::from_value(v).unwrap();
        let t = Script::new(vec![ok(json!({ "access_token": "x" }))]);
        let clock = Fake {
            now: Cell::new(0),
            waits: RefCell::new(vec![]),
        };
        finish(&t, &clock, &s, None).unwrap();
        assert_eq!(*clock.waits.borrow(), vec![2]);
    }

    #[test]
    fn stops_on_expiry_and_on_a_refusal() {
        let s: DeviceStart = serde_json::from_value(started()).unwrap();
        let t = Script::new(vec![err(400, "expired_token")]);
        let clock = Fake {
            now: Cell::new(0),
            waits: RefCell::new(vec![]),
        };
        assert_eq!(
            finish(&t, &clock, &s, None),
            Err(Stopped::Failed(KiffError::Expired))
        );

        let t = Script::new(vec![err(403, "denied")]);
        let clock = Fake {
            now: Cell::new(0),
            waits: RefCell::new(vec![]),
        };
        assert_eq!(
            finish(&t, &clock, &s, None),
            Err(Stopped::Failed(KiffError::Server {
                status: 403,
                message: "denied".into()
            }))
        );

        // The 10 minutes run out while it's pending.
        let answers = (0..200)
            .map(|_| err(400, "authorization_pending"))
            .collect();
        let t = Script::new(answers);
        let clock = Fake {
            now: Cell::new(0),
            waits: RefCell::new(vec![]),
        };
        assert_eq!(
            finish(&t, &clock, &s, None),
            Err(Stopped::Failed(KiffError::Expired))
        );
        assert_eq!(clock.waits.borrow().len(), 120);
    }
}
