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
