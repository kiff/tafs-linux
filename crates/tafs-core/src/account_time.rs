//! The account's time zone (`AccountTime` in `Components.swift`): every time the app draws is in
//! it. Signed in with KIFF it's the account's (`GET /v1/me`'s `time_zone`, UTC when KIFF sends
//! none, as KIFF's own default); in the sample and signed out it's this computer's. Set as the
//! account is read, read wherever a time is drawn.

use std::sync::RwLock;

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, Offset, TimeZone, Utc};
use chrono_tz::Tz;

/// None: this computer's zone.
static ZONE: RwLock<Option<Tz>> = RwLock::new(None);

/// The zone an identifier names; none (no account) is this computer's. An identifier this
/// computer doesn't know is UTC.
pub fn zone_of(identifier: Option<&str>) -> Option<Tz> {
    identifier.map(|id| id.parse::<Tz>().unwrap_or(Tz::UTC))
}

/// `AccountTime.use`: draws every time from now on in the account's zone.
pub fn use_zone(identifier: Option<&str>) {
    if let Ok(mut z) = ZONE.write() {
        *z = zone_of(identifier);
    }
}

fn zone() -> Option<Tz> {
    ZONE.read().ok().and_then(|z| *z)
}

fn local_in<T: TimeZone>(zone: Option<Tz>, at: &DateTime<T>) -> NaiveDateTime {
    let utc = at.with_timezone(&Utc);
    match zone {
        Some(tz) => utc.with_timezone(&tz).naive_local(),
        None => utc.with_timezone(&Local).naive_local(),
    }
}

fn label_in<T: TimeZone>(zone: Option<Tz>, at: &DateTime<T>) -> String {
    let Some(tz) = zone else {
        return String::new();
    };
    let there = at.with_timezone(&tz);
    let here = at.with_timezone(&Local).offset().fix();
    if there.offset().fix() == here {
        String::new()
    } else {
        format!(" {}", there.format("%Z"))
    }
}

/// An instant as the account's clock shows it.
pub fn local<T: TimeZone>(at: &DateTime<T>) -> NaiveDateTime {
    local_in(zone(), at)
}

/// The day an instant falls on, in the account's zone (`AccountTime.calendar`).
pub fn day<T: TimeZone>(at: &DateTime<T>) -> NaiveDate {
    local(at).date()
}

/// `shortTime`: the time of day, as 14:05, in the account's zone.
pub fn short_time<T: TimeZone>(at: &DateTime<T>) -> String {
    local(at).format("%H:%M").to_string()
}

/// `AccountTime.label`: the zone's short name after a time (" CET") when the account's zone
/// shows a different clock from this computer's at that moment; nothing when they agree.
pub fn label<T: TimeZone>(at: &DateTime<T>) -> String {
    label_in(zone(), at)
}

/// `zonedShortTime`: `short_time`, with the zone's short name when it isn't this computer's.
pub fn zoned_short_time<T: TimeZone>(at: &DateTime<T>) -> String {
    short_time(at) + &label(at)
}

/// This computer's zone by its identifier (`TimeZone.current.identifier`): `TZ` when it names
/// one, else what `/etc/localtime` links to, else UTC.
pub fn local_identifier() -> String {
    if let Ok(tz) = std::env::var("TZ") {
        let tz = tz.trim_start_matches(':');
        if tz.parse::<Tz>().is_ok() {
            return tz.to_string();
        }
    }
    std::fs::read_link("/etc/localtime")
        .ok()
        .and_then(|p| {
            let p = p.to_string_lossy().to_string();
            p.split_once("zoneinfo/").map(|(_, id)| id.to_string())
        })
        .filter(|id| id.parse::<Tz>().is_ok())
        .unwrap_or_else(|| "UTC".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_accounts_zone_draws_every_time() {
        let at = Utc.with_ymd_and_hms(2026, 11, 15, 23, 30, 0).unwrap();
        let tokyo = zone_of(Some("Asia/Tokyo"));
        assert_eq!(local_in(tokyo, &at).format("%H:%M").to_string(), "08:30");
        assert_eq!(
            local_in(tokyo, &at).date(),
            NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
        );
        // Tokyo's clock isn't this machine's unless it runs there.
        if Local
            .offset_from_utc_datetime(&at.naive_utc())
            .fix()
            .local_minus_utc()
            != 9 * 3600
        {
            assert_eq!(label_in(tokyo, &at), " JST");
        }
        // A zone this computer doesn't know is UTC; KIFF's default.
        assert_eq!(zone_of(Some("Mars/Olympus")), Some(Tz::UTC));
        assert_eq!(
            local_in(zone_of(Some("Mars/Olympus")), &at)
                .format("%H:%M")
                .to_string(),
            "23:30"
        );
        // No account: this computer's zone, and no label.
        assert_eq!(local_in(None, &at), at.with_timezone(&Local).naive_local());
        assert_eq!(label_in(None, &at), "");
    }
}
