//! Export my chats (`ExportChatsView.swift`, `TafsCloud.exportChats` and `splitExport`,
//! `AppModel.exportChats`; kiff/tafs#424): the person's own chats, trials and notes, for their
//! rights of access and portability, from `GET /v1/me/chats/export`, as two files:
//! `tafs-chats.json` and `tafs-chats.txt`.

use serde_json::Value;

use crate::words::{WordBook, WordsError};

pub const PATH: &str = "/v1/me/chats/export";
pub const JSON_FILE: &str = "tafs-chats.json";
pub const TEXT_FILE: &str = "tafs-chats.txt";

/// The two files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Files {
    /// The answer without `readable`, its keys sorted, pretty-printed.
    pub json: Vec<u8>,
    /// `readable`, the same as text; empty when tafs cloud sent none.
    pub text: String,
}

/// `splitExport`: tafs cloud's answer split into its two files; none when it isn't an object.
pub fn split(body: &[u8]) -> Option<Files> {
    let Value::Object(mut o) = serde_json::from_slice::<Value>(body).ok()? else {
        return None;
    };
    let text = match o.remove("readable") {
        Some(Value::String(s)) => s,
        _ => String::new(),
    };
    // serde_json's map keeps its keys sorted, as `.sortedKeys`; slashes are never escaped.
    let json = serde_json::to_vec_pretty(&Value::Object(o)).ok()?;
    Some(Files { json, text })
}

/// The section's words (`ExportChatsSection`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lines {
    pub row: String,
    pub try_again: String,
    pub preparing: String,
    pub footer: String,
}

pub fn lines(w: &WordBook) -> Result<Lines, WordsError> {
    Ok(Lines {
        row: w.line("export.row")?,
        try_again: w.line("common.try_again")?,
        preparing: w.line("export.preparing")?,
        footer: w.line("export.own_chats_each_taf_coach")?,
    })
}

/// "Ready: {files}. Share again", naming the two files.
pub fn ready(w: &WordBook) -> Result<String, WordsError> {
    w.get(
        "export.ready_share_again",
        &[("files", &format!("{JSON_FILE}, {TEXT_FILE}"))],
    )
}

/// Why it wasn't exported: not read (or not what tafs cloud sends), or not saved here.
pub fn not_read(w: &WordBook) -> Result<String, WordsError> {
    w.line("connect.chats_were_not_exported_check")
}

pub fn not_saved(w: &WordBook) -> Result<String, WordsError> {
    let device = w.line("device.linux")?;
    w.get("chat.chats_were_not_exported_could", &[("device", &device)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_export_is_split_into_its_two_files() {
        let body = br#"{"readable":"Chats in tafs\nRae: no chats","tafs":[{"id":"t/1","name":"Rae","chats":[],"notes":[]}],"person":"p","exported_at":"2026-11-16T03:00:00Z","account":"a"}"#;
        let f = split(body).unwrap();
        assert_eq!(f.text, "Chats in tafs\nRae: no chats");
        let s = String::from_utf8(f.json.clone()).unwrap();
        assert!(!s.contains("readable"));
        // Sorted keys, pretty-printed, slashes as they are.
        let keys: Vec<usize> = ["\"account\"", "\"exported_at\"", "\"person\"", "\"tafs\""]
            .iter()
            .map(|k| s.find(k).unwrap())
            .collect();
        assert!(keys.windows(2).all(|p| p[0] < p[1]), "{s}");
        assert!(s.contains("\"t/1\""));
        assert!(s.contains("\n  "));
        let back: Value = serde_json::from_slice(&f.json).unwrap();
        assert_eq!(back["tafs"][0]["name"], "Rae");
        // No readable: an empty text file.
        assert_eq!(split(br#"{"tafs":[]}"#).unwrap().text, "");
        // Not an object: not what tafs cloud sends.
        assert_eq!(split(b"[]"), None);
        assert_eq!(split(b"nope"), None);
    }

    #[test]
    fn the_words_read_in_every_language() {
        let en = WordBook::load(&WordBook::default_root(), "en").unwrap();
        assert_eq!(
            ready(&en).unwrap(),
            "Ready: tafs-chats.json, tafs-chats.txt. Share again"
        );
        assert_eq!(lines(&en).unwrap().row, "Export my chats");
        for lang in crate::words::SHIPPED {
            let w = WordBook::load(&WordBook::default_root(), lang).unwrap();
            lines(&w).unwrap();
            ready(&w).unwrap();
            not_read(&w).unwrap();
            not_saved(&w).unwrap();
        }
    }
}
