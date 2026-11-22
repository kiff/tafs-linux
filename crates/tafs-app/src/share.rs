//! Sharing a taf (`ShareLine`, `ShareTafSheet`): on a taf's page, for whoever manages it, who it's
//! shared with and Share. The shares, groups and people are read as the session; the sheet's two
//! steps (who uses it, then what that means) are drawn whole, and its Share, which the iPad signs
//! once for every change, says this computer can't approve and sends nothing.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use tafs_core::access::Share;
use tafs_core::people::{Group, Person};
use tafs_core::sidebar::Mode;
use tafs_core::words::WordBook;

use crate::widgets::{one_line, symbol, text};

/// Where the shares, groups and people are read.
#[derive(Clone)]
pub enum ShareSource {
    /// KIFF, as the session.
    Cloud {
        token: String,
        languages: Vec<&'static str>,
    },
    /// `--people`'s file: its `groups` and `members`, and each agent's shares under `grants`, as
    /// KIFF answers them. Nothing is read again or sent.
    File(std::path::PathBuf),
}

/// The taf being shared.
#[derive(Clone)]
pub struct ShareTaf {
    pub agent: String,
    pub name: String,
    /// KIFF's manage on it: Share is drawn.
    pub manages: bool,
    /// The account's email, to leave the reader out of the line.
    pub email: Option<String>,
}

/// What a read brought: the taf's shares (none when they couldn't be read), the groups and the
/// people, arranged as People & groups shows them.
#[derive(Clone, Default)]
struct Read {
    shares: Option<Vec<Share>>,
    groups: Vec<Group>,
    people: Vec<Person>,
}

fn read(source: &ShareSource, agent: &str) -> Read {
    use tafs_core::access::{read_shares, shares_path};
    let (shares, groups, people) = match source {
        ShareSource::Cloud { token, languages } => {
            let get = |path: &str| {
                crate::kiff::kiff_get(token, languages, path)
                    .ok()
                    .filter(|a| a.status == 200)
                    .map(|a| a.body)
            };
            (
                get(&shares_path(agent)).and_then(|b| read_shares(&b)),
                get(tafs_core::people::GROUPS_PATH)
                    .and_then(|b| Group::read_all(&b))
                    .unwrap_or_default(),
                get(tafs_core::people::PEOPLE_PATH)
                    .and_then(|b| Person::read_all(&b))
                    .unwrap_or_default(),
            )
        }
        ShareSource::File(path) => {
            let v: serde_json::Value = std::fs::read(path)
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
                .unwrap_or_default();
            let part = |v: &serde_json::Value| serde_json::to_vec(v).unwrap_or_default();
            (
                read_shares(&part(&v["grants"][agent])),
                Group::read_all(&part(&v["groups"])).unwrap_or_default(),
                Person::read_all(&part(&v["members"])).unwrap_or_default(),
            )
        }
    };
    let (groups, people) = tafs_core::people::arrange(groups, people);
    Read {
        shares,
        groups,
        people,
    }
}

/// Read off the main thread, then `then` with what came.
fn read_then(source: &ShareSource, agent: &str, then: impl FnOnce(Read) + 'static) {
    let (source, agent) = (source.clone(), agent.to_string());
    glib::spawn_future_local(async move {
        let r = gio::spawn_blocking(move || read(&source, &agent))
            .await
            .unwrap_or_default();
        then(r);
    });
}

fn line_words(w: &WordBook, taf: &ShareTaf, r: Option<&Read>) -> String {
    let empty = Read::default();
    let r = r.unwrap_or(&empty);
    let you = |s: &str| tafs_core::share::is_you(s, &r.people, taf.email.as_deref());
    let label = |s: &str| tafs_core::share::label(s, &r.people);
    tafs_core::share::shared_with(w, r.shares.as_deref(), &r.groups, &you, &label)
        .unwrap_or_default()
}

