//! Account › Devices (`DevicesView`): the devices signed in as this person on the account, each
//! with when it was last used (`KIFFClient.Device`, `GET /v1/me/devices`), and the iOS
//! `RelativeDateTimeFormatter` line it says that with.

use chrono::{DateTime, Duration, FixedOffset, Utc};
use serde_json::Value;

use crate::words::{WordBook, WordsError};

/// A device signed in as this person (`KIFFClient.Device`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub id: String,
    /// KIFF's name for it in the app's language (`display_name`), else the one it gave at
    /// sign-in (`name`).
    pub name: Option<String>,
    /// When it last reached KIFF (`last_used_at`; `last_seen_at` from a session before devices).
    pub last_used: Option<DateTime<FixedOffset>>,
    pub this_device: bool,
}

/// A string, absent or null; anything else fails the read (`decodeIfPresent`).
fn strict_text(v: &Value, key: &str) -> Result<Option<String>, ()> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(()),
    }
}

fn strict_date(v: &Value, key: &str) -> Result<Option<DateTime<FixedOffset>>, ()> {
    match strict_text(v, key)? {
        None => Ok(None),
        Some(s) => DateTime::parse_from_rfc3339(&s).map(Some).map_err(|_| ()),
    }
}

impl Device {
    fn read(v: &Value) -> Result<Device, ()> {
        Ok(Device {
            id: v.get("id").and_then(Value::as_str).ok_or(())?.to_string(),
            name: match strict_text(v, "display_name")? {
                Some(n) => Some(n),
                None => strict_text(v, "name")?,
            },
            last_used: match strict_date(v, "last_used_at")? {
                Some(d) => Some(d),
                None => strict_date(v, "last_seen_at")?,
            },
            this_device: match v.get("this_device") {
                None | Some(Value::Null) => false,
                Some(Value::Bool(b)) => *b,
                Some(_) => return Err(()),
            },
        })
    }

    /// KIFF's answer, as strictly as the iPad's decoder: one device it can't read fails the read.
    pub fn read_all(body: &[u8]) -> Option<Vec<Device>> {
        let v: Value = serde_json::from_slice(body).ok()?;
        v.get("devices")?
            .as_array()?
            .iter()
            .map(Device::read)
            .collect::<Result<_, _>>()
            .ok()
    }
}

/// `GET /v1/me/devices` at KIFF.
pub const PATH: &str = "/v1/me/devices";

/// This computer's id in KIFF (`KIFFClient.thisDeviceID`), from the same answer: the first device
/// marked `this_device` whose id is a device's (`dev_…`). planque asks KIFF about the device by
/// it. Read as the iPad's own decoder reads it, its id and mark alone: a device without its id,
/// or with a mark that isn't true or false, fails the read. None when KIFF lists this session
/// without a device yet.
pub fn this_device_id(body: &[u8]) -> Option<String> {
    let v: Value = serde_json::from_slice(body).ok()?;
    let mut found = None;
    for d in v.get("devices")?.as_array()? {
        let id = d.get("id")?.as_str()?;
        let this = match d.get("this_device") {
            None | Some(Value::Null) => false,
            Some(Value::Bool(b)) => *b,
            Some(_) => return None,
        };
        if found.is_none() && this && id.starts_with("dev_") {
            found = Some(id.to_string());
        }
    }
    found
}

/// A row's second line (`DevicesView.lastUsed`): "Last used 3 hours ago", or "Not used yet".
pub fn last_used(
    w: &WordBook,
    at: Option<DateTime<FixedOffset>>,
    now: DateTime<Utc>,
) -> Result<String, WordsError> {
    let Some(at) = at else {
        return w.line("settings.devices.not_used");
    };
    w.get(
        "settings.devices.last_used",
        &[("ago", &relative(w.language(), at.with_timezone(&Utc) - now))],
    )
}

/// iOS's `RelativeDateTimeFormatter` (full, numeric) in each shipped language: the largest unit
/// that fits, "3 hours ago" for a time past and "in 0 seconds" for now and later.
pub fn relative(language: &str, delta: Duration) -> String {
    use crate::locale::Unit;
    let secs = delta.num_seconds().unsigned_abs() as i64;
    // Under a second either way is 0, which iOS says in the future form ("in 0 seconds").
    let past = delta < Duration::zero() && secs > 0;
    let (n, unit) = match secs {
        s if s < 60 => (s, Unit::Second),
        s if s < 3600 => (s / 60, Unit::Minute),
        s if s < 86_400 => (s / 3600, Unit::Hour),
        s if s < 7 * 86_400 => (s / 86_400, Unit::Day),
        s if s < 30 * 86_400 => (s / (7 * 86_400), Unit::Week),
        s if s < 365 * 86_400 => (s / (30 * 86_400), Unit::Month),
        s => (s / (365 * 86_400), Unit::Year),
    };
    crate::locale::relative(language, n, unit, past)
}

