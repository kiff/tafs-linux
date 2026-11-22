//! The kept session: this computer's KIFF session across launches, as the iPad keeps its token,
//! tenant and key in the Keychain (`AppModel.useSession`, `tokenAccount`, `tenantAccount`).
//!
//! The app seals it in the person's keyring (the Secret Service); this is what it holds and the
//! rules for reading it.

use serde::{Deserialize, Serialize};

use crate::keys::DeviceKey;
use crate::signin::DeviceToken;

/// KIFF's API host, the only one a kept session is read for.
pub const KIFF_HOST: &str = "api.kiff.dev";
/// tafs cloud's host: it asks KIFF who the session is, and gets only reads.
pub const TAFS_CLOUD_HOST: &str = "cloud.tafs.kiff.dev";

/// How the session approves (contract §1.1, §1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalMode {
    /// Reads, refuses and answers hand-offs; approves nothing.
    View,
    /// Approves with its approval key.
    Key,
    /// Approves with a passkey.
    Passkey,
}

impl ApprovalMode {
    /// KIFF's word. Anything it doesn't say plainly (none, `device_key`, a word this app doesn't
    /// know) counts as view: the app never claims more than KIFF granted.
    pub fn from_kiff(word: Option<&str>) -> ApprovalMode {
        match word {
            Some("key") => ApprovalMode::Key,
            Some("passkey") => ApprovalMode::Passkey,
            _ => ApprovalMode::View,
        }
    }

    /// The view-only notice shows exactly when KIFF says view.
    pub fn is_view_only(self) -> bool {
        self == ApprovalMode::View
    }
}

/// What the keyring holds.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeptSession {
    pub host: String,
    pub token: String,
    pub tenant: Option<String>,
    pub subject: Option<String>,
    pub approval_mode: ApprovalMode,
    /// Seconds since 1970, UTC.
    pub expires_at: i64,
    /// The device key the session is bound to, PKCS#8 DER in base64.
    pub device_key: String,
    /// Added with a phone (a pairing): its key is in software, so it stays view-only and its
    /// approvals go to the phone. A session kept before this was known reads as not paired.
    #[serde(default)]
    pub paired: bool,
    /// The security key registered with this computer when it was paired (`pairing_key`): its
    /// credential id, base64url. Approvals are its assertions, one request at a time; the device
    /// key grants nothing. None: view-only.
    #[serde(default)]
    pub security_key: Option<String>,
}

impl std::fmt::Debug for KeptSession {
    // The token and the key are never printed.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeptSession")
            .field("host", &self.host)
            .field("tenant", &self.tenant)
            .field("approval_mode", &self.approval_mode)
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}

impl KeptSession {
    /// The session KIFF handed over, bound to the key that signed in.
    pub fn from_token(token: &DeviceToken, key: &DeviceKey, now: i64) -> KeptSession {
        use base64::Engine;
        // KIFF's sessions last 90 days (contract §1) when the answer says nothing.
        let lasts = if token.expires_in > 0 {
            token.expires_in
        } else {
            90 * 24 * 60 * 60
        };
        KeptSession {
            host: KIFF_HOST.to_string(),
            token: token.access_token.clone(),
            tenant: token.tenant.clone(),
            subject: token.subject.clone(),
            approval_mode: ApprovalMode::from_kiff(token.approval_mode.as_deref()),
            expires_at: now + lasts,
            device_key: base64::engine::general_purpose::STANDARD.encode(key.to_pkcs8()),
            paired: false,
            security_key: None,
        }
    }

    /// As the keyring keeps it.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("a session encodes")
    }

    /// A kept session read back: only KIFF's own host, only until it expires. Anything
    /// unreadable reads as none.
    pub fn read(json: &str, now: i64) -> Option<KeptSession> {
        let s: KeptSession = serde_json::from_str(json).ok()?;
        if s.host != KIFF_HOST || s.token.is_empty() || s.expires_at <= now {
            return None;
        }
        Some(s)
    }

    pub fn key(&self) -> Option<DeviceKey> {
        use base64::Engine;
        let der = base64::engine::general_purpose::STANDARD
            .decode(&self.device_key)
            .ok()?;
        DeviceKey::from_pkcs8(&der)
    }
}

