//! Account › Time zone (`TimeZonePicker`): tafs cloud's time zones for what the person typed
//! (`TafsCloud.TimeZoneChoice`, `GET /v1/time-zones?q=…`), the account's own checked.

use serde_json::Value;

/// One zone: its IANA id and tafs cloud's title for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zone {
    pub id: String,
    pub title: String,
}

/// tafs cloud's answer, as strictly as the iPad's decoder: one zone it can't read fails the read.
pub fn read(body: &[u8]) -> Option<Vec<Zone>> {
    let v: Value = serde_json::from_slice(body).ok()?;
    v.get("zones")?
        .as_array()?
        .iter()
        .map(|z| {
            Some(Zone {
                id: z.get("id")?.as_str()?.to_string(),
                title: z.get("title")?.as_str()?.to_string(),
            })
        })
        .collect()
}

/// The path for what was typed, every character but a letter or digit escaped (`query`).
pub fn path(q: &str) -> String {
    let mut out = String::from("/v1/time-zones?q=");
    for b in q.bytes() {
        if b.is_ascii_alphanumeric() {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Choosing `zone` for the account, from the list or as this computer's
/// (`setAccountTimeZone`): the owner's act, which the iPad signs (`tafs.time-zone.v1`) and then
/// sends as a signed setup call. This computer draws it up to the signature.
pub fn choose(zone: &str) -> crate::signed::Act {
    crate::signed::Act::SetTimeZone { zone: zone.into() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zones_read_strictly_and_paths_escape() {
        let z = read(br#"{"zones":[{"id":"Europe/Paris","title":"Paris (UTC+1)"}]}"#).unwrap();
        assert_eq!(z[0].id, "Europe/Paris");
        assert!(read(br#"{"zones":[{"id":"x"}]}"#).is_none());
        assert_eq!(path("new york/é"), "/v1/time-zones?q=new%20york%2F%C3%A9");
        assert_eq!(path(""), "/v1/time-zones?q=");
    }

    #[test]
    fn choosing_a_zone_is_signed() {
        let (method, path) = choose("Europe/Paris").route();
        assert_eq!((method, path.as_str()), ("PUT", "/v1/me/time-zone"));
        assert!(crate::signed::Unsigned::new(method, path, vec![]).is_none());
    }
}
