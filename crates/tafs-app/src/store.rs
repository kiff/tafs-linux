//! The kept session, sealed in the person's keyring (the Secret Service: GNOME Keyring, KWallet,
//! KeePassXC), as the iPad keeps its session in the Keychain. Another user on this computer
//! can't read it, and it's unlocked with the person's own sign-in.

use std::collections::HashMap;

use tafs_core::session::KeptSession;

const LABEL: &str = "tafs: KIFF session";

fn attributes() -> HashMap<&'static str, &'static str> {
    HashMap::from([("application", "dev.kiff.tafs"), ("kind", "kiff-session")])
}

/// Why the keyring refused, without anything that could hold a path or a secret.
#[derive(Debug, Clone)]
pub struct StoreError(pub &'static str);

fn run<T>(f: impl std::future::Future<Output = Result<T, oo7::Error>>) -> Result<T, StoreError> {
    futures_lite::future::block_on(f).map_err(|e| {
        StoreError(match e {
            oo7::Error::File(_) => "keyring file",
            oo7::Error::DBus(_) => "Secret Service",
        })
    })
}

/// The session kept here, if one is there and still good.
pub fn load(now: i64) -> Option<KeptSession> {
    run(async {
        let keyring = oo7::Keyring::new().await?;
        let _ = keyring.unlock().await;
        let items = keyring.search_items(&attributes()).await?;
        for item in items {
            let secret = item.secret().await?;
            if let Ok(text) = std::str::from_utf8(secret.as_bytes()) {
                if let Some(s) = KeptSession::read(text, now) {
                    return Ok(Some(s));
                }
            }
        }
        Ok(None)
    })
    .ok()
    .flatten()
}

/// Keeps a session, then reads it back: a keyring that refuses is an error, never a session
/// that's gone at the next launch.
pub fn save(session: &KeptSession, now: i64) -> Result<(), StoreError> {
    let json = session.to_json();
    run(async {
        let keyring = oo7::Keyring::new().await?;
        let _ = keyring.unlock().await;
        keyring
            .create_item(LABEL, &attributes(), json.as_bytes(), true)
            .await
    })?;
    match load(now) {
        Some(back) if back == *session => Ok(()),
        _ => Err(StoreError("read back")),
    }
}

/// Whether the keyring takes a session, asked before a pairing shows its code
/// (`SecretStore.checkWritable`, tafs-macos #30): the session kept is written over itself, or,
/// with none, an empty item is written, read back and deleted. A keyring that refuses stops the
/// pairing there: no code, no claim, and nothing for the phone to approve.
pub fn check_writable(now: i64) -> Result<(), StoreError> {
    if let Some(kept) = load(now) {
        return save(&kept, now);
    }
    const PROBE: &[u8] = b"{}";
    let back = run(async {
        let keyring = oo7::Keyring::new().await?;
        let _ = keyring.unlock().await;
        keyring
            .create_item(LABEL, &attributes(), PROBE, true)
            .await?;
        let mut found = false;
        for item in keyring.search_items(&attributes()).await? {
            found |= item.secret().await?.as_bytes() == PROBE;
        }
        keyring.delete(&attributes()).await?;
        Ok(found)
    })?;
    if back {
        Ok(())
    } else {
        Err(StoreError("read back"))
    }
}

/// Forgets the session (sign out): the token and its key leave the keyring.
pub fn clear() -> Result<(), StoreError> {
    run(async {
        let keyring = oo7::Keyring::new().await?;
        keyring.delete(&attributes()).await
    })
}

/// The theme chosen in Account (`ThemePreference`, the iPad's `tafs.theme`): `light`, `dark` or
/// `system`, kept in the user's config folder. Anything else reads as system.
pub fn theme() -> String {
    std::fs::read_to_string(theme_path())
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| s == "light" || s == "dark")
        .unwrap_or_else(|| "system".into())
}

pub fn set_theme(choice: &str) {
    let path = theme_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, choice);
}

fn theme_path() -> std::path::PathBuf {
    gtk::glib::user_config_dir().join("tafs").join("theme")
}

/// The language picked in Account › Language (the iPad's `tafs.language`): one the app ships, kept
/// in the user's config folder. None: this computer's language.
pub fn language() -> Option<&'static str> {
    let kept = std::fs::read_to_string(language_path()).ok()?;
    tafs_core::words::SHIPPED
        .iter()
        .find(|l| **l == kept.trim())
        .copied()
}

/// Picks a language, or this computer's with `None`.
pub fn set_language(code: Option<&str>) {
    let path = language_path();
    match code {
        Some(code) => {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(path, code);
        }
        None => {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn language_path() -> std::path::PathBuf {
    gtk::glib::user_config_dir().join("tafs").join("language")
}

/// Alerts on this computer (Account › Alerts): on unless turned off, kept in the user's config
/// folder.
pub fn alerts_on() -> bool {
    flag("alerts", true)
}

pub fn set_alerts_on(on: bool) {
    set_flag("alerts", on);
}

/// Their sound (`alertSound`): on unless turned off.
pub fn alert_sound() -> bool {
    flag("alert-sound", true)
}

pub fn set_alert_sound(on: bool) {
    set_flag("alert-sound", on);
}

/// Coach mode's width (`coach.width`, `coach.full`), as last chosen; a third of the window at first.
pub fn coach_width() -> tafs_core::coach_mode::Width {
    std::fs::read_to_string(
        gtk::glib::user_config_dir()
            .join("tafs")
            .join("coach-width"),
    )
    .map(|s| tafs_core::coach_mode::Width::read(&s))
    .unwrap_or_default()
}

pub fn set_coach_width(w: &tafs_core::coach_mode::Width) {
    let dir = gtk::glib::user_config_dir().join("tafs");
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join("coach-width"), w.kept());
}

fn flag(name: &str, default: bool) -> bool {
    match std::fs::read_to_string(gtk::glib::user_config_dir().join("tafs").join(name)) {
        Ok(s) if s.trim() == "off" => false,
        Ok(s) if s.trim() == "on" => true,
        _ => default,
    }
}

fn set_flag(name: &str, on: bool) {
    let path = gtk::glib::user_config_dir().join("tafs").join(name);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, if on { "on" } else { "off" });
}