/// The reference set's Devices over the sample (`SampleScreens.devices`): this computer, used
/// now, a phone 3 hours ago and a Mac a day ago.
pub fn sample(w: &WordBook, now: DateTime<Utc>) -> Result<Vec<Device>, WordsError> {
    let name = |kind: &str| w.get("settings.devices.this_device_name", &[("device", kind)]);
    let at = |hours: i64| Some((now - Duration::hours(hours)).fixed_offset());
    Ok(vec![
        Device {
            id: "dev_this".into(),
            name: Some(name(&w.line("device.linux")?)?),
            last_used: at(0),
            this_device: true,
        },
        Device {
            id: "dev_phone".into(),
            name: Some(name("iPhone")?),
            last_used: at(3),
            this_device: false,
        },
        Device {
            id: "dev_mac".into(),
            name: Some(name("Mac")?),
            last_used: at(26),
            this_device: false,
        },
    ])
}

/// The question before another device is signed out (`DevicesView`'s confirmation): "Sign Rae's
/// iPhone out?", or "Sign this device out?" for one with no name; Sign out in red, and Cancel. None
/// for this computer, which signs out with Sign out of KIFF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignOutQuestion {
    pub title: String,
    pub button: String,
    pub cancel: String,
    /// What the iPad signs once Sign out is tapped (`removeDevice`).
    pub act: crate::signed::Act,
    /// The act as the iPad names it to Face ID: "Sign Rae's iPhone out of your account".
    pub prompt: String,
}

pub fn sign_out_question(
    w: &WordBook,
    device: &Device,
) -> Result<Option<SignOutQuestion>, WordsError> {
    if device.this_device {
        return Ok(None);
    }
    Ok(Some(SignOutQuestion {
        title: match &device.name {
            Some(name) => w.get("settings.devices.sign_out_named", &[("device", name)])?,
            None => w.line("settings.devices.sign_out_this")?,
        },
        button: w.line("settings.devices.sign_out")?,
        cancel: w.line("common.cancel")?,
        act: crate::signed::Act::SignOutDevice {
            device: device.id.clone(),
        },
        prompt: w.get(
            "settings.devices.sign_out_prompt",
            &[(
                "device",
                &match &device.name {
                    Some(n) => n.clone(),
                    None => w.line("settings.devices.this_device")?,
                },
            )],
        )?,
    }))
}

/// Where the account's passkeys are read (`KIFFClient.passkeysPath`), as the session.
pub const PASSKEYS_PATH: &str = "/v1/me/passkeys";

/// One of the account's passkeys (`KIFFClient.Passkey`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Passkey {
    pub id: Option<String>,
    pub name: Option<String>,
    /// The computer it's that computer's own passkey of, when it is one.
    pub device: Option<String>,
}

impl Passkey {
    /// Only a passkey of its own can be removed: a computer's goes with the computer.
    pub fn removable(&self) -> bool {
        self.id.is_some() && self.device.is_none()
    }

