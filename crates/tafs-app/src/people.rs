//! Account › People & groups (`PeopleGroupsView`): the account's groups, each with its people,
//! managers first, and the tafs it uses. Read again every 20 seconds while it's open. Every change
//! (a group, its name, its people, their role) is the person's act, signed: there, dimmed, until
//! this computer can approve. The lines are `tafs-core/src/people.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use tafs_core::account::Account;
use tafs_core::people::{
    account_people, arrange, managers_first, manages_group, uses, Group, Person, GROUPS_PATH,
    PEOPLE_PATH,
};
use tafs_core::words::WordBook;

use crate::widgets::{one_line, symbol, text};

/// Where People & groups reads.
#[derive(Clone)]
pub enum PeopleSource {
    /// KIFF, as the session.
    Cloud {
        token: String,
        languages: Vec<&'static str>,
    },
    /// A file of synthetic answers (`--people`, for the screenshot): `groups` and `members` as
    /// KIFF answers them, and `account` as `GET /v1/me` does.
    File(std::path::PathBuf),
    /// The sample and signed out: no account, no groups.
    None,
}

/// What the screen knows besides the read: the account, and each KIFF agent's taf by name.
pub struct Known {
    pub account: Option<Account>,
    pub agents: Vec<(String, String)>,
}

/// A read of the account's groups and people, and the account when a file gives it.
type Read = (Vec<Group>, Vec<Person>, Option<Account>);

#[derive(Default)]
struct State {
    groups: Vec<Group>,
    people: Vec<Person>,
    account: Option<Account>,
    error: Option<String>,
    loaded: bool,
}

fn read(w: &WordBook, source: &PeopleSource) -> Result<Read, String> {
    let why = |why: String| w.get("people.not_read", &[("why", &why)]).unwrap_or(why);
    match source {
        PeopleSource::Cloud { token, languages } => {
            let get = |path: &str| -> Result<Vec<u8>, String> {
                let a = crate::kiff::kiff_get(token, languages, path)
                    .map_err(|e| why(e.words(w).unwrap_or_default()))?;
                if !(200..300).contains(&a.status) {
                    return Err(why(tafs_core::signin::refusal(&a, Some(w))
                        .words(w)
                        .unwrap_or_default()));
                }
                Ok(a.body)
            };
            let unreadable = || why(tafs_core::signin::plain_words(w, 200).unwrap_or_default());
            let groups = Group::read_all(&get(GROUPS_PATH)?).ok_or_else(unreadable)?;
            let people = Person::read_all(&get(PEOPLE_PATH)?).ok_or_else(unreadable)?;
            Ok((groups, people, None))
        }
        PeopleSource::File(path) => {
            let v: serde_json::Value = std::fs::read(path)
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
                .unwrap_or_default();
            let part = |k: &str| serde_json::to_vec(&v[k]).unwrap_or_default();
            Ok((
                Group::read_all(&part("groups")).unwrap_or_default(),
                Person::read_all(&part("members")).unwrap_or_default(),
                serde_json::from_value(v["account"].clone()).ok(),
            ))
        }
        PeopleSource::None => Ok((vec![], vec![], None)),
    }
}