/// `ShareLine`: Share (a capsule with the share mark) for whoever manages the taf, and who it's
/// shared with beside it (14, soft): "Sharing…" until read. Read again once the sheet closes.
pub fn line(w: Rc<WordBook>, source: ShareSource, taf: ShareTaf, mode: Mode) -> gtk::Widget {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row.set_margin_top(2);
    row.set_widget_name("share-line");
    let words = one_line(&line_words(&w, &taf, None), 14, 400, &["tafs-soft"]);
    words.set_hexpand(true);
    let refresh: Rc<dyn Fn()> = {
        let (w, source, taf, words) = (w.clone(), source.clone(), taf.clone(), words.clone());
        Rc::new(move || {
            let (w, taf2, words) = (w.clone(), taf.clone(), words.clone());
            read_then(&source, &taf.agent, move |r| {
                words.set_label(&line_words(&w, &taf2, Some(&r)));
            });
        })
    };
    if taf.manages {
        let b = gtk::Button::new();
        b.add_css_class("tafs-share-button");
        let inner = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        inner.append(&symbol("send-to-symbolic", 15));
        let share = w.line("people.share.action").unwrap_or_default();
        inner.append(&one_line(&share, 15, 600, &["tafs-ink"]));
        b.set_child(Some(&inner));
        b.set_widget_name("share-taf");
        let (w, source, taf, refresh) = (w.clone(), source.clone(), taf.clone(), refresh.clone());
        b.connect_clicked(move |b| {
            sheet(
                w.clone(),
                source.clone(),
                taf.clone(),
                mode,
                b,
                refresh.clone(),
            )
        });
        row.append(&b);
    }
    row.append(&words);
    refresh();
    row.upcast()
}

/// The sheet drawn again from its state.
type Draw = Rc<dyn Fn()>;

/// What the sheet holds while it's open.
struct State {
    step: u8,
    loaded: bool,
    had: Vec<Share>,
    chosen: Vec<Share>,
    read: Read,
}

