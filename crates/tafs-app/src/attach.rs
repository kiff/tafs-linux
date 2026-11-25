//! Attaching photos and files in a chat (`AttachViews.swift`): the paperclip in the message bar,
//! which chooses photos or files with this computer's file chooser (no camera here); each file
//! made ready as the iPad does (a photo redrawn as a JPEG on white, its long edge at most 2576 px,
//! so nothing but its pixels goes; any other kind checked against its limit first), declared to
//! tafs cloud as the session and its bytes sent straight to storage (`tafs-core/src/attach.rs`);
//! and the tray above the box (`FileTray`, `FileChip`), each file going up, ready, or why not,
//! with its remove control. The files go with the next message.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gdk, gio, glib};
use tafs_core::attach::{self, Files, Pending, Prepared, State};
use tafs_core::coach::Attachment;
use tafs_core::words::WordBook;

use crate::widgets::{one_line, symbol, text};

thread_local! {
    /// `--attach files.json`: the first chat's tray, as a screenshot draws it.
    pub static SEED: RefCell<Option<std::path::PathBuf>> = const { RefCell::new(None) };
}

/// Where a chat's files go: the session, and the taf at tafs cloud (Coach's, or the chat's own).
#[derive(Clone)]
pub struct Target {
    pub token: String,
    pub languages: Vec<&'static str>,
    pub taf: String,
}

/// One chat's paperclip, tray and files.
pub struct Attach {
    w: Rc<WordBook>,
    /// None: a screenshot's, which sends nothing, or a chat whose taf isn't read yet.
    target: RefCell<Option<Target>>,
    pub files: RefCell<Files>,
    pub button: gtk::MenuButton,
    pub tray: gtk::Box,
    /// The composer's state follows the files.
    changed: RefCell<Option<Rc<dyn Fn()>>>,
}

impl Attach {
    pub fn new(w: Rc<WordBook>, target: Option<Target>) -> Rc<Attach> {
        let button = gtk::MenuButton::new();
        button.add_css_class("tafs-attach");
        button.set_icon_name("mail-attachment-symbolic");
        button.set_widget_name("chat.attach");
        button.set_valign(gtk::Align::Center);
        let tray = gtk::Box::new(gtk::Orientation::Vertical, 6);
        tray.set_margin_start(12);
        tray.set_margin_end(12);
        tray.set_margin_top(8);
        tray.set_vexpand(false);
        tray.set_visible(false);
        let a = Rc::new(Attach {
            w,
            target: RefCell::new(target),
            files: RefCell::new(Files::default()),
            button,
            tray,
            changed: RefCell::new(None),
        });
        // Choose photos, or choose a file: there's no camera to take one with here.
        let menu = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let popover = gtk::Popover::new();
        for (key, photos) in [
            ("chat.file.choose_photos", true),
            ("chat.file.choose_file", false),
        ] {
            let b = gtk::Button::new();
            b.add_css_class("flat");
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            row.append(&symbol(
                if photos {
                    "image-x-generic-symbolic"
                } else {
                    "text-x-generic-symbolic"
                },
                16,
            ));
            row.append(&gtk::Label::new(Some(&a.w.line(key).unwrap_or_default())));
            b.set_child(Some(&row));
            let (p, popover2) = (Rc::downgrade(&a), popover.clone());
            b.connect_clicked(move |_| {
                popover2.popdown();
                if let Some(p) = p.upgrade() {
                    p.choose(photos);
                }
            });
            menu.append(&b);
        }
        popover.set_child(Some(&menu));
        a.button.set_popover(Some(&popover));
        a.draw();
        // A screenshot's tray, once the composer follows the files.
        if let Some(path) = SEED.with(|s| s.borrow_mut().take()) {
            let a = a.clone();
            glib::idle_add_local_once(move || a.seed(&path));
        }
        a
    }

    /// Where the files go, once the chat's taf is known.
    pub fn set_target(&self, target: Option<Target>) {
        *self.target.borrow_mut() = target;
    }

