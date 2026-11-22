//! KIFF's way back after its sign-in page, `dev.kiff.tafs:/oauth?…`, reaches this app through the
//! desktop: a `.desktop` file that handles `x-scheme-handler/dev.kiff.tafs`. A distribution's
//! package installs `packaging/dev.kiff.tafs.desktop`. An AppImage has no install step, so on its
//! first run it writes its own into the person's applications folder, pointing at the AppImage
//! where it is, as AppImage tools do, and asks the desktop to take it for the scheme. Nothing is
//! written outside the person's own folders, and only the AppImage does it.

use std::path::{Path, PathBuf};

/// The entry, for an AppImage at `appimage`: its `Exec` is the AppImage, quoted as the Desktop
/// Entry spec asks, with `%u` for the link.
pub fn entry(appimage: &Path) -> String {
    let quoted = appimage
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('`', "\\`")
        .replace('$', "\\$");
    format!(
        "[Desktop Entry]\nType=Application\nName=tafs\nComment=Your tafs, Needs you and the Record, with KIFF\nExec=\"{quoted}\" %u\nIcon=dev.kiff.tafs\nTerminal=false\nNoDisplay=true\nStartupWMClass=dev.kiff.tafs\nMimeType=x-scheme-handler/dev.kiff.tafs;\nX-tafs-AppImage=true\n"
    )
}

/// Where the entry goes: `$XDG_DATA_HOME/applications`, else `~/.local/share/applications`.
fn applications() -> Option<PathBuf> {
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))?;
    Some(data.join("applications"))
}

/// Run from an AppImage (`$APPIMAGE`): writes the entry when it's missing or points elsewhere,
/// then makes it the scheme's handler. A package's own entry (`dev.kiff.tafs.desktop`, written by
/// no AppImage) is left alone. Best effort: a refusal only means Continue can't come back here,
/// and the welcome's phone way still signs in.
pub fn register_appimage() {
    let Some(appimage) = std::env::var_os("APPIMAGE").map(PathBuf::from) else {
        return;
    };
    let Some(dir) = applications() else {
        return;
    };
    let path = dir.join("dev.kiff.tafs.desktop");
    let wanted = entry(&appimage);
    match std::fs::read_to_string(&path) {
        Ok(now) if now == wanted => return,
        // Installed by a package: not ours to change.
        Ok(now) if !now.contains("X-tafs-AppImage=true") => return,
        _ => {}
    }
    if std::fs::create_dir_all(&dir).is_err() || std::fs::write(&path, wanted).is_err() {
        eprintln!("couldn't register dev.kiff.tafs: links back from KIFF won't reach tafs");
        return;
    }
    let _ = std::process::Command::new("xdg-mime")
        .args([
            "default",
            "dev.kiff.tafs.desktop",
            "x-scheme-handler/dev.kiff.tafs",
        ])
        .status();
    let _ = std::process::Command::new("update-desktop-database")
        .arg(&dir)
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entry_runs_the_appimage_with_the_link() {
        let e = entry(Path::new("/home/rae/Apps/tafs v1 $x.AppImage"));
        assert!(e.contains("Exec=\"/home/rae/Apps/tafs v1 \\$x.AppImage\" %u\n"));
        assert!(e.contains("MimeType=x-scheme-handler/dev.kiff.tafs;\n"));
        assert!(e.contains("X-tafs-AppImage=true"));
    }
}