    /// `passkeys()`'s answer: `passkeys`, none sent being none. A field present but not a string,
    /// or a list that isn't one, fails the read, as the iPad's decoder.
    pub fn read_all(body: &[u8]) -> Option<Vec<Passkey>> {
        let v: Value = serde_json::from_slice(body).ok()?;
        let list = match v.as_object()?.get("passkeys") {
            None | Some(Value::Null) => return Some(vec![]),
            Some(l) => l.as_array()?,
        };
        list.iter()
            .map(|p| {
                p.as_object()?;
                Some(Passkey {
                    id: strict_text(p, "id").ok()?,
                    name: strict_text(p, "name").ok()?,
                    device: strict_text(p, "device_id").ok()?,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passkeys_read_as_the_ipad_reads_them() {
        let p = Passkey::read_all(
            br#"{"passkeys":[{"id":"pk_1","name":"Rae's iPhone","synced":true,"created_at":"2026-11-01T09:00:00Z"},
            {"id":"pk_2","name":null,"device_id":"dev_mac"}]}"#,
        )
        .unwrap();
        assert_eq!(p.len(), 2);
        assert!(p[0].removable());
        assert!(!p[1].removable());
        assert_eq!(Passkey::read_all(b"{}").unwrap(), vec![]);
        assert_eq!(Passkey::read_all(br#"{"passkeys":[{"name":3}]}"#), None);
        assert!(!crate::signed::signed("GET", PASSKEYS_PATH));
    }

    #[test]
    fn relative_times_as_ios_says_them() {
        let ago = |s: i64| Duration::seconds(-s);
        assert_eq!(relative("en", Duration::zero()), "in 0 seconds");
        assert_eq!(relative("en", Duration::milliseconds(-400)), "in 0 seconds");
        assert_eq!(relative("en", ago(3 * 3600)), "3 hours ago");
        assert_eq!(relative("en", ago(26 * 3600)), "1 day ago");
        assert_eq!(relative("en", ago(60)), "1 minute ago");
        assert_eq!(relative("fr", ago(3 * 3600)), "il y a 3 heures");
        assert_eq!(relative("fr", ago(26 * 3600)), "il y a 1 jour");
        assert_eq!(relative("de", ago(26 * 3600)), "vor 1 Tag");
        assert_eq!(relative("de", ago(3 * 86_400)), "vor 3 Tagen");
        assert_eq!(relative("pt", ago(3 * 3600)), "há 3 horas");
        assert_eq!(relative("pl", ago(3 * 3600)), "3 godziny temu");
        assert_eq!(relative("pl", ago(5 * 3600)), "5 godzin temu");
        assert_eq!(relative("nl", ago(2 * 7 * 86_400)), "2 weken geleden");
        assert_eq!(relative("it", Duration::seconds(5)), "tra 5 secondi");
    }

    #[test]
    fn this_computer_is_the_device_kiff_marks() {
        let body = br#"{"devices":[{"id":"dev_1","last_used_at":"yesterday"},{"id":"ses_2","this_device":true},{"id":"dev_3","this_device":true},{"id":"dev_4","this_device":true}]}"#;
        assert_eq!(this_device_id(body).as_deref(), Some("dev_3"));
        assert_eq!(this_device_id(br#"{"devices":[{"id":"dev_1"}]}"#), None);
        // Its id and mark are read strictly; nothing else is.
        assert_eq!(
            this_device_id(br#"{"devices":[{"id":"dev_1","this_device":"yes"}]}"#),
            None
        );
        assert_eq!(
            this_device_id(
                br#"{"devices":[{"this_device":true},{"id":"dev_2","this_device":true}]}"#
            ),
            None
        );
        assert_eq!(this_device_id(br#"{}"#), None);
    }

    #[test]
    fn devices_read_as_strictly_as_the_ipad() {
        let all = Device::read_all(
            br#"{"devices":[{"id":"dev_1","display_name":"tafs on Linux","name":"old","last_seen_at":"2026-11-15T10:00:00Z","this_device":true},{"id":"dev_2"}]}"#,
        )
        .unwrap();
        assert_eq!(all[0].name.as_deref(), Some("tafs on Linux"));
        assert!(all[0].last_used.is_some() && all[0].this_device);
        assert_eq!(all[1].name, None);
        assert!(!all[1].this_device);
        // A date it can't read, or a device without its id, fails the whole read.
        assert!(
            Device::read_all(br#"{"devices":[{"id":"d","last_used_at":"yesterday"}]}"#).is_none()
        );
        assert!(Device::read_all(br#"{"devices":[{"name":"x"}]}"#).is_none());
        assert!(Device::read_all(br#"{}"#).is_none());
    }

    #[test]
    fn signing_another_device_out_is_asked_then_signed() {
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        let now = chrono::Utc::now();
        let devices = sample(&w, now).unwrap();
        let here = devices.iter().find(|d| d.this_device).unwrap();
        assert_eq!(sign_out_question(&w, here).unwrap(), None);
        let other = devices.iter().find(|d| !d.this_device).unwrap();
        let q = sign_out_question(&w, other).unwrap().unwrap();
        assert_eq!(
            q.title,
            format!("Sign {} out?", other.name.as_deref().unwrap())
        );
        assert_eq!(
            (q.button.as_str(), q.cancel.as_str()),
            ("Sign out", "Cancel")
        );
        let unnamed = Device {
            name: None,
            ..other.clone()
        };
        assert_eq!(
            sign_out_question(&w, &unnamed).unwrap().unwrap().title,
            "Sign this device out?"
        );
        // Signed by the approval key: it can't be sent from here.
        let (method, path) = q.act.route();
        assert!(crate::signed::Unsigned::new(method, path, vec![]).is_none());
    }
}