    /// What follows a change of the files: the composer's Send.
    pub fn on_change(&self, f: Rc<dyn Fn()>) {
        *self.changed.borrow_mut() = Some(f);
    }

    fn changed(self: &Rc<Self>) {
        self.draw();
        let f = self.changed.borrow().clone();
        if let Some(f) = f {
            f();
        }
    }

    /// `fileImporter` / `photosPicker`: this computer's file chooser, several at once, up to the
    /// room left in the message.
    fn choose(self: &Rc<Self>, photos: bool) {
        let filter = gtk::FileFilter::new();
        if photos {
            filter.add_mime_type("image/*");
        } else {
            for t in attach::PICKABLE {
                filter.add_mime_type(t);
            }
        }
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        let dialog = gtk::FileDialog::new();
        dialog.set_modal(true);
        dialog.set_filters(Some(&filters));
        dialog.set_default_filter(Some(&filter));
        let window = self.button.root().and_downcast::<gtk::Window>();
        let p = self.clone();
        dialog.open_multiple(window.as_ref(), gio::Cancellable::NONE, move |picked| {
            let Ok(list) = picked else { return };
            let room = p.files.borrow().room();
            let files: Vec<gio::File> = (0..list.n_items())
                .filter_map(|i| list.item(i).and_downcast::<gio::File>())
                .take(room.max(1))
                .collect();
            for (n, file) in files.into_iter().enumerate() {
                let p = p.clone();
                glib::spawn_future_local(async move { p.add_file(file, photos, n).await });
            }
        });
    }

    /// One file picked: read, made ready, added to the tray and sent up.
    async fn add_file(self: &Rc<Self>, file: gio::File, photo: bool, n: usize) {
        let w = &self.w;
        let name = file
            .basename()
            .map(|b| b.to_string_lossy().into_owned())
            .unwrap_or_default();
        let info = file
            .query_info_future(
                "standard::size,standard::content-type",
                gio::FileQueryInfoFlags::NONE,
                glib::Priority::DEFAULT,
            )
            .await;
        let (size, content_type) = match &info {
            Ok(i) => (
                u64::try_from(i.size()).unwrap_or(0),
                i.content_type()
                    .and_then(|t| gio::content_type_get_mime_type(&t))
                    .map(|m| m.to_string())
                    .unwrap_or_default(),
            ),
            Err(_) => (0, String::new()),
        };
        // Over every limit, or of a kind tafs doesn't take: refused before it's read.
        match attach::before_reading(w, &name, size, &content_type) {
            Ok(Some(why)) => return self.refuse(why),
            Err(_) => return,
            Ok(None) => {}
        }
        let Ok((bytes, _)) = file.load_contents_future().await else {
            let why = if photo {
                w.line("attachment.photo_could_not_read")
            } else {
                w.get("chat.attach.unreadable", &[("name", &name)])
            };
            return self.refuse(why.unwrap_or_default());
        };
        let prepared = if attach::is_image(&content_type) {
            let shown = if photo && name.is_empty() {
                w.get("attachment.photo_numbered", &[("n", &(n + 1).to_string())])
                    .unwrap_or_default()
            } else {
                name.clone()
            };
            photo_of(w, &bytes, &shown)
        } else {
            let mt = attach::media_type(&content_type).unwrap_or_default();
            match attach::check(w, &name, bytes.len() as u64, mt) {
                Ok(None) => Ok(Prepared {
                    name: name.clone(),
                    media_type: mt.to_string(),
                    data: bytes.to_vec(),
                    thumbnail: None,
                }),
                Ok(Some(why)) => Err(why),
                Err(_) => return,
            }
        };
        match prepared {
            Ok(p) => self.attach(p).await,
            Err(why) => self.refuse(why),
        }
    }

    /// `refuseFile`: said in the tray, never uploaded.
    fn refuse(self: &Rc<Self>, why: String) {
        self.files.borrow_mut().refuse(why);
        self.changed();
    }