pub fn page(
    w: Rc<WordBook>,
    source: PeopleSource,
    known: Rc<Known>,
    back: Rc<dyn Fn()>,
) -> gtk::Widget {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let column = gtk::Box::new(gtk::Orientation::Vertical, 10);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(12);
    column.set_margin_bottom(12);
    let top = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let back_button = gtk::Button::new();
    back_button.set_child(Some(&symbol("go-previous-symbolic", 20)));
    back_button.add_css_class("tafs-round-back");
    back_button.update_property(&[gtk::accessible::Property::Label(&l("settings.title"))]);
    back_button.connect_clicked(move |_| back());
    top.append(&back_button);
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    top.append(&spacer);
    column.append(&top);
    let title = text(&l("people.title"), 34, 700, &["tafs-ink"]);
    title.set_width_chars(12);
    column.append(&title);
    let list = gtk::Box::new(gtk::Orientation::Vertical, 10);
    column.append(&list);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&column));

    let state = Rc::new(RefCell::new(State {
        account: known.account.clone(),
        ..Default::default()
    }));
    let draw: Rc<dyn Fn()> = {
        let (w, list, top, state, known) = (
            w.clone(),
            list.clone(),
            top.clone(),
            state.clone(),
            known.clone(),
        );
        Rc::new(move || {
            while let Some(c) = list.first_child() {
                list.remove(&c);
            }
            // "+ Group" in the bar, for whoever manages people.
            while top
                .last_child()
                .is_some_and(|c| c.has_css_class("tafs-add-group"))
            {
                if let Some(c) = top.last_child() {
                    top.remove(&c);
                }
            }
            let s = state.borrow();
            let l = |k: &str| w.line(k).unwrap_or_default();
            let can_manage = s
                .account
                .as_ref()
                .and_then(|a| a.capabilities.as_ref())
                .is_some_and(|c| c.iter().any(|x| x == "members.manage"));
            if can_manage {
                let add = gtk::Button::with_label(&l("people.add_group"));
                add.add_css_class("tafs-add-group");
                add.add_css_class("tafs-text-button");
                add.update_property(&[gtk::accessible::Property::Label(&l("people.new_group"))]);
                add.set_sensitive(false);
                top.append(&add);
            }
            if let Some(why) = &s.error {
                let p = crate::widgets::panel(false);
                p.append(&text(why, 14, 400, &["tafs-no"]));
                list.append(&p);
            }
            if let Some(a) = &s.account {
                let g = group_box();
                let line = text(
                    &account_people(&w, a, s.people.len()).unwrap_or_default(),
                    14,
                    400,
                    &["tafs-soft"],
                );
                line.set_margin_top(12);
                line.set_margin_bottom(12);
                g.append(&line);
                list.append(&g);
            }
            let me = s.people.iter().find(|p| p.you).map(|p| p.subject.clone());
            for group in &s.groups {
                let name = group.shown_name(&w).unwrap_or_default();
                // The header: its name, the tafs it uses, and its actions.
                let head = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                head.set_margin_start(12);
                head.set_margin_top(10);
                let words = gtk::Box::new(gtk::Orientation::Vertical, 2);
                words.set_hexpand(true);
                words.append(&one_line(&name, 15, 600, &["tafs-ink"]));
                let names: Vec<String> = group
                    .agents
                    .iter()
                    .filter_map(|a| {
                        known
                            .agents
                            .iter()
                            .find(|(id, _)| id == a)
                            .map(|(_, n)| n.clone())
                    })
                    .collect();
                words.append(&text(
                    &uses(&w, group, &names).unwrap_or_default(),
                    12,
                    400,
                    &["tafs-dim"],
                ));
                head.append(&words);
                if can_manage && !group.everyone {
                    let more = gtk::Button::new();
                    more.set_child(Some(&symbol("view-more-symbolic", 17)));
                    more.add_css_class("tafs-plain");
                    more.add_css_class("tafs-soft");
                    more.update_property(&[gtk::accessible::Property::Label(
                        &w.get("people.group_actions", &[("group", &name)])
                            .unwrap_or_default(),
                    )]);
                    more.set_sensitive(false);
                    head.append(&more);
                }
                list.append(&head);
                let g = group_box();
                let mut first = true;
                for m in managers_first(&group.members, &s.people) {
                    if !first {
                        let line = gtk::Separator::new(gtk::Orientation::Horizontal);
                        line.add_css_class("tafs-line");
                        g.append(&line);
                    }
                    first = false;
                    let label = s
                        .people
                        .iter()
                        .find(|p| p.subject == m.subject)
                        .map(|p| p.label().to_string())
                        .unwrap_or_else(|| m.subject.clone());
                    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                    row.set_margin_top(10);
                    row.set_margin_bottom(10);
                    // `PersonBadge`: the label's first letter on a raised circle, 32 across.
                    let badge = gtk::Label::new(Some(
                        &label
                            .chars()
                            .next()
                            .map(|c| c.to_uppercase().to_string())
                            .unwrap_or_default(),
                    ));
                    badge.add_css_class("tafs-person-badge");
                    badge.set_size_request(32, 32);
                    badge.set_valign(gtk::Align::Center);
                    row.append(&badge);
                    let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
                    v.append(&one_line(&label, 15, 400, &["tafs-ink"]));
                    v.append(&one_line(
                        &l(if m.is_manager() {
                            "people.manager"
                        } else {
                            "people.member"
                        }),
                        12,
                        400,
                        &[if m.is_manager() {
                            "tafs-amber-text"
                        } else {
                            "tafs-dim"
                        }],
                    ));
                    row.append(&v);
                    g.append(&row);
                }
                // KIFF's default group has everyone in it: no one is added to it.
                if manages_group(group, can_manage, me.as_deref()) && !group.everyone {
                    if !first {
                        let line = gtk::Separator::new(gtk::Orientation::Horizontal);
                        line.add_css_class("tafs-line");
                        g.append(&line);
                    }
                    // `Label(…, systemImage: "plus")`, 15.
                    let add = gtk::Button::new();
                    let inside = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                    inside.append(&symbol("list-add-symbolic", 15));
                    inside.append(&one_line(
                        &w.get("people.add_someone_to", &[("name", &name)])
                            .unwrap_or_default(),
                        15,
                        400,
                        &[],
                    ));
                    add.set_child(Some(&inside));
                    add.add_css_class("tafs-plain");
                    add.add_css_class("tafs-amber-text");
                    add.set_halign(gtk::Align::Start);
                    add.set_margin_top(6);
                    add.set_margin_bottom(6);
                    add.set_sensitive(false);
                    g.append(&add);
                }
                list.append(&g);
            }
            if s.loaded && s.groups.is_empty() && s.error.is_none() {
                list.append(&text(&l("people.no_groups"), 14, 400, &["tafs-dim"]));
            }
        })
    };
    // Reads now, then every 20 seconds while the screen is open, so someone who joins shows.
    let load: Rc<dyn Fn()> = {
        let (w, source, state, draw) = (w.clone(), source.clone(), state.clone(), draw.clone());
        Rc::new(move || {
            let (words, source) = ((*w).clone(), source.clone());
            let (state, draw) = (state.clone(), draw.clone());
            gtk::glib::spawn_future_local(async move {
                let got = gtk::gio::spawn_blocking(move || read(&words, &source))
                    .await
                    .unwrap_or_else(|_| Err(String::new()));
                {
                    let mut s = state.borrow_mut();
                    match got {
                        Ok((groups, people, account)) => {
                            let (groups, people) = arrange(groups, people);
                            s.groups = groups;
                            s.people = people;
                            if account.is_some() {
                                s.account = account;
                            }
                            s.error = None;
                        }
                        Err(e) => s.error = Some(e),
                    }
                    s.loaded = true;
                }
                draw();
            });
        })
    };
    load();
    {
        let weak = scroller.downgrade();
        let load = load.clone();
        gtk::glib::timeout_add_seconds_local(20, move || match weak.upgrade() {
            Some(s) if s.parent().is_some() => {
                load();
                gtk::glib::ControlFlow::Continue
            }
            _ => gtk::glib::ControlFlow::Break,
        });
    }
    draw();
    crate::widgets::ground(&scroller);
    scroller.upcast()
}

fn group_box() -> gtk::Box {
    let g = gtk::Box::new(gtk::Orientation::Vertical, 0);
    g.add_css_class("tafs-group");
    g
}
