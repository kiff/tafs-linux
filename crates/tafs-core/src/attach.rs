//! Files a person attaches to a message (`Services/Attachments.swift`, `AttachViews.swift`,
//! `AppModel`'s attachments): what may be attached and how much each kind may weigh, checked here
//! first so a file over its limit is refused before upload (`FilePrep`); each file declared to
//! tafs cloud (`POST /v1/tafs/{id}/attachments`, with its SHA-256), which answers where its bytes
//! go, straight to storage (`Upload`); and the files waiting for the chat's next message
//! (`PendingFile`, `chatFiles`). Nothing here is signed: the declaration is the session's.

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::coach::{field, int, object, text, Attachment, Read};
use crate::words::{WordBook, WordsError};

/// The most files one message carries (`FilePrep.perMessage`).
pub const PER_MESSAGE: usize = 5;

/// A photo's long edge, at most, in pixels (`FilePrep.longEdge`).
pub const LONG_EDGE: i32 = 2576;

/// The tray's thumbnail's long edge.
pub const THUMBNAIL_EDGE: i32 = 160;

const MB: u64 = 1 << 20;

/// A file that isn't an image is refused over this before it's read (`AttachButton.read`).
pub const READ_AT_MOST: u64 = 25 * MB;

/// `FilePrep.limits`: the media types tafs takes, with the most each may weigh.
pub fn limit(media_type: &str) -> Option<u64> {
    Some(match media_type {
        "image/jpeg" | "image/png" | "image/gif" | "image/webp" => 10 * MB,
        "application/pdf" => 25 * MB,
        "text/plain"
        | "text/csv"
        | "text/markdown"
        | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => 2 * MB,
        _ => return None,
    })
}

/// The kinds the file chooser offers (`FilePrep.pickable`), as MIME types.
pub const PICKABLE: [&str; 9] = [
    "application/pdf",
    "text/plain",
    "text/csv",
    "text/markdown",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "image/*",
    "image/heic",
    "image/heif",
];

/// `FilePrep.mediaType`: the media type tafs sends a file of this kind as (`content_type`, as this
/// computer reads it), or none when it can't be attached. Images are sent as photos (a JPEG).
pub fn media_type(content_type: &str) -> Option<&'static str> {
    let t = content_type.to_ascii_lowercase();
    Some(match t.as_str() {
        "application/pdf" => "application/pdf",
        "text/csv" | "text/comma-separated-values" => "text/csv",
        "text/markdown" | "text/x-markdown" => "text/markdown",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        }
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        }
        "image/jpeg" => "image/jpeg",
        "image/png" => "image/png",
        "image/gif" => "image/gif",
        "image/webp" => "image/webp",
        // Plain text, and any other text that conforms to it.
        _ if t.starts_with("text/") => "text/plain",
        _ => return None,
    })
}

/// Whether a file of this kind is an image: made a JPEG as a photo is.
pub fn is_image(content_type: &str) -> bool {
    content_type.to_ascii_lowercase().starts_with("image/")
}

/// `FilePrep.megabytes`: "2.4 MB".
pub fn megabytes(n: u64) -> String {
    format!("{:.1} MB", n as f64 / MB as f64)
}

/// Why a file can't be attached, in words the person reads (`FileProblem`).
pub type Problem = String;

/// `FilePrep.check`: refused when its kind isn't one tafs takes, or it's over that kind's limit.
pub fn check(
    w: &WordBook,
    name: &str,
    size: u64,
    media_type: &str,
) -> Result<Option<Problem>, WordsError> {
    let Some(most) = limit(media_type) else {
        return Ok(Some(w.get("attachment.cant_attach", &[("name", name)])?));
    };
    if size > most {
        return Ok(Some(w.get(
            "attachment.too_large",
            &[
                ("name", name),
                ("size", &megabytes(size)),
                ("limit", &megabytes(most)),
            ],
        )?));
    }
    Ok(None)
}

