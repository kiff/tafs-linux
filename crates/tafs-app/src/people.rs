//! Account › People & groups (`PeopleGroupsView`): the account's groups, each with its people,
//! managers first, and the tafs it uses. Read again every 20 seconds while it's open. Every change
//! (a group, its name, its people, their role) is the person's act, signed: this computer only
//! views, so each is drawn as the iPad's, with its sheet (`GroupNameSheet`, `AddToGroupSheet`) or
//! question, up to the signature, and the tap that would sign says this computer can't approve.
//! Nothing is sent. The lines are `tafs-core/src/people.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use tafs_core::account::Account;
use tafs_core::people::{
    account_people, add_act, add_sheet, arrange, delete_question, group_menu, managers_first,
    manages_group, member_menu, name_act, name_sheet, outside, uses, Group, Person, GROUPS_PATH,
    PEOPLE_PATH,
};
use tafs_core::sidebar::Mode;
use tafs_core::signed::Act;
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

thread_local! {
    /// `--page people-group-name` or `people-add`: that sheet opens once the groups are read,
    /// for its screenshot.
    pub static OPEN: std::cell::Cell<Option<&'static str>> = const { std::cell::Cell::new(None) };
}

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
    // Signed in with KIFF, or a screenshot's account: the iPad's `changePeople` signs only there.
    let mode = match source {
        PeopleSource::None => Mode::SignedOut,
        _ => Mode::Kiff,
    };
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
    // Where the iPad says why a change didn't go (`peopleError`): here, that this computer can't
    // approve, once a tap would have signed.
    let refused = crate::approval::refusal_slot();
    let refused_panel = crate::widgets::panel(false);
    refused_panel.append(&refused);
    refused_panel.set_visible(false);
    column.append(&refused_panel);
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
    // The tap that would sign: the words where the iPad's error goes, and nothing sent.
    let refuse: Rc<dyn Fn(Act)> = {
        let (w, refused, refused_panel) = (w.clone(), refused.clone(), refused_panel.clone());
        Rc::new(move |act: Act| {
            crate::approval::refuse(&w, mode, &act, &refused);
            refused_panel.set_visible(refused.is_visible());
        })
    };
    let draw: Rc<dyn Fn()> = {
        let (w, list, top, state, known, refuse) = (
            w.clone(),
            list.clone(),
            top.clone(),
            state.clone(),
            known.clone(),
            refuse.clone(),
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
                add.add_css_class("tafs-amber-text");
                add.set_widget_name("people.add-group");
                add.update_property(&[gtk::accessible::Property::Label(&l("people.new_group"))]);
                let w = w.clone();
                add.connect_clicked(move |b| name_group(&w, mode, None, b, false));
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
                let manages = manages_group(group, can_manage, me.as_deref());
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
                if let Ok(Some((rename, delete))) = group_menu(&w, group, can_manage) {
                    let more = gtk::MenuButton::new();
                    more.set_child(Some(&symbol("view-more-symbolic", 17)));
                    more.add_css_class("flat");
                    more.add_css_class("tafs-soft");
                    more.set_widget_name(&format!("people.group-actions.{}", group.id));
                    more.update_property(&[gtk::accessible::Property::Label(
                        &w.get("people.group_actions", &[("group", &name)])
                            .unwrap_or_default(),
                    )]);
                    let popover = gtk::Popover::new();
                    let items = gtk::Box::new(gtk::Orientation::Vertical, 0);
                    let renaming = menu_item(&rename, false);
                    let deleting = menu_item(&delete, true);
                    items.append(&renaming);
                    items.append(&deleting);
                    popover.set_child(Some(&items));
                    more.set_popover(Some(&popover));
                    {
                        let (w, group, p) = (w.clone(), group.clone(), popover.clone());
                        renaming.connect_clicked(move |b| {
                            p.popdown();
                            name_group(&w, mode, Some(group.clone()), b, false);
                        });
                    }
                    {
                        let (w, refuse, group, p, more) = (
                            w.clone(),
                            refuse.clone(),
                            group.clone(),
                            popover.clone(),
                            more.clone(),
                        );
                        deleting.connect_clicked(move |_| {
                            p.popdown();
                            delete_group(&w, &group, &more, refuse.clone());
                        });
                    }
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
                    let row = person_row(&label);
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
                    // The iPad's context menu (and its swipe, Remove): a right click or a long
                    // press here.
                    if let Ok(menu) = member_menu(&w, m, group, can_manage, manages) {
                        let items: Vec<(String, Act, bool)> = menu
                            .role
                            .into_iter()
                            .map(|(words, act)| (words, act, false))
                            .chain(menu.remove.map(|(words, act)| (words, act, true)))
                            .collect();
                        if !items.is_empty() {
                            menu_on(&row, items, refuse.clone());
                        }
                    }
                    g.append(&row);
                }
                // KIFF's default group has everyone in it: no one is added to it.
                if manages && !group.everyone {
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
                    add.set_widget_name(&format!("people.add.{}", group.id));
                    add.set_halign(gtk::Align::Start);
                    add.set_margin_top(6);
                    add.set_margin_bottom(6);
                    let (w, group, people) = (w.clone(), group.clone(), s.people.clone());
                    add.connect_clicked(move |b| {
                        add_to_group(&w, mode, &group, &people, can_manage, b)
                    });
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
        let (w, source, state, draw, list) = (
            w.clone(),
            source.clone(),
            state.clone(),
            draw.clone(),
            list.clone(),
        );
        Rc::new(move || {
            let (words, source) = ((*w).clone(), source.clone());
            let (w, state, draw, list) = (w.clone(), state.clone(), draw.clone(), list.clone());
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
                // A screenshot's sheet, over the first group that takes one: renaming it, with
                // Save tapped, or adding someone to it.
                if let Some(which) = OPEN.with(|o| o.take()) {
                    let s = state.borrow();
                    let Some(group) = s.groups.iter().find(|g| !g.everyone).cloned() else {
                        return;
                    };
                    let can_manage = s
                        .account
                        .as_ref()
                        .is_some_and(|a| a.allows("members.manage"));
                    let (w, list, people) = (w.clone(), list.clone(), s.people.clone());
                    gtk::glib::timeout_add_local_once(
                        std::time::Duration::from_millis(300),
                        move || {
                            if which == "add" {
                                add_to_group(&w, mode, &group, &people, can_manage, &list);
                            } else {
                                name_group(&w, mode, Some(group), &list, true);
                            }
                        },
                    );
                }
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

/// A person's row: `PersonBadge`, the label's first letter on a raised circle, 32 across, and
/// what comes after it.
fn person_row(label: &str) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.set_margin_top(10);
    row.set_margin_bottom(10);
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
    badge.set_accessible_role(gtk::AccessibleRole::Presentation);
    row.append(&badge);
    row
}

/// A menu's item: its words, in red when it's destructive.
fn menu_item(words: &str, destructive: bool) -> gtk::Button {
    let b = gtk::Button::with_label(words);
    b.add_css_class("flat");
    if let Some(c) = b.child() {
        c.set_halign(gtk::Align::Start);
    }
    if destructive {
        b.add_css_class("tafs-no");
    }
    b
}

/// A person's menu in a group, on a right click or a long press: each item's tap would sign.
fn menu_on(row: &gtk::Box, items: Vec<(String, Act, bool)>, refuse: Rc<dyn Fn(Act)>) {
    let items = Rc::new(items);
    let open = {
        let row = row.clone();
        Rc::new(move |x: f64, y: f64| {
            let list = gtk::Box::new(gtk::Orientation::Vertical, 0);
            let popover = gtk::Popover::new();
            for (words, act, destructive) in items.iter() {
                let b = menu_item(words, *destructive);
                let (act, p, refuse) = (act.clone(), popover.clone(), refuse.clone());
                b.connect_clicked(move |_| {
                    p.popdown();
                    refuse(act.clone());
                });
                list.append(&b);
            }
            popover.set_child(Some(&list));
            popover.set_parent(&row);
            popover.set_has_arrow(true);
            popover.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
            popover.connect_closed(|p| {
                let p = p.clone();
                gtk::glib::idle_add_local_once(move || p.unparent());
            });
            popover.popup();
        })
    };
    let click = gtk::GestureClick::new();
    click.set_button(gtk::gdk::BUTTON_SECONDARY);
    {
        let open = open.clone();
        click.connect_pressed(move |g, _, x, y| {
            g.set_state(gtk::EventSequenceState::Claimed);
            open(x, y);
        });
    }
    row.add_controller(click);
    let press = gtk::GestureLongPress::new();
    press.connect_pressed(move |_, x, y| open(x, y));
    row.add_controller(press);
}

/// `GroupNameSheet`: a new group's name, or a group's new one; Create or Save once a name is
/// typed. Its tap would sign: the sheet stays open and says this computer can't approve.
fn name_group(
    w: &Rc<WordBook>,
    mode: Mode,
    group: Option<Group>,
    over: &impl IsA<gtk::Widget>,
    tap_now: bool,
) {
    let Ok(lines) = name_sheet(w, group.as_ref()) else {
        return;
    };
    let sheet = crate::sheet::new(&lines.title, &lines.cancel, Some(&lines.confirm), 480);
    let confirm = sheet.confirm.clone().expect("the sheet's confirm");
    confirm.set_widget_name("people.name-confirm");
    let field = gtk::Entry::new();
    field.set_placeholder_text(Some(&lines.hint));
    field.add_css_class("tafs-field");
    field.set_widget_name("people.group-name");
    field.set_text(group.as_ref().map(|g| g.name.as_str()).unwrap_or(""));
    let g = group_box();
    field.set_margin_top(8);
    field.set_margin_bottom(8);
    g.append(&field);
    sheet.column.append(&g);
    sheet.column.append(&crate::sheet::footer(&lines.note));
    let refused = crate::approval::refusal_slot();
    refused.set_margin_start(16);
    refused.set_margin_end(16);
    sheet.column.append(&refused);
    let save: Rc<dyn Fn()> = {
        let (w, field, refused, group) = (w.clone(), field.clone(), refused.clone(), group.clone());
        Rc::new(move || {
            if let Some(act) = name_act(group.as_ref(), &field.text()) {
                crate::approval::refuse(&w, mode, &act, &refused);
            }
        })
    };
    {
        let (confirm, group) = (confirm.clone(), group.clone());
        let refresh = move |f: &gtk::Entry| {
            confirm.set_sensitive(name_act(group.as_ref(), &f.text()).is_some());
        };
        refresh(&field);
        field.connect_changed(refresh);
    }
    {
        let save = save.clone();
        confirm.connect_clicked(move |_| save());
    }
    field.connect_activate(move |_| save());
    sheet.dialog.present(Some(over));
    // A screenshot's sheet: Save tapped.
    if tap_now {
        confirm.emit_clicked();
    }
}

/// The question before a group goes: its title, what it does, and Delete in red. Its tap would
/// sign.
fn delete_group(
    w: &WordBook,
    group: &Group,
    over: &impl IsA<gtk::Widget>,
    refuse: Rc<dyn Fn(Act)>,
) {
    let Ok(q) = delete_question(w, group) else {
        return;
    };
    let ask = adw::AlertDialog::new(Some(&q.title), Some(&q.body));
    ask.add_response("cancel", &q.cancel);
    ask.add_response("delete", &q.button);
    ask.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
    ask.set_default_response(Some("cancel"));
    ask.set_close_response("cancel");
    let id = group.id.clone();
    ask.connect_response(None, move |_, response| {
        if response == "delete" {
            refuse(Act::DeleteGroup { group: id.clone() });
        }
    });
    ask.present(Some(over));
}

/// `AddToGroupSheet`: As the group's manager (for whoever manages people) with what a manager
/// does, then the account's people not in it yet; a tap on one would sign, so the sheet stays
/// open and says this computer can't approve.
fn add_to_group(
    w: &Rc<WordBook>,
    mode: Mode,
    group: &Group,
    people: &[Person],
    can_manage: bool,
    over: &impl IsA<gtk::Widget>,
) {
    let Ok(lines) = add_sheet(w, group, can_manage) else {
        return;
    };
    let sheet = crate::sheet::new(&lines.title, &lines.cancel, None, 480);
    let as_manager = gtk::Switch::new();
    as_manager.set_valign(gtk::Align::Center);
    if let Some((label, explains)) = &lines.as_manager {
        let g = group_box();
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row.set_margin_top(12);
        row.set_margin_bottom(12);
        let l = text(label, 17, 400, &["tafs-ink"]);
        l.set_hexpand(true);
        row.append(&l);
        as_manager.update_property(&[gtk::accessible::Property::Label(label)]);
        row.append(&as_manager);
        g.append(&row);
        sheet.column.append(&g);
        sheet.column.append(&crate::sheet::footer(explains));
    }
    sheet.column.append(&crate::sheet::header(&lines.header));
    let g = group_box();
    let them = outside(group, people);
    let refused = crate::approval::refusal_slot();
    refused.set_margin_start(16);
    refused.set_margin_end(16);
    for (i, person) in them.iter().enumerate() {
        if i > 0 {
            let line = gtk::Separator::new(gtk::Orientation::Horizontal);
            line.add_css_class("tafs-line");
            g.append(&line);
        }
        let row = person_row(person.label());
        row.append(&one_line(person.label(), 15, 400, &["tafs-ink"]));
        let b = gtk::Button::new();
        b.set_child(Some(&row));
        b.add_css_class("tafs-plain");
        b.set_widget_name(&format!("people.person.{}", person.subject));
        let act_for = {
            let (group, person, as_manager) =
                (group.clone(), (*person).clone(), as_manager.clone());
            move || add_act(&group, &person, as_manager.is_active(), can_manage)
        };
        let (w, refused) = (w.clone(), refused.clone());
        b.connect_clicked(move |_| crate::approval::refuse(&w, mode, &act_for(), &refused));
        g.append(&b);
    }
    if them.is_empty() {
        let e = text(&lines.everyone_in, 14, 400, &["tafs-dim"]);
        e.set_margin_top(12);
        e.set_margin_bottom(12);
        g.append(&e);
    }
    sheet.column.append(&g);
    sheet.column.append(&refused);
    sheet.dialog.present(Some(over));
}