/// Where the token may go: https only, on exactly KIFF's API or tafs cloud, on the default
/// port, with no user in the link. The app refuses any other link before sending.
pub fn may_send_to(link: &str) -> bool {
    let Ok(url) = url::Url::parse(link) else {
        return false;
    };
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && matches!(url.host_str(), Some(KIFF_HOST) | Some(TAFS_CLOUD_HOST))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(mode: Option<&str>) -> DeviceToken {
        serde_json::from_value(serde_json::json!({
            "access_token": "secret-token", "expires_in": 100, "tenant": "t1", "subject": "u1", "approval_mode": mode
        }))
        .unwrap()
    }

    #[test]
    fn modes_count_as_view_unless_kiff_says_otherwise() {
        assert_eq!(ApprovalMode::from_kiff(Some("view")), ApprovalMode::View);
        assert_eq!(ApprovalMode::from_kiff(Some("key")), ApprovalMode::Key);
        assert_eq!(
            ApprovalMode::from_kiff(Some("passkey")),
            ApprovalMode::Passkey
        );
        assert_eq!(
            ApprovalMode::from_kiff(Some("device_key")),
            ApprovalMode::View
        );
        assert_eq!(ApprovalMode::from_kiff(Some("admin")), ApprovalMode::View);
        assert_eq!(ApprovalMode::from_kiff(None), ApprovalMode::View);
    }

    #[test]
    fn a_relaunch_reads_the_session_until_it_expires() {
        let key = DeviceKey::generate();
        let kept = KeptSession::from_token(&token(Some("passkey")), &key, 1_000);
        let json = kept.to_json();
        let back = KeptSession::read(&json, 1_050).unwrap();
        assert_eq!(back, kept);
        assert_eq!(back.key().unwrap().public_spki_der(), key.public_spki_der());
        assert!(KeptSession::read(&json, 1_100).is_none());
        assert!(KeptSession::read("{not json", 0).is_none());
        let mut other = kept.clone();
        other.host = "api.kiff.dev.evil.com".into();
        assert!(KeptSession::read(&other.to_json(), 0).is_none());
        assert!(!format!("{kept:?}").contains("secret-token"));
    }

    #[test]
    fn a_paired_session_is_kept_as_paired_and_an_older_one_reads_as_not() {
        let key = DeviceKey::generate();
        let signed_in = KeptSession::from_token(&token(Some("view")), &key, 1_000);
        assert!(!signed_in.paired);
        let paired = KeptSession {
            paired: true,
            ..signed_in.clone()
        };
        assert!(KeptSession::read(&paired.to_json(), 1_050).unwrap().paired);
        // A session kept before `paired` was written reads as not paired.
        let mut older: serde_json::Value = serde_json::from_str(&signed_in.to_json()).unwrap();
        older.as_object_mut().unwrap().remove("paired");
        let back = KeptSession::read(&older.to_string(), 1_050).unwrap();
        assert!(!back.paired);
        assert_eq!(back, signed_in);
    }

    #[test]
    fn the_token_goes_to_two_exact_hosts() {
        assert!(may_send_to("https://api.kiff.dev/v1/me"));
        assert!(may_send_to("https://cloud.tafs.kiff.dev/v1/home"));
        for bad in [
            "http://api.kiff.dev/v1/me",
            "https://app.kiff.dev/",
            "https://api.kiff.dev:444/",
            "https://a@api.kiff.dev/",
            "https://evil.com/?api.kiff.dev",
        ] {
            assert!(!may_send_to(bad), "{bad}");
        }
    }
}