/// A file from the chooser, before it's read: refused when it isn't a kind tafs takes, or, not
/// an image, over every limit (`AttachButton.read`, `FilePrep.file`).
pub fn before_reading(
    w: &WordBook,
    name: &str,
    size: u64,
    content_type: &str,
) -> Result<Option<Problem>, WordsError> {
    if is_image(content_type) {
        return Ok(None);
    }
    if size > READ_AT_MOST {
        return Ok(Some(w.get(
            "attachment.too_large",
            &[
                ("name", name),
                ("size", &megabytes(size)),
                ("limit", &megabytes(READ_AT_MOST)),
            ],
        )?));
    }
    match media_type(content_type) {
        Some(mt) => check(w, name, size, mt),
        None => Ok(Some(w.get("attachment.wrong_kind", &[("name", name)])?)),
    }
}

/// A photo's name once it's a JPEG: its own name without its extension, or "Photo", then `.jpg`.
pub fn photo_name(w: &WordBook, name: &str) -> Result<String, WordsError> {
    let base = match name.rfind('.') {
        Some(i) if i > 0 => &name[..i],
        Some(_) => "",
        None => name,
    };
    let base = if base.is_empty() {
        w.line("attachment.photo")?
    } else {
        base.to_string()
    };
    Ok(format!("{base}.jpg"))
}

/// A photo's size, its long edge at most `edge` (`FilePrep.scaled`): never larger, never nothing.
pub fn scaled(width: i32, height: i32, edge: i32) -> (i32, i32) {
    let long = width.max(height);
    if long <= 0 {
        return (width.max(1), height.max(1));
    }
    let ratio = (f64::from(edge) / f64::from(long)).min(1.0);
    (
        ((f64::from(width) * ratio).round() as i32).max(1),
        ((f64::from(height) * ratio).round() as i32).max(1),
    )
}

/// A file made ready to attach (`PreparedFile`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prepared {
    pub name: String,
    pub media_type: String,
    pub data: Vec<u8>,
    /// A photo's small JPEG for the tray.
    pub thumbnail: Option<Vec<u8>>,
}

/// `FilePrep.sha256`: lowercase hex.
pub fn sha256(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// `attach(_:)`'s path.
pub fn attach_path(taf: &str) -> String {
    format!("/v1/tafs/{}/attachments", crate::work::segment(taf))
}

/// `attach(_:)`'s body: the file declared, never its bytes.
pub fn attach_body(file: &Prepared) -> Vec<u8> {
    json!({
        "media_type": file.media_type,
        "name": file.name,
        "size": file.data.len(),
        "sha256": sha256(&file.data),
    })
    .to_string()
    .into_bytes()
}

/// Where a file's bytes go (`Upload`): a request to `url` with `headers`, sent as given, straight
/// to storage with no session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Upload {
    pub url: String,
    pub method: String,
    pub headers: Vec<(String, String)>,
}

impl Upload {
    /// Only an HTTPS URL, and a method that carries bytes: nothing else is sent.
    pub fn sendable(&self) -> bool {
        url::Url::parse(&self.url).is_ok_and(|u| {
            u.scheme() == "https"
                && u.username().is_empty()
                && u.password().is_none()
                && u.host_str().is_some()
        }) && matches!(self.method.as_str(), "PUT" | "POST")
    }
}

fn attachment(v: &Value) -> Read<Attachment> {
    let a = object(v)?;
    Ok(Attachment {
        id: text(a, "id")?,
        kind: text(a, "kind")?,
        media_type: text(a, "media_type")?,
        name: text(a, "name")?,
        size: int(field(a, "size").ok_or(())?)?,
    })
}

