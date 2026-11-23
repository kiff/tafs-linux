//! The picture a paired Chrome kept just before a taf's step acted (kiff/tafs `docs/chrome.md`, "Screenshots of a run's
//! acts"; the iPhone's `ChromeShotView`, tafs#1725, and Android's tafs#1734). A Record line's receipt lists the pictures
//! kept for its operation (KIFF's `proposal_id`), and each opens its step's. Both reads are planque's safe routes, signed
//! by this computer's device key (`planque::Safe::{Shots, Shot}`); planque keeps none while its switch is off.
//!
//! A picture is shown only if it checks out: WebP, at most 120 KB, its SHA-256 the one planque kept, the step asked for,
//! and a time it was taken. Anything else is no picture, never shown.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sha2::{Digest, Sha256};

/// The most a picture may weigh: planque keeps none over 120 KB.
pub const LARGEST: usize = 120 * 1024;

/// One picture kept for an operation's Chrome step, as planque lists it: no bytes.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ShotRef {
    pub step: String,
    #[serde(default)]
    pub taken_at: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
}

/// planque's list for an operation, oldest first: none when there are none, when its switch is off (404), or when the
/// answer can't be read. A step without its id is left out.
pub fn read_list(status: u16, body: &[u8]) -> Vec<ShotRef> {
    #[derive(Deserialize)]
    struct List {
        #[serde(default)]
        shots: Vec<ShotRef>,
    }
    if !(200..300).contains(&status) {
        return vec![];
    }
    serde_json::from_slice::<List>(body)
        .map(|l| {
            l.shots
                .into_iter()
                .filter(|s| !s.step.trim().is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// A step's picture, checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shot {
    pub step: String,
    pub taken_at: DateTime<Utc>,
    /// The WebP bytes, as kept.
    pub image: Vec<u8>,
}

/// What reading a step's picture came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Read {
    Shown(Shot),
    /// planque keeps none for this step (404): past its 30 days, or never kept.
    None,
    /// Any other answer, or a picture that didn't check out.
    Failed,
}

/// planque's answer to `GET …/chromes/screenshots/{step}`, for the step asked.
pub fn read(status: u16, body: &[u8], step: &str) -> Read {
    match status {
        404 => Read::None,
        200..=299 => checked(body)
            .filter(|s| s.step == step)
            .map(Read::Shown)
            .unwrap_or(Read::Failed),
        _ => Read::Failed,
    }
}

/// The picture, or none when it isn't one to show: not the shape, not base64url, too big, not WebP, bytes whose
/// SHA-256 isn't planque's, or no time it was taken.
fn checked(body: &[u8]) -> Option<Shot> {
    let v: serde_json::Value = serde_json::from_slice(body).ok()?;
    let text = |k: &str| v.get(k).and_then(|x| x.as_str());
    let step = text("step")?.to_string();
    let sha = text("sha256")?.to_ascii_lowercase();
    let image = URL_SAFE_NO_PAD.decode(text("image")?).ok()?;
    if image.len() < 12 || image.len() > LARGEST {
        return None;
    }
    if &image[0..4] != b"RIFF" || &image[8..12] != b"WEBP" {
        return None;
    }
    let hash: String = Sha256::digest(&image)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if hash != sha {
        return None;
    }
    let taken_at = DateTime::parse_from_rfc3339(text("taken_at")?)
        .ok()?
        .with_timezone(&Utc);
    Some(Shot {
        step,
        taken_at,
        image,
    })
}

/// The picture as RGBA pixels, width and height: none when the WebP can't be decoded, so the sheet says the picture
/// couldn't be loaded rather than showing nothing.
pub fn pixels(shot: &Shot) -> Option<(u32, u32, Vec<u8>)> {
    // At most 120 KB, but a WebP can still claim any size: a page's picture is never wider or
    // taller than this, and decoding stops before it takes more memory (`image::Limits`).
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    limits.max_alloc = Some(MAX_ALLOC);
    let mut reader = image::ImageReader::with_format(
        std::io::Cursor::new(&shot.image[..]),
        image::ImageFormat::WebP,
    );
    reader.limits(limits);
    let rgba = reader.decode().ok()?.to_rgba8();
    Some((rgba.width(), rgba.height(), rgba.into_raw()))
}

/// The widest or tallest picture shown, in pixels.
pub const MAX_SIDE: u32 = 4096;
/// The most memory one picture's decoding may take: 4096 × 4096 RGBA.
pub const MAX_ALLOC: u64 = 64 * 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_wider_than_any_page_has_no_pixels() {
        let mut out = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(MAX_SIDE + 1, 1, image::Rgba([0, 0, 0, 255]))
            .write_to(&mut out, image::ImageFormat::WebP)
            .unwrap();
        let shot = |image| Shot {
            step: "s".into(),
            taken_at: Utc::now(),
            image,
        };
        assert_eq!(pixels(&shot(out.into_inner())), None);
        assert!(pixels(&shot(webp())).is_some());
    }

    /// A 1×1 lossless WebP.
    fn webp() -> Vec<u8> {
        let mut out = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(1, 1, image::Rgba([10, 20, 30, 255]))
            .write_to(&mut out, image::ImageFormat::WebP)
            .unwrap();
        out.into_inner()
    }

    fn answer(step: &str, bytes: &[u8], sha: Option<String>, taken_at: Option<&str>) -> Vec<u8> {
        let sha = sha.unwrap_or_else(|| {
            Sha256::digest(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect()
        });
        let mut v = serde_json::json!({"step": step, "sha256": sha, "image": URL_SAFE_NO_PAD.encode(bytes)});
        if let Some(t) = taken_at {
            v["taken_at"] = t.into();
        }
        serde_json::to_vec(&v).unwrap()
    }

    #[test]
    fn the_list_reads_and_none_is_no_link() {
        let l = read_list(
            200,
            br#"{"shots":[{"step":"s1","taken_at":"2026-11-22T20:00:00Z","sha256":"ab"},{"step":" "},{"step":"s2"}]}"#,
        );
        assert_eq!(
            l.iter().map(|s| s.step.as_str()).collect::<Vec<_>>(),
            ["s1", "s2"]
        );
        assert!(read_list(200, br#"{"shots":[]}"#).is_empty());
        assert!(read_list(404, b"").is_empty());
        assert!(read_list(500, b"").is_empty());
        assert!(read_list(200, b"not json").is_empty());
    }

    #[test]
    fn a_picture_is_shown_only_when_it_checks_out() {
        let img = webp();
        let at = Some("2026-11-22T20:00:00Z");
        let Read::Shown(s) = read(200, &answer("s1", &img, None, at), "s1") else {
            panic!("a checked picture");
        };
        assert_eq!(s.image, img);
        assert_eq!(pixels(&s).map(|(w, h, p)| (w, h, p.len())), Some((1, 1, 4)));
        // Another step's picture is never shown under this one.
        assert_eq!(read(200, &answer("s2", &img, None, at), "s1"), Read::Failed);
        // Another hash, not WebP, too big, no time, a bad time: no picture.
        assert_eq!(
            read(200, &answer("s1", &img, Some("00".repeat(32)), at), "s1"),
            Read::Failed
        );
        let png = b"\x89PNG\r\n\x1a\n0000000000".to_vec();
        assert_eq!(read(200, &answer("s1", &png, None, at), "s1"), Read::Failed);
        let mut big = img.clone();
        big.resize(LARGEST + 1, 0);
        assert_eq!(read(200, &answer("s1", &big, None, at), "s1"), Read::Failed);
        assert_eq!(
            read(200, &answer("s1", &img, None, None), "s1"),
            Read::Failed
        );
        assert_eq!(
            read(200, &answer("s1", &img, None, Some("yesterday")), "s1"),
            Read::Failed
        );
        // Padded base64 isn't base64url as planque writes it.
        let padded = serde_json::to_vec(&serde_json::json!({
            "step": "s1", "taken_at": "2026-11-22T20:00:00Z",
            "sha256": Sha256::digest(&img).iter().map(|b| format!("{b:02x}")).collect::<String>(),
            "image": base64::engine::general_purpose::STANDARD.encode(&img) + "=",
        }))
        .unwrap();
        assert_eq!(read(200, &padded, "s1"), Read::Failed);
        assert_eq!(read(200, b"not json", "s1"), Read::Failed);
        // None kept is said as such; any other refusal is a failed read.
        assert_eq!(read(404, b"", "s1"), Read::None);
        assert_eq!(read(503, b"", "s1"), Read::Failed);
        assert_eq!(read(403, b"", "s1"), Read::Failed);
    }

    #[test]
    fn a_picture_that_wont_decode_has_no_pixels() {
        let mut fake = b"RIFF\0\0\0\0WEBP".to_vec();
        fake.extend_from_slice(&[0; 20]);
        let s = Shot {
            step: "s".into(),
            taken_at: Utc::now(),
            image: fake,
        };
        assert_eq!(pixels(&s), None);
    }
}