    /// `attach`: in the tray at once, going up; declared to tafs cloud, then its bytes to
    /// storage. One that can't go says why, and is left out of the message.
    async fn attach(self: &Rc<Self>, file: Prepared) {
        let id = match self.files.borrow_mut().add(&self.w, &file) {
            Ok(Some(id)) => id,
            _ => return self.changed(),
        };
        self.changed();
        let Some(t) = self.target.borrow().clone() else {
            // A screenshot's: nothing is sent, so it stays going up.
            return;
        };
        let w = (*self.w).clone();
        let state = gio::spawn_blocking(move || upload(&w, &t, &file))
            .await
            .unwrap_or_else(|_| State::Failed(String::new()));
        self.files.borrow_mut().settle(id, state);
        self.changed();
    }

    /// `openFile`: a file sent in this chat, read from tafs cloud (only the person's own), saved
    /// in this app's cache under its name with its kind's extension, and opened by the desktop's
    /// app for it, as the iPad's Quick Look. One that can't be read says why, in the tray.
    pub fn open_sent(self: &Rc<Self>, file: &Attachment, chip: &gtk::Button) {
        let Some(t) = self.target.borrow().clone() else {
            return;
        };
        chip.set_sensitive(false);
        let (me, file, chip) = (self.clone(), file.clone(), chip.clone());
        glib::spawn_future_local(async move {
            let words = (*me.w).clone();
            let f = file.clone();
            let saved = gio::spawn_blocking(move || -> Result<std::path::PathBuf, String> {
                let why = |e: tafs_core::signin::KiffError| e.words(&words).unwrap_or_default();
                let a =
                    crate::kiff::cloud(&t.token, &t.languages, &attach::file_path(&t.taf, &f.id))
                        .map_err(why)?;
                let url = (a.status == 200)
                    .then(|| attach::file_url(&a.body))
                    .flatten()
                    .ok_or_else(|| words.line("chat.file.could_not_open").unwrap_or_default())?;
                let d = crate::kiff::download(&url).map_err(why)?;
                if !(200..300).contains(&d.status) {
                    return Err(words.line("chat.file.could_not_open").unwrap_or_default());
                }
                // One folder per file, named by its id's letters, digits, `-` and `_` only.
                let id: String =
                    f.id.chars()
                        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
                        .collect();
                if id.is_empty() {
                    return Err(words.line("chat.file.could_not_open").unwrap_or_default());
                }
                let dir = opened_dir().join(id);
                private_dir(&opened_dir())
                    .and_then(|()| private_dir(&dir))
                    .map_err(|e| e.to_string())?;
                let path = dir.join(attach::opened_name(&f.name, &f.media_type));
                std::fs::write(&path, &d.body).map_err(|e| e.to_string())?;
                Ok(path)
            })
            .await
            .unwrap_or_else(|_| Err(String::new()));
            chip.set_sensitive(true);
            match saved {
                Ok(path) => {
                    let window = chip.root().and_downcast::<gtk::Window>();
                    gtk::FileLauncher::new(Some(&gio::File::for_path(&path))).launch(
                        window.as_ref(),
                        gio::Cancellable::NONE,
                        |_| {},
                    );
                }
                Err(why) => {
                    let words =
                        me.w.get(
                            "chat.file.not_opened",
                            &[("file", &file.name), ("why", &why)],
                        )
                        .unwrap_or_default();
                    me.refuse(words);
                }
            }
        });
    }

    /// What Send does with the files: whether some are ready to go, and, while Send waits for
    /// them (`holdsSend`), what it says.
    pub fn send_state(&self, send: &str, thinking: bool) -> (bool, Option<String>) {
        let files = self.files.borrow();
        let held = files.holds_send(thinking).then(|| {
            if files.uploading() {
                self.w.line("chat.send_once_files_up").unwrap_or_default()
            } else {
                send.to_string()
            }
        });
        (files.ready(), held)
    }