/// `ShareTafSheet`: Share {taf}, in two steps. One: who uses it, each group (its people under it)
/// and each person, picked with a circle; Next once something changed. Two: who gets it and what
/// that means; Share there is signed, so its tap says this computer can't approve and sends
/// nothing. Cancel (Back on step two) at the start.
pub fn sheet(
    w: Rc<WordBook>,
    source: ShareSource,
    taf: ShareTaf,
    mode: Mode,
    parent: &impl IsA<gtk::Widget>,
    closed: Rc<dyn Fn()>,
) {
    let Ok(words) = tafs_core::share::sheet_words(&w, &taf.name) else {
        return;
    };
    let words = Rc::new(words);
    // The whole change, as the iPad names it to Face ID: "Share Support replies".
    let what = w.get("people.share.title", &[("taf", &taf.name)]).ok();
    let header = adw::HeaderBar::new();
    header.set_show_end_title_buttons(false);
    header.set_show_start_title_buttons(false);
    header.set_title_widget(Some(&one_line(&words.title, 15, 600, &["tafs-ink"])));
    let leading = gtk::Button::with_label(&words.cancel);
    leading.add_css_class("tafs-unsent-act");
    leading.set_margin_start(8);
    header.pack_start(&leading);
    let trailing = gtk::Button::with_label(&words.next);
    trailing.add_css_class("tafs-unsent-act");
    trailing.add_css_class("strong");
    trailing.set_margin_end(8);
    trailing.set_widget_name("share.next");
    header.pack_end(&trailing);
    let column = gtk::Box::new(gtk::Orientation::Vertical, 12);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(12);
    column.set_margin_bottom(20);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&column));
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&scroller));
    crate::widgets::ground(&toolbar);
    let dialog = adw::Dialog::new();
    dialog.set_title(&words.title);
    dialog.set_content_width(520);
    dialog.set_content_height(620);
    dialog.set_child(Some(&toolbar));
    dialog.connect_closed(move |_| closed());
    let refused = crate::approval::refusal_slot();

    let state = Rc::new(RefCell::new(State {
        step: 1,
        loaded: false,
        had: vec![],
        chosen: vec![],
        read: Read::default(),
    }));
    let draw: Rc<RefCell<Option<Draw>>> = Rc::default();
    let redraw = {
        let draw = draw.clone();
        move || {
            let d = draw.borrow().clone();
            if let Some(d) = d {
                d();
            }
        }
    };
    *draw.borrow_mut() = Some({
        let (w, words, taf, state, column, leading, trailing, refused) = (
            w.clone(),
            words.clone(),
            taf.clone(),
            state.clone(),
            column.clone(),
            leading.clone(),
            trailing.clone(),
            refused.clone(),
        );
        let redraw = redraw.clone();
        Rc::new(move || {
            while let Some(c) = column.first_child() {
                column.remove(&c);
            }
            let s = state.borrow();
            if s.step == 1 {
                leading.set_label(&words.cancel);
                trailing.set_label(&words.next);
                trailing.set_sensitive(s.loaded && !tafs_core::share::same(&s.chosen, &s.had));
                column.append(&text(&words.step1, 11, 700, &["tafs-dim"]));
                column.append(&text(&words.who_uses, 20, 700, &["tafs-ink"]));
                let pick = |share: Share, label: gtk::Widget| -> gtk::Button {
                    let on = s.chosen.contains(&share);
                    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                    label.set_hexpand(true);
                    row.append(&label);
                    let mark = symbol(
                        if on {
                            "checkbox-checked-symbolic"
                        } else {
                            "radio-symbolic"
                        },
                        20,
                    );
                    mark.add_css_class(if on { "tafs-amber-text" } else { "tafs-dim" });
                    row.append(&mark);
                    row.set_margin_top(10);
                    row.set_margin_bottom(10);
                    let b = gtk::Button::new();
                    b.add_css_class("tafs-plain");
                    b.set_child(Some(&row));
                    if on {
                        b.update_state(&[gtk::accessible::State::Selected(Some(true))]);
                    }
                    let (state, redraw) = (state.clone(), redraw.clone());
                    b.connect_clicked(move |_| {
                        tafs_core::share::toggle(&mut state.borrow_mut().chosen, share.clone());
                        redraw();
                    });
                    b
                };
                if !s.read.groups.is_empty() {
                    column.append(&crate::sheet::header(&words.groups));
                    let g = group_box();
                    for group in &s.read.groups {
                        let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
                        v.append(&one_line(
                            &group.shown_name(&w).unwrap_or_default(),
                            15,
                            600,
                            &["tafs-ink"],
                        ));
                        v.append(&one_line(
                            &tafs_core::share::group_people(&w, group, &s.read.people)
                                .unwrap_or_default(),
                            12,
                            400,
                            &["tafs-dim"],
                        ));
                        g.append(&pick(
                            Share {
                                group: Some(group.id.clone()),
                                subject: None,
                            },
                            v.upcast(),
                        ));
                    }
                    column.append(&g);
                }
                column.append(&crate::sheet::header(&words.one_person));
                let g = group_box();
                for person in &s.read.people {
                    let h = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                    let badge = gtk::Label::new(Some(
                        &person
                            .label()
                            .chars()
                            .next()
                            .map(|c| c.to_uppercase().to_string())
                            .unwrap_or_default(),
                    ));
                    badge.add_css_class("tafs-person-badge");
                    badge.set_size_request(28, 28);
                    badge.set_valign(gtk::Align::Center);
                    badge.set_accessible_role(gtk::AccessibleRole::Presentation);
                    h.append(&badge);
                    h.append(&one_line(person.label(), 15, 400, &["tafs-ink"]));
                    g.append(&pick(
                        Share {
                            group: None,
                            subject: Some(person.subject.clone()),
                        },
                        h.upcast(),
                    ));
                }
                column.append(&g);
                column.append(&crate::sheet::footer(&words.choose_someone));
            } else {
                leading.set_label(&words.back);
                trailing.set_label(&words.share);
                trailing.set_sensitive(true);
                column.append(&text(&words.step2, 11, 700, &["tafs-dim"]));
                column.append(&text(
                    if s.chosen.is_empty() {
                        &words.not_shared
                    } else {
                        &words.who_gets
                    },
                    20,
                    700,
                    &["tafs-ink"],
                ));
                let g = group_box();
                for l in tafs_core::share::consequences(
                    &w,
                    &taf.name,
                    &s.had,
                    &s.chosen,
                    &s.read.groups,
                    &s.read.people,
                )
                .unwrap_or_default()
                {
                    let h = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                    h.set_margin_top(10);
                    h.set_margin_bottom(10);
                    let mark = symbol("emblem-ok-symbolic", 16);
                    mark.set_valign(gtk::Align::Start);
                    h.append(&mark);
                    let line = text(&l, 14, 400, &["tafs-ink"]);
                    line.set_hexpand(true);
                    h.append(&line);
                    g.append(&h);
                }
                column.append(&g);
                if let Some(p) = refused.parent() {
                    if let Ok(b) = p.downcast::<gtk::Box>() {
                        b.remove(&refused);
                    }
                }
                column.append(&refused);
            }
        })
    });
    {
        let (state, dialog, redraw, refused) = (
            state.clone(),
            dialog.downgrade(),
            redraw.clone(),
            refused.clone(),
        );
        leading.connect_clicked(move |_| {
            let step = state.borrow().step;
            if step == 2 {
                state.borrow_mut().step = 1;
                refused.set_visible(false);
                redraw();
            } else if let Some(d) = dialog.upgrade() {
                d.close();
            }
        });
    }
    {
        let (state, redraw, w, refused, agent) = (
            state.clone(),
            redraw.clone(),
            w.clone(),
            refused.clone(),
            taf.agent.clone(),
        );
        let what = what.clone();
        trailing.connect_clicked(move |_| {
            let step = state.borrow().step;
            if step == 1 {
                state.borrow_mut().step = 2;
                redraw();
                return;
            }
            // `save`: one Face ID for every change on the iPad. Here the first of them says this
            // computer can't approve, and none is sent.
            let s = state.borrow();
            if let Some(act) = tafs_core::share::acts(&agent, &s.had, &s.chosen).first() {
                crate::approval::refuse_as(&w, mode, act, what.clone(), &refused);
            }
        });
    }
    redraw();
    dialog.present(Some(parent));
    let (state, redraw2, agent) = (state.clone(), redraw.clone(), taf.agent.clone());
    read_then(&source, &taf.agent, move |r| {
        {
            let mut s = state.borrow_mut();
            if let Some(now) = &r.shares {
                s.had = now.clone();
                s.chosen = now.clone();
            }
            s.read = r;
            s.loaded = true;
        }
        redraw2();
        // `--page share-step2:…`: the second step, as a screenshot shows it.
        if OPEN_STEP2.with(|o| o.take()) {
            let mut s = state.borrow_mut();
            if let Some(first) = s.read.groups.first().map(|g| g.id.clone()) {
                tafs_core::share::toggle(
                    &mut s.chosen,
                    Share {
                        group: Some(first),
                        subject: None,
                    },
                );
            }
            s.step = 2;
            let act = tafs_core::share::acts(&agent, &s.had, &s.chosen)
                .into_iter()
                .next();
            drop(s);
            redraw2();
            // And Share tapped: the can't-approve line, nothing sent.
            if let Some(act) = act {
                crate::approval::refuse_as(&w, mode, &act, what.clone(), &refused);
            }
        }
    });
}

fn group_box() -> gtk::Box {
    let g = gtk::Box::new(gtk::Orientation::Vertical, 0);
    g.add_css_class("tafs-group");
    g
}

thread_local! {
    /// `--page share:<key>`: the sheet opens over the taf's page, for a screenshot.
    pub static OPEN: RefCell<Option<String>> = const { RefCell::new(None) };
    /// `--page share-step2:<key>`: and goes on to its second step, a change picked, and Share
    /// tapped.
    pub static OPEN_STEP2: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