/// `attach(_:)`'s answer: the file as the thread will name it, and where its bytes go. None for
/// any other shape, as the iPad's decoder throws.
pub fn attached(body: &[u8]) -> Option<(Attachment, Upload)> {
    let v: Value = serde_json::from_slice(body).ok()?;
    let r: &Map<String, Value> = v.as_object()?;
    let a = attachment(field(r, "attachment")?).ok()?;
    let u = object(field(r, "upload")?).ok()?;
    let headers = object(field(u, "headers")?)
        .ok()?
        .iter()
        .map(|(k, v)| Some((k.clone(), v.as_str()?.to_string())))
        .collect::<Option<Vec<_>>>()?;
    Some((
        a,
        Upload {
            url: text(u, "url").ok()?,
            method: text(u, "method").ok()?,
            headers,
        },
    ))
}

/// Where a file stands in the tray (`PendingFile.State`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Uploading,
    Ready(Attachment),
    Failed(String),
}

/// A file picked for the next message (`PendingFile`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    pub id: u64,
    pub name: String,
    pub media_type: String,
    pub thumbnail: Option<Vec<u8>>,
    pub state: State,
}

impl Pending {
    pub fn attachment(&self) -> Option<&Attachment> {
        match &self.state {
            State::Ready(a) => Some(a),
            _ => None,
        }
    }
}

/// The files picked for one chat's next message, as they go up (`chatFiles`, `fileNotices`).
#[derive(Debug, Clone, Default)]
pub struct Files {
    pub files: Vec<Pending>,
    /// Why a file wasn't added, or a message with files didn't go.
    pub notice: Option<String>,
    next: u64,
}

impl Files {
    /// `attach`'s start: the file in the tray, going up; none when the message is full, and the
    /// notice says so.
    pub fn add(&mut self, w: &WordBook, file: &Prepared) -> Result<Option<u64>, WordsError> {
        self.notice = None;
        if self.files.len() >= PER_MESSAGE {
            self.notice = Some(w.get(
                "chat.message_carries_most_files",
                &[("per_message", &PER_MESSAGE.to_string())],
            )?);
            return Ok(None);
        }
        self.next += 1;
        self.files.push(Pending {
            id: self.next,
            name: file.name.clone(),
            media_type: file.media_type.clone(),
            thumbnail: file.thumbnail.clone(),
            state: State::Uploading,
        });
        Ok(Some(self.next))
    }

    /// The upload's end: ready, or why not. A file removed meanwhile stays removed.
    pub fn settle(&mut self, id: u64, state: State) {
        if let Some(f) = self.files.iter_mut().find(|f| f.id == id) {
            f.state = state;
        }
    }

    /// `refuseFile`: said in the tray, never uploaded.
    pub fn refuse(&mut self, why: String) {
        self.notice = Some(why);
    }

    pub fn remove(&mut self, id: u64) {
        self.files.retain(|f| f.id != id);
    }

    /// Room left in this message.
    pub fn room(&self) -> usize {
        PER_MESSAGE.saturating_sub(self.files.len())
    }

    /// `filesUploading`.
    pub fn uploading(&self) -> bool {
        self.files.iter().any(|f| f.state == State::Uploading)
    }

    /// Some file is ready to go with the message.
    pub fn ready(&self) -> bool {
        self.files.iter().any(|f| f.attachment().is_some())
    }

    /// `holdsSend`: Send waits while a file is still going up, and while files are ready but
    /// the chat's agent is at work: files go with a message of their own.
    pub fn holds_send(&self, thinking: bool) -> bool {
        self.uploading() || (thinking && self.ready())
    }

    /// `takeFiles`: the ready files, taken for the message; ones that failed stay.
    pub fn take(&mut self) -> Vec<Pending> {
        let (ready, rest): (Vec<Pending>, Vec<Pending>) = std::mem::take(&mut self.files)
            .into_iter()
            .partition(|f| f.attachment().is_some());
        self.files = rest;
        ready
    }