    /// Into a composer: the paperclip at the start of its row, the tray above its box.
    pub fn place(&self, composer: &crate::chat::Composer) {
        composer.row.prepend(&self.button);
        composer.bar.prepend(&self.tray);
    }

    /// The ready files, taken for the message being sent.
    pub fn take(self: &Rc<Self>) -> Vec<Pending> {
        let taken = self.files.borrow_mut().take();
        self.changed();
        taken
    }

    /// The files a message didn't take, back in the tray.
    pub fn put_back(self: &Rc<Self>, taken: Vec<Pending>) {
        self.files.borrow_mut().put_back(taken);
        self.changed();
    }

    /// The paperclip's state and the tray (`FileTray`): each file's chip, then why one wasn't
    /// added.
    fn draw(self: &Rc<Self>) {
        while let Some(c) = self.tray.first_child() {
            self.tray.remove(&c);
        }
        let files = self.files.borrow();
        let room = files.room();
        let label = if room == 0 {
            self.w
                .count("chat.file.attach_full", attach::PER_MESSAGE as i64, &[])
                .unwrap_or_default()
        } else {
            self.w.line("chat.file.attach").unwrap_or_default()
        };
        self.button.set_tooltip_text(Some(&label));
        self.button
            .update_property(&[gtk::accessible::Property::Label(&label)]);
        self.button.set_sensitive(room > 0);
        if !files.files.is_empty() {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            for f in &files.files {
                row.append(&self.chip(f));
            }
            let scroll = gtk::ScrolledWindow::new();
            scroll.set_vscrollbar_policy(gtk::PolicyType::Never);
            scroll.set_propagate_natural_height(true);
            // As tall as its chips: no taller, so the thread above keeps its room.
            scroll.set_vexpand(false);
            scroll.set_child(Some(&row));
            self.tray.append(&scroll);
        }
        if let Some(n) = &files.notice {
            self.tray.append(&text(n, 13, 400, &["tafs-no"]));
        }
        self.tray
            .set_visible(!files.files.is_empty() || files.notice.is_some());
    }

    /// `FileChip`: its picture, its name, its state, and remove.
    fn chip(self: &Rc<Self>, f: &Pending) -> gtk::Widget {
        let chip = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        chip.add_css_class("tafs-file-chip");
        chip.append(&file_icon(&f.media_type, f.thumbnail.as_deref()));
        let words = gtk::Box::new(gtk::Orientation::Vertical, 2);
        words.set_valign(gtk::Align::Center);
        words.set_size_request(-1, -1);
        let name = one_line(&f.name, 13, 600, &["tafs-ink"]);
        name.set_max_width_chars(22);
        words.append(&name);
        let state = attach::state_words(&self.w, &f.state).unwrap_or_default();
        match &f.state {
            State::Uploading => {
                let h = gtk::Box::new(gtk::Orientation::Horizontal, 4);
                let spin = gtk::Spinner::new();
                spin.set_spinning(true);
                spin.set_size_request(10, 10);
                h.append(&spin);
                h.append(&text(&state, 12, 400, &["tafs-dim"]));
                words.append(&h);
            }
            State::Ready(_) => words.append(&text(&state, 12, 400, &["tafs-dim"])),
            State::Failed(_) => {
                let l = text(&state, 12, 400, &["tafs-no"]);
                l.set_lines(2);
                l.set_ellipsize(gtk::pango::EllipsizeMode::End);
                l.set_max_width_chars(22);
                words.append(&l);
            }
        }
        chip.append(&words);
        let remove = gtk::Button::new();
        remove.add_css_class("tafs-plain");
        remove.set_child(Some(&symbol("window-close-symbolic", 11)));
        remove.set_size_request(28, 28);
        let words = self
            .w
            .get("chat.file.remove", &[("name", &f.name)])
            .unwrap_or_default();
        remove.update_property(&[gtk::accessible::Property::Label(&words)]);
        let (p, id) = (Rc::downgrade(self), f.id);
        remove.connect_clicked(move |_| {
            if let Some(p) = p.upgrade() {
                p.files.borrow_mut().remove(id);
                p.changed();
            }
        });
        chip.append(&remove);
        chip.upcast()
    }

