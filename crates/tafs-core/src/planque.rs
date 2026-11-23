//! planque, which keeps a taf's vendor sign-ins (`PlanqueClient`): what this computer signs for it,
//! with its device key and no prompt, and nothing else. Only planque's *safe* routes, those that
//! take a device key as it is because they read or only take authority away (planque's
//! `signedSafe`):
//! - its read of the account's grants (`grantsReadProof`), which tafs cloud forwards as the
//!   device's proof (`Planque-Device-Signature`), so Coach's cards say what a sign-in can't be
//!   narrowed to (`wider`);
//! - the Chromes paired to the account (Account › Chrome), and unpairing one.
//!
//! Pairing a Chrome, approving or confirming anything stays the phone's.

use std::sync::Mutex;

use chrono::{DateTime, TimeZone, Utc};
use sha2::{Digest, Sha256};

use crate::keys::DeviceKey;
use crate::work::segment;

/// planque's base (`PlanqueClient.baseURL`).
pub const PLANQUE: &str = "https://planque.kiff.dev";

/// The header the proof travels in.
pub const HEADER: &str = "Planque-Device-Signature";

/// What the device signs (`PlanqueClient.message`, planque's `device.Message`): seven lines, no
/// trailing newline. The base without its trailing slashes, the tenant, the method, the path and
/// query as sent, the body's SHA-256 in lowercase hex, and the signing time.
fn message(
    base: &str,
    tenant: &str,
    method: &str,
    uri: &str,
    body: &[u8],
    signed_at: &str,
) -> Vec<u8> {
    let base = base.trim_end_matches('/');
    let hash: String = Sha256::digest(body)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    [
        "planque.request.v1",
        base,
        tenant,
        method,
        uri,
        &hash,
        signed_at,
    ]
    .join("\n")
    .into_bytes()
}

/// The last time this app signed with, in milliseconds since 1970 (`SigningTime.last`).
static LAST: Mutex<Option<i64>> = Mutex::new(None);

/// `SigningTime.next`: RFC 3339 in UTC with milliseconds, never the same twice in this app, each
/// later than the last. planque and KIFF refuse a signed request they have already seen, and the
/// time is part of what's signed: two reads in the same millisecond would look like a replay.
pub fn signing_time() -> String {
    signing_time_at(Utc::now())
}