    /// `putBackFiles`: what a message didn't take, ahead of any picked since.
    pub fn put_back(&mut self, taken: Vec<Pending>) {
        if taken.is_empty() {
            return;
        }
        let since = std::mem::take(&mut self.files);
        self.files = taken.into_iter().chain(since).collect();
    }
}

/// A file's tray line under its name: Uploading, Ready, or why it failed.
pub fn state_words(w: &WordBook, s: &State) -> Result<String, WordsError> {
    match s {
        State::Uploading => w.line("chat.file.uploading"),
        State::Ready(_) => w.line("chat.file.ready"),
        State::Failed(why) => Ok(why.clone()),
    }
}

/// `FileIcon.symbol`: a photo, a PDF, a sheet, or text, as this computer's icon names.
pub fn icon(media_type: &str) -> &'static str {
    match media_type {
        t if t.starts_with("image/") => "image-x-generic-symbolic",
        "application/pdf" => "x-office-document-symbolic",
        "text/csv" | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => {
            "x-office-spreadsheet-symbolic"
        }
        _ => "text-x-generic-symbolic",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book() -> WordBook {
        WordBook::load(&WordBook::default_root(), "en").unwrap()
    }

    fn prepared(name: &str, mt: &str, n: usize) -> Prepared {
        Prepared {
            name: name.into(),
            media_type: mt.into(),
            data: vec![b'a'; n],
            thumbnail: None,
        }
    }

    #[test]
    fn the_kinds_and_limits_kiff_agents_keeps() {
        assert_eq!(media_type("application/pdf"), Some("application/pdf"));
        assert_eq!(media_type("text/x-markdown"), Some("text/markdown"));
        assert_eq!(media_type("text/x-python"), Some("text/plain"));
        assert_eq!(media_type("text/csv"), Some("text/csv"));
        assert_eq!(media_type("application/zip"), None);
        assert_eq!(limit("application/pdf"), Some(25 << 20));
        assert_eq!(limit("image/png"), Some(10 << 20));
        assert_eq!(limit("text/csv"), Some(2 << 20));
        assert_eq!(limit("image/heic"), None);
        assert_eq!(megabytes(2_621_440), "2.5 MB");
    }

    #[test]
    fn a_file_over_its_limit_or_of_another_kind_is_refused_before_upload() {
        let w = book();
        assert_eq!(check(&w, "a.pdf", 1000, "application/pdf").unwrap(), None);
        assert_eq!(
            check(&w, "notes.txt", 3 << 20, "text/plain")
                .unwrap()
                .unwrap(),
            "notes.txt is 3.0 MB; the most is 2.0 MB."
        );
        assert_eq!(
            before_reading(&w, "big.pdf", 30 << 20, "application/pdf")
                .unwrap()
                .unwrap(),
            "big.pdf is 30.0 MB; the most is 25.0 MB."
        );
        assert!(before_reading(&w, "a.zip", 10, "application/zip")
            .unwrap()
            .unwrap()
            .starts_with("a.zip can't be attached. Photos"));
        // An image is read whatever its size: it's made a JPEG first.
        assert_eq!(
            before_reading(&w, "p.heic", 40 << 20, "image/heic").unwrap(),
            None
        );
    }

    #[test]
    fn a_photo_is_a_jpeg_named_as_it_was_and_no_larger_than_its_edge() {
        let w = book();
        assert_eq!(photo_name(&w, "IMG_1.HEIC").unwrap(), "IMG_1.jpg");
        assert_eq!(photo_name(&w, ".png").unwrap(), "Photo.jpg");
        assert_eq!(photo_name(&w, "scan").unwrap(), "scan.jpg");
        assert_eq!(scaled(5152, 3864, LONG_EDGE), (2576, 1932));
        assert_eq!(scaled(800, 600, LONG_EDGE), (800, 600));
        assert_eq!(scaled(10000, 1, THUMBNAIL_EDGE), (160, 1));
    }

    #[test]
    fn the_declaration_names_the_file_and_its_hash_never_its_bytes() {
        let f = prepared("a.txt", "text/plain", 3);
        let v: Value = serde_json::from_slice(&attach_body(&f)).unwrap();
        assert_eq!(
            v,
            json!({"media_type": "text/plain", "name": "a.txt", "size": 3,
                   "sha256": "9834876dcfb05cb167a5c24953eba58c4ac89b1adf57f28f2f9d09af107ee8f0"})
        );
        assert_eq!(attach_path("taf-1/x"), "/v1/tafs/taf-1%2Fx/attachments");
        // The session's declaration, never a signed call.
        assert!(crate::signed::Unsigned::new("POST", attach_path("taf-1"), vec![]).is_some());
    }

    #[test]
    fn tafs_clouds_answer_says_where_the_bytes_go() {
        let body = br#"{"attachment":{"id":"att_1","kind":"text","media_type":"text/plain","name":"a.txt","size":3},
            "upload":{"url":"https://storage.example/put?sig=x","method":"PUT","headers":{"Content-Type":"text/plain"}}}"#;
        let (a, u) = attached(body).unwrap();
        assert_eq!(a.id, "att_1");
        assert_eq!(
            u.headers,
            [("Content-Type".to_string(), "text/plain".to_string())]
        );
        assert!(u.sendable());
        // Of another shape: none.
        assert_eq!(attached(br#"{"attachment":{"id":"a"},"upload":{}}"#), None);
        assert_eq!(
            attached(br#"{"attachment":{"id":"a","kind":"k","media_type":"m","name":"n","size":1},"upload":{"url":"u","method":"PUT","headers":{"x":1}}}"#),
            None
        );
        // Only HTTPS, with a method that carries bytes.
        for (url, method) in [
            ("http://x/y", "PUT"),
            ("https://x/y", "GET"),
            ("file:///etc", "PUT"),
        ] {
            let u = Upload {
                url: url.into(),
                method: method.into(),
                headers: vec![],
            };
            assert!(!u.sendable(), "{url} {method}");
        }
    }

    #[test]
    fn the_tray_holds_five_and_send_waits_for_them() {
        let w = book();
        let mut f = Files::default();
        let ids: Vec<u64> = (0..5)
            .map(|i| {
                f.add(&w, &prepared(&format!("{i}.txt"), "text/plain", 1))
                    .unwrap()
                    .unwrap()
            })
            .collect();
        assert_eq!(f.room(), 0);
        assert_eq!(
            f.add(&w, &prepared("6.txt", "text/plain", 1)).unwrap(),
            None
        );
        assert_eq!(
            f.notice.as_deref(),
            Some("A message carries at most 5 files.")
        );
        assert!(f.uploading() && f.holds_send(false));
        let a = |id: &str| Attachment {
            id: id.into(),
            kind: "text".into(),
            media_type: "text/plain".into(),
            name: "x".into(),
            size: 1,
        };
        for (i, id) in ids.iter().enumerate() {
            f.settle(
                *id,
                if i == 4 {
                    State::Failed("no".into())
                } else {
                    State::Ready(a(&i.to_string()))
                },
            );
        }
        assert!(!f.holds_send(false));
        // While the agent works, ready files wait for a message of their own.
        assert!(f.holds_send(true));
        f.remove(ids[0]);
        let taken = f.take();
        assert_eq!(taken.len(), 3);
        assert_eq!(f.files.len(), 1, "the failed one stays");
        f.add(&w, &prepared("new.txt", "text/plain", 1)).unwrap();
        f.put_back(taken);
        assert_eq!(f.files[0].name, "1.txt");
        assert_eq!(f.files.last().unwrap().name, "new.txt");
        assert_eq!(state_words(&w, &State::Uploading).unwrap(), "Uploading");
        assert_eq!(state_words(&w, &State::Ready(a("z"))).unwrap(), "Ready");
    }
}