    /// A screenshot's tray, from a file: each `{name, media_type, state}`, where `state` is
    /// "uploading", "ready" or "failed". Nothing is read or sent.
    pub fn seed(self: &Rc<Self>, path: &std::path::Path) {
        let Some(list) = std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
            .and_then(|v| v.as_array().cloned())
        else {
            eprintln!("{} isn't a list of files", path.display());
            std::process::exit(2);
        };
        {
            let mut files = self.files.borrow_mut();
            for f in list {
                let s = |k: &str| {
                    f.get(k)
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string()
                };
                let p = Prepared {
                    name: s("name"),
                    media_type: s("media_type"),
                    data: vec![],
                    thumbnail: None,
                };
                if let Ok(Some(id)) = files.add(&self.w, &p) {
                    let state = match s("state").as_str() {
                        "uploading" => State::Uploading,
                        "ready" => State::Ready(Attachment {
                            id: format!("att_{id}"),
                            kind: String::new(),
                            media_type: p.media_type.clone(),
                            name: p.name.clone(),
                            size: 0,
                        }),
                        // Why, in the app's words, as an upload that failed says it.
                        _ => State::Failed(
                            self.w.line("error.file_not_uploaded").unwrap_or_default(),
                        ),
                    };
                    files.settle(id, state);
                }
            }
        }
        self.changed();
    }
}

/// `FileIcon`: a photo's thumbnail, else an icon for its kind, 36 points on a soft square.
pub fn file_icon(media_type: &str, thumbnail: Option<&[u8]>) -> gtk::Widget {
    if let Some(t) = thumbnail.and_then(|b| gdk::Texture::from_bytes(&glib::Bytes::from(b)).ok()) {
        let p = gtk::Picture::for_paintable(&t);
        p.set_content_fit(gtk::ContentFit::Cover);
        p.set_size_request(36, 36);
        p.add_css_class("tafs-file-icon");
        p.set_can_shrink(true);
        return p.upcast();
    }
    let b = gtk::Box::new(gtk::Orientation::Vertical, 0);
    b.add_css_class("tafs-file-icon");
    b.set_size_request(36, 36);
    let i = symbol(attach::icon(media_type), 17);
    i.set_vexpand(true);
    i.set_valign(gtk::Align::Center);
    b.append(&i);
    b.upcast()
}

/// `FilePrep.photo`: any picture this computer reads, redrawn on white as a JPEG at 85, its long
/// edge at most 2576 px, turned as it was taken; with a small thumbnail for the tray. Only its
/// pixels go: no location or other metadata.
fn photo_of(w: &WordBook, bytes: &[u8], name: &str) -> Result<Prepared, String> {
    let not_a_photo = || {
        w.get("chat.attach.not_a_photo", &[("name", name)])
            .unwrap_or_default()
    };
    let stream = gio::MemoryInputStream::from_bytes(&glib::Bytes::from(bytes));
    let pixbuf = gtk::gdk_pixbuf::Pixbuf::from_stream(&stream, gio::Cancellable::NONE)
        .map_err(|_| not_a_photo())?;
    let pixbuf = pixbuf.apply_embedded_orientation().unwrap_or(pixbuf);
    let jpeg = |edge: i32, quality: &str| -> Option<Vec<u8>> {
        let (tw, th) = attach::scaled(pixbuf.width(), pixbuf.height(), edge);
        let white =
            gtk::gdk_pixbuf::Pixbuf::new(gtk::gdk_pixbuf::Colorspace::Rgb, false, 8, tw, th)?;
        white.fill(0xffff_ffff);
        pixbuf.composite(
            &white,
            0,
            0,
            tw,
            th,
            0.0,
            0.0,
            f64::from(tw) / f64::from(pixbuf.width()),
            f64::from(th) / f64::from(pixbuf.height()),
            gtk::gdk_pixbuf::InterpType::Bilinear,
            255,
        );
        white.save_to_bufferv("jpeg", &[("quality", quality)]).ok()
    };
    let data = jpeg(attach::LONG_EDGE, "85").ok_or_else(not_a_photo)?;
    let named = attach::photo_name(w, name).map_err(|_| not_a_photo())?;
    match attach::check(w, &named, data.len() as u64, "image/jpeg") {
        Ok(None) => {}
        Ok(Some(why)) => return Err(why),
        Err(_) => return Err(not_a_photo()),
    }
    Ok(Prepared {
        name: named,
        media_type: "image/jpeg".into(),
        data,
        thumbnail: jpeg(attach::THUMBNAIL_EDGE, "70"),
    })
}