/// `SigningTime.next(now:)`.
pub fn signing_time_at(now: DateTime<Utc>) -> String {
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    let ms = now.timestamp_millis();
    let at = match *last {
        Some(l) => ms.max(l + 1),
        None => ms,
    };
    *last = Some(at);
    Utc.timestamp_millis_opt(at)
        .single()
        .unwrap_or(now)
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

/// The account's grants at planque, its tenant escaped as one path segment.
pub fn grants_path(tenant: &str) -> String {
    format!("/v1/accounts/{}/grants", segment(tenant))
}

/// Where a safe request may go: planque's exact host, over HTTPS, on the default port, with no user
/// in the link.
pub fn is_planque(link: &str) -> bool {
    url::Url::parse(link).is_ok_and(|u| {
        u.scheme() == "https"
            && u.host_str() == Some("planque.kiff.dev")
            && u.port().is_none()
            && u.username().is_empty()
            && u.password().is_none()
    })
}

/// The only requests this computer signs at planque: its safe routes, each a read or a removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Safe<'a> {
    /// `GET …/grants`.
    Grants,
    /// `GET …/chromes`: the Chromes paired to the account.
    Chromes,
    /// `DELETE …/chromes/{chrome}`: one unpaired. It only takes authority away.
    Unpair(&'a str),
    /// `DELETE …/grants/{grant}`: a connection ended (Disconnect, Clear). It only takes access
    /// away, and this computer sends it only once its security key's touch and PIN checked the
    /// person (the owner's rule), never on the device key alone
    /// (`key_approval::end_with_presence`).
    Disconnect(&'a str),
    /// `GET …/chromes/screenshots?op={operation}`: the pictures kept for one operation's Chrome
    /// steps, without their bytes (planque#359).
    Shots(&'a str),
    /// `GET …/chromes/screenshots/{step}`: one step's picture.
    Shot(&'a str),
    /// `GET …/grants/{grant}/channels`: the channels a Slack bot connection can see, setup
    /// metadata, never a message.
    Channels(&'a str),
}

impl Safe<'_> {
    pub fn method(self) -> &'static str {
        match self {
            Safe::Grants | Safe::Chromes | Safe::Shots(_) | Safe::Shot(_) | Safe::Channels(_) => {
                "GET"
            }
            Safe::Unpair(_) | Safe::Disconnect(_) => "DELETE",
        }
    }

    /// The path as sent and signed, each segment escaped.
    pub fn path(self, tenant: &str) -> String {
        match self {
            Safe::Grants => grants_path(tenant),
            Safe::Chromes => format!("/v1/accounts/{}/chromes", segment(tenant)),
            Safe::Unpair(chrome) => {
                format!(
                    "/v1/accounts/{}/chromes/{}",
                    segment(tenant),
                    segment(chrome)
                )
            }
            Safe::Disconnect(grant) => format!("{}/{}", grants_path(tenant), segment(grant)),
            // The query is part of what's signed.
            Safe::Shots(op) => format!(
                "/v1/accounts/{}/chromes/screenshots?op={}",
                segment(tenant),
                segment(op)
            ),
            Safe::Shot(step) => format!(
                "/v1/accounts/{}/chromes/screenshots/{}",
                segment(tenant),
                segment(step)
            ),
            Safe::Channels(grant) => format!("{}/{}/channels", grants_path(tenant), segment(grant)),
        }
    }
}

/// This device's signature over one safe request, `t={at},v1={sig},d={device}`. The device key
/// signs it with no prompt (`signConfirmed`): it grants nothing on its own. `device` is this
/// device's id in KIFF, which planque asks KIFF about.
pub fn safe_proof(key: &DeviceKey, device: &str, tenant: &str, base: &str, route: Safe) -> String {
    let at = signing_time();
    let bytes = message(base, tenant, route.method(), &route.path(tenant), b"", &at);
    format!("t={at},v1={},d={device}", key.sign_b64(&bytes))
}

/// `grantsReadProof`: this device's signed read of the account's grants.
pub fn grants_read_proof(key: &DeviceKey, device: &str, tenant: &str, base: &str) -> String {
    safe_proof(key, device, tenant, base, Safe::Grants)
}

/// A Slack channel a Slack bot connection can see (`PlanqueClient.Channel`): setup metadata,
/// never a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub private: bool,
}

impl Channel {
    /// "#name", else "#id".
    pub fn label(&self) -> String {
        format!(
            "#{}",
            if self.name.is_empty() {
                &self.id
            } else {
                &self.name
            }
        )
    }
}

/// planque's `GET …/grants/{grant}/channels` (`{"channels": [{id, name?, private?}]}`): none
/// when it can't be read; a channel without an id is left out.
pub fn channels(body: &[u8]) -> Option<Vec<Channel>> {
    let v: serde_json::Value = serde_json::from_slice(body).ok()?;
    Some(
        v.get("channels")?
            .as_array()?
            .iter()
            .filter_map(|c| {
                Some(Channel {
                    id: c.get("id")?.as_str()?.to_string(),
                    name: c
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    private: c.get("private").and_then(|p| p.as_bool()) == Some(true),
                })
            })
            .collect(),
    )
}

/// Each connected tool's title, as its connection gives it, by its name in KIFF (the iPad's
/// `AppModel.toolTitle`), from planque's `GET …/grants`: `{"grants": [{"request": {"tools":
/// [{tool, name?, title?}]}}]}`. A tool's name in KIFF is its `name`, else its `tool`; the
/// first grant to give it a title that isn't blank wins. Read leniently: what can't be read
/// gives no title, never an error.
pub fn tool_titles(body: &[u8]) -> std::collections::HashMap<String, String> {
    let mut titles = std::collections::HashMap::new();
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(body) else {
        return titles;
    };
    let grants = v.get("grants").and_then(|g| g.as_array());
    for tool in grants
        .into_iter()
        .flatten()
        .filter_map(|g| g.get("request")?.get("tools")?.as_array())
        .flatten()
    {
        let action = tool
            .get("name")
            .and_then(|n| n.as_str())
            .or_else(|| tool.get("tool").and_then(|t| t.as_str()));
        let title = tool
            .get("title")
            .and_then(|t| t.as_str())
            .map(str::trim)
            .filter(|t| !t.is_empty());
        if let (Some(action), Some(title)) = (action, title) {
            titles
                .entry(action.to_string())
                .or_insert_with(|| title.to_string());
        }
    }
    titles
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This computer signs only planque's safe routes: the grants read, the Chromes list and
    /// unpairing one, each with an empty body. Nothing here can sign an approval, a confirm, a
    /// pairing or a window, and no other file builds planque's message.
    #[test]
    fn this_computer_signs_nothing_at_planque_but_its_safe_routes() {
        let root = env!("CARGO_MANIFEST_DIR");
        let source = std::fs::read_to_string(format!("{root}/src/planque.rs")).unwrap();
        let code = &source[..source.find("#[cfg(test)]").unwrap()];
        // One signature, in `safe_proof`, over a `Safe` route with no body, and nothing else.
        assert_eq!(code.matches("sign_b64(").count(), 1);
        assert_eq!(code.matches("message(base, tenant, ").count(), 1);
        assert!(code
            .contains(r#"message(base, tenant, route.method(), &route.path(tenant), b"", &at)"#));
        for route in [
            "approve", "confirm", "pairings", "POST", "PUT", "PATCH", "w=",
        ] {
            assert!(!code.contains(&format!("\"{route}")), "{route}");
        }
        // DELETE only for what takes access away: unpairing a Chrome, and ending a connection.
        assert_eq!(code.matches("\"DELETE\"").count(), 1);
        assert!(code.contains(r#"Safe::Unpair(_) | Safe::Disconnect(_) => "DELETE""#));
        assert_eq!(
            Safe::Disconnect("g/1").path("t 1"),
            "/v1/accounts/t%201/grants/g%2F1"
        );
        assert_eq!(Safe::Chromes.path("t 1"), "/v1/accounts/t%201/chromes");
        assert_eq!(
            Safe::Unpair("c/1").path("t"),
            "/v1/accounts/t/chromes/c%2F1"
        );
        assert!(is_planque(&format!("{PLANQUE}{}", Safe::Chromes.path("t"))));
        // A Chrome step's pictures: reads, the operation in the signed query, escaped.
        assert_eq!(Safe::Shots("op 1").method(), "GET");
        assert_eq!(
            Safe::Shots("prp_1&x=2").path("t"),
            "/v1/accounts/t/chromes/screenshots?op=prp_1%26x%3D2"
        );
        assert_eq!(
            Safe::Shot("s/1").path("t"),
            "/v1/accounts/t/chromes/screenshots/s%2F1"
        );
        for bad in [
            "http://planque.kiff.dev/v1",
            "https://planque.kiff.dev.evil.com/v1",
            "https://planque.kiff.dev:444/v1",
            "https://a@planque.kiff.dev/v1",
            "https://api.kiff.dev/v1",
        ] {
            assert!(!is_planque(bad), "{bad}");
        }
        // No other file in either crate builds planque's request message.
        let mut files = vec![];
        for dir in [format!("{root}/src"), format!("{root}/../tafs-app/src")] {
            let mut stack = vec![std::path::PathBuf::from(dir)];
            while let Some(d) = stack.pop() {
                for e in std::fs::read_dir(d).unwrap().flatten() {
                    let path = e.path();
                    if path.is_dir() {
                        stack.push(path);
                    } else if path.extension().is_some_and(|x| x == "rs") {
                        files.push(path);
                    }
                }
            }
        }
        let builders: Vec<_> = files
            .iter()
            .filter(|f| {
                std::fs::read_to_string(f)
                    .unwrap()
                    .contains("planque.request")
            })
            .collect();
        assert_eq!(builders.len(), 1, "{builders:?}");
        assert!(builders[0].ends_with("planque.rs"));
        // And the one proof is over that read.
        let key = DeviceKey::generate();
        let proof = grants_read_proof(&key, "dev_1", "t_1", PLANQUE);
        assert!(proof.starts_with("t=") && proof.contains(",v1=") && proof.ends_with(",d=dev_1"));
        assert!(!proof.contains(",w="));
    }
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use p256::ecdsa::signature::Verifier;
    use p256::ecdsa::{DerSignature, VerifyingKey};
    use p256::pkcs8::DecodePublicKey;

    #[test]
    fn the_message_is_plannques_seven_lines() {
        let m = message(
            "https://planque.kiff.dev//",
            "t_1",
            "GET",
            "/v1/accounts/t_1/grants",
            b"",
            "2026-11-16T10:00:00.123Z",
        );
        assert_eq!(
            String::from_utf8(m).unwrap(),
            "planque.request.v1\nhttps://planque.kiff.dev\nt_1\nGET\n/v1/accounts/t_1/grants\n\
             e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\n2026-11-16T10:00:00.123Z"
        );
        // A body is hashed, never sent in the message.
        let m = message(PLANQUE, "t", "POST", "/x", b"{}", "at");
        assert!(String::from_utf8(m)
            .unwrap()
            .contains("\n44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a\n"));
    }

    #[test]
    fn the_proof_is_the_header_planque_reads_and_it_verifies() {
        let key = DeviceKey::generate();
        let proof = grants_read_proof(&key, "dev_9", "t 1", PLANQUE);
        let parts: Vec<&str> = proof.split(',').collect();
        assert_eq!(parts.len(), 3);
        let at = parts[0].strip_prefix("t=").unwrap();
        let sig = parts[1].strip_prefix("v1=").unwrap();
        assert_eq!(parts[2], "d=dev_9");
        assert!(chrono::DateTime::parse_from_rfc3339(at).is_ok());
        assert_eq!(at.len(), "2026-11-16T10:00:00.123Z".len());
        let public = VerifyingKey::from_public_key_der(&key.public_spki_der()).unwrap();
        let der = STANDARD.decode(sig).unwrap();
        let sig = DerSignature::try_from(der.as_slice()).unwrap();
        // Over the grants' path, the tenant escaped as one segment.
        let signed = message(PLANQUE, "t 1", "GET", "/v1/accounts/t%201/grants", b"", at);
        public.verify(&signed, &sig).unwrap();
        assert!(public
            .verify(&message(PLANQUE, "t 1", "GET", "/v1/other", b"", at), &sig)
            .is_err());
    }

    #[test]
    fn signing_times_only_go_forward() {
        let now = Utc.with_ymd_and_hms(2026, 11, 16, 10, 0, 0).unwrap();
        let a = signing_time_at(now);
        let b = signing_time_at(now);
        let c = signing_time_at(now - chrono::Duration::seconds(5));
        assert!(a < b && b < c, "{a} {b} {c}");
        let d = signing_time();
        let e = signing_time();
        assert!(c < d && d < e, "{c} {d} {e}");
        assert!(e.ends_with('Z'));
    }

    #[test]
    fn a_tools_title_comes_from_its_connection_by_its_name_in_kiff() {
        let t = tool_titles(
            br#"{"grants":[
                {"id":"g1","request":{"tools":[
                    {"tool":"list_charges","name":"kiff__stripe_list_charges_ab12","title":"Reading List payments (charges)"},
                    {"tool":"refund","title":"  "},
                    {"tool":"plain","title":"Plain"}]}},
                {"id":"g2","request":{"tools":[
                    {"tool":"x","name":"kiff__stripe_list_charges_ab12","title":"Later"}]}},
                {"id":"g3"},
                {"id":"g4","request":{"tools":"nope"}}]}"#,
        );
        assert_eq!(
            t.get("kiff__stripe_list_charges_ab12").map(String::as_str),
            Some("Reading List payments (charges)")
        );
        // No name: its tool. A blank title is none.
        assert_eq!(t.get("plain").map(String::as_str), Some("Plain"));
        assert!(!t.contains_key("refund"));
        assert_eq!(t.len(), 2);
        // An answer that can't be read gives no titles, never an error.
        assert!(tool_titles(b"[]").is_empty());
        assert!(tool_titles(b"nope").is_empty());
    }

    #[test]
    fn a_slack_bots_channels_and_their_route() {
        let c = channels(br#"{"channels":[{"id":"C1","name":"refunds"},{"id":"C2","private":true},{"name":"no id"}]}"#).unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].label(), "#refunds");
        assert_eq!(c[1].label(), "#C2");
        assert!(c[1].private);
        assert_eq!(channels(b"{}"), None);
        assert_eq!(
            Safe::Channels("b/1").path("t1"),
            "/v1/accounts/t1/grants/b%2F1/channels"
        );
        assert_eq!(Safe::Channels("b1").method(), "GET");
    }
}