/// The declaration at tafs cloud as the session, then the bytes to storage: ready, or why not.
fn upload(w: &WordBook, t: &Target, file: &Prepared) -> State {
    // As the iPad says it: the error's own words.
    let failed = State::Failed;
    let declared = crate::kiff::cloud_send(
        &t.token,
        &t.languages,
        "POST",
        &attach::attach_path(&t.taf),
        Some(&attach::attach_body(file)),
    );
    let a = match declared {
        Ok(a) => a,
        Err(e) => return failed(e.words(w).unwrap_or_default()),
    };
    if !(200..300).contains(&a.status) {
        // A file over its limit is refused here, before any upload, and the refusal says which.
        return State::Failed(
            tafs_core::coach_send::Refusal::read(a.status, &a.body)
                .words(w)
                .unwrap_or_default(),
        );
    }
    let Some((attachment, to)) = attach::attached(&a.body) else {
        return failed(tafs_core::signin::plain_words(w, 200).unwrap_or_default());
    };
    match crate::kiff::upload(&to, &file.data) {
        Ok(status) if (200..300).contains(&status) => State::Ready(attachment),
        _ => State::Failed(w.line("error.file_not_uploaded").unwrap_or_default()),
    }
}

/// Where a file opened from a chat is written for the desktop to open: one folder per file, in the
/// user's cache, readable by this user alone.
pub fn opened_dir() -> std::path::PathBuf {
    glib::user_cache_dir().join("tafs").join("opened")
}

/// The files opened from a chat go: at sign-out, when KIFF ends the session, and at a start with no
/// session, since a crash or a kill before sign-out could leave them. A person's own files never stay
/// on this computer after them, as Windows' and the iPad's `removeOpenedFiles`.
pub fn clear_opened() {
    clear_opened_in(&opened_dir());
}

fn clear_opened_in(dir: &std::path::Path) {
    if let Err(e) = std::fs::remove_dir_all(dir) {
        if e.kind() != std::io::ErrorKind::NotFound {
            eprintln!("the files opened from a chat stayed: {:?}", e.kind());
        }
    }
}

/// A folder only this user may read (0700), made if it's missing and narrowed if it's wider.
fn private_dir(dir: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
}

#[cfg(test)]
mod opened_tests {
    use super::{clear_opened_in, private_dir};
    use std::os::unix::fs::PermissionsExt;

    fn temp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("tafs-opened-{name}-{}", std::process::id()))
    }

    #[test]
    fn an_opened_files_folder_is_this_users_alone() {
        let root = temp("private");
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join("opened").join("file_1");
        private_dir(&dir).unwrap();
        assert_eq!(
            std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
        // One that was there, wider, is narrowed.
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        private_dir(&dir).unwrap();
        assert_eq!(
            std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_opened_files_go_and_none_is_no_error() {
        let root = temp("clear");
        let dir = root.join("opened");
        private_dir(&dir.join("file_1")).unwrap();
        std::fs::write(dir.join("file_1").join("invoice.pdf"), b"%PDF").unwrap();
        clear_opened_in(&dir);
        assert!(!dir.exists());
        clear_opened_in(&dir);
        std::fs::remove_dir_all(&root).unwrap();
    }
}
