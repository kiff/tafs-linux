//! An app's connections in Connect apps (`AppConnectionsView`), one connection's detail
//! (`ConnectionDetailView`) and a connection's approval (`ConnectApprovalSheet`). Removing
//! access, cancelling a request or a sign-in, clearing a removed taf's connections and signing in
//! again are each signed at planque behind Face ID: this computer asks the iPad's question, and the
//! tap that would sign says it can't approve, sending nothing. The approval's own button is
//! `ApprovalGate`'s, so its line stands in the button's place. Signed in with KIFF, the account's
//! connections are read from planque with this computer's device key (`readsConnections`,
//! `connect::read_connections`), and read again once the key approved an act; until then the
//! screen says so. The sample's screenshot draws the reference set's. The lines are `tafs-core/src/connections.rs`.

use std::rc::Rc;

use adw::prelude::*;
use tafs_core::connect::{App, ConnectStart, RemovedTaf};
use tafs_core::connections::{
    attention, cancel_title, cancel_word, clearable, connections_of, disconnect_message,
    disconnect_title, end_word, groups, kept_by, misplaced, removed_listed, row_used_by,
    shows_footer, status_line, Action, Grant,
};
use tafs_core::signed::Act;

use crate::connect::{group, section_header, separator, Ctx, Screens};
use crate::widgets::{one_line, primary, symbol, text};

thread_local! {
    /// A screenshot's page once an app's connections are open (`connect::OPEN`'s).
    pub static OPEN: std::cell::Cell<Option<&'static str>> = const { std::cell::Cell::new(None) };
}

/// An act's tap, with what its question named it (`what`): the security key's sheet says it, as
/// the iPad's Face ID reason (`DisconnectWords.title`, `ConnectionGroupWords.clearTitle`).
type Refuse = Rc<dyn Fn(Act, Option<String>)>;

/// A screen's head: Back with the screen under it, and its title, inline.
fn head(back_words: &str, title: &str, back: Rc<dyn Fn()>) -> gtk::CenterBox {
    let head = gtk::CenterBox::new();
    let back_button = gtk::Button::new();
    // Round, its words for the screen reader and the pointer, as the iPad's Back.
    back_button.add_css_class("tafs-round-back");
    back_button.set_child(Some(&symbol("go-previous-symbolic", 20)));
    back_button.set_tooltip_text(Some(back_words));
    back_button.update_property(&[gtk::accessible::Property::Label(back_words)]);
    back_button.connect_clicked(move |_| back());
    back_button.set_margin_start(8);
    head.set_start_widget(Some(&back_button));
    let t = one_line(title, 17, 600, &["tafs-ink"]);
    t.set_max_width_chars(40);
    head.set_center_widget(Some(&t));
    head
}

/// A screen: its head over its column, scrolled, on the app's cream.
fn screen(head: &gtk::CenterBox, column: &gtk::Box) -> gtk::Widget {
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    outer.set_margin_top(12);
    outer.append(head);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(12);
    column.set_margin_bottom(12);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_vexpand(true);
    scroller.set_child(Some(column));
    outer.append(&scroller);
    crate::widgets::ground(&outer);
    outer.upcast()
}

/// Where the iPad says why removing didn't go: here, once a tap would have signed, that this
/// computer can't approve. `refuse` fills it.
fn refusal(ctx: &Rc<Ctx>, column: &gtk::Box) -> Refuse {
    let slot = crate::approval::refusal_slot();
    let panel = crate::widgets::panel(false);
    panel.append(&slot);
    panel.set_visible(false);
    column.append(&panel);
    let ctx = ctx.clone();
    Rc::new(move |act: Act, what: Option<String>| {
        // Once the key approved and the act went, the connections are read again, as the iPad's.
        let then = ctx.reread.borrow().clone();
        crate::approval::refuse_then(&ctx.w, ctx.mode(), &act, what, vec![], &slot, then);
        panel.set_visible(slot.get_visible());
    })
}

/// A destructive button with no border, as the iPad's `.borderless` with `role: .destructive`.
fn destructive(words: &str) -> gtk::Button {
    let b = gtk::Button::with_label(words);
    b.add_css_class("tafs-row-button");
    b.add_css_class("tafs-no");
    b.set_valign(gtk::Align::Center);
    b
}

/// A question over the screen (`confirmationDialog`), its act in red, then Cancel (or `keep`).
fn ask(
    over: &gtk::Widget,
    title: &str,
    body: Option<&str>,
    act_words: &str,
    keep: &str,
    then: impl Fn() + 'static,
) {
    let d = adw::AlertDialog::new(Some(title), body);
    d.add_response("keep", keep);
    d.add_response("act", act_words);
    d.set_response_appearance("act", adw::ResponseAppearance::Destructive);
    d.set_close_response("keep");
    d.connect_response(Some("act"), move |_, _| then());
    d.present(Some(over));
}

/// `AppConnectionsView`: the app's connections grouped by the taf each was set up for, those of
/// removed tafs with Clear, the rest last; Disconnect's footer; why removing didn't go.
pub(crate) fn app_page(
    ctx: Rc<Ctx>,
    screens: Rc<Screens>,
    app: App,
    removed_tafs: Vec<RemovedTaf>,
) -> gtk::Widget {
    let w = ctx.w.clone();
    let l = |k: &str| w.line(k).unwrap_or_default();
    let title = w
        .get(
            "connect.apps.your_connections",
            &[("app", app.display_name())],
        )
        .unwrap_or_default();
    let back = {
        let screens = screens.clone();
        Rc::new(move || screens.back("apps"))
    };
    let head = head(&l("connect.apps.title"), &title, back);
    let column = gtk::Box::new(gtk::Orientation::Vertical, 10);
    let grants = ctx.grants.borrow();
    let conns = connections_of(&app, &grants);
    let removed_agents = removed_tafs.iter().map(|t| t.agent.clone()).collect();
    let grouped = groups(&conns, &|g| ctx.taf_of(g), &removed_agents, &attention);
    // What tafs cloud says of each removed taf's connection: only one it lists with nobody using
    // it is cleared. Not listed means no Clear.
    let listed = removed_listed(&removed_tafs);
    let clear_ids = clearable(&grouped.removed, &listed);
    // Until this computer has read its connections (`readsConnections`), it says so, rather than
    // an empty list.
    if !ctx.read.get() {
        let device = l("device.linux");
        column.append(&text(
            &w.get(
                "connect.start_connected_tools_first",
                &[("device", &device)],
            )
            .unwrap_or_default(),
            15,
            400,
            &["tafs-soft"],
        ));
    }
    // Filled once the sections are drawn: the refusal goes under them, as the iPad's error.
    let sections = gtk::Box::new(gtk::Orientation::Vertical, 10);
    column.append(&sections);
    let refuse = refusal(&ctx, &column);
    let open_connection: Rc<dyn Fn(String)> = {
        let (ctx, screens, title, app_title) = (
            ctx.clone(),
            screens.clone(),
            title.clone(),
            app.display_name().to_string(),
        );
        Rc::new(move |id: String| {
            let page = connection_page(ctx.clone(), screens.clone(), &id, &app_title, &title);
            screens.show("connection", &page);
        })
    };
    let manages = ctx.manages();
    let entries = |gs: &[&Grant]| {
        let g_box = group();
        for (i, g) in gs.iter().enumerate() {
            if i > 0 {
                g_box.append(&separator());
            }
            g_box.append(&entry(&ctx, &app, g, &refuse, &open_connection));
        }
        g_box
    };
    for t in &grouped.tafs {
        sections.append(&section_header(&t.taf));
        sections.append(&entries(&t.grants));
    }
    // Removed tafs' connections, as tafs cloud names them: no Sign in again and no Remove access
    // each, one action that clears them.
    if !grouped.removed.is_empty() {
        sections.append(&section_header(&l("connect.from_removed")));
        let g_box = group();
        for (i, g) in grouped.removed.iter().enumerate() {
            if i > 0 {
                g_box.append(&separator());
            }
            g_box.append(&removed_row(&ctx, &app, g, listed.get(&g.id)));
        }
        if manages && !clear_ids.is_empty() {
            g_box.append(&separator());
            let b = destructive(&l("connect.clear_removed.button"));
            b.set_halign(gtk::Align::Start);
            b.set_margin_top(6);
            b.set_margin_bottom(6);
            let (w, refuse) = (w.clone(), refuse.clone());
            b.connect_clicked(move |b| {
                let n = clear_ids.len() as i64;
                let ids = clear_ids.clone();
                let refuse = refuse.clone();
                let title = w
                    .count("connect.clear_removed.title", n, &[])
                    .unwrap_or_default();
                // Asked first, then the security key's touch and PIN for all of them (or the
                // can't-approve line, with no key here).
                ask(
                    b.upcast_ref(),
                    &title.clone(),
                    Some(&w.line("connect.clear_removed.body").unwrap_or_default()),
                    &w.line("connect.clear_removed.clear").unwrap_or_default(),
                    &w.line("common.cancel").unwrap_or_default(),
                    move || {
                        refuse(
                            Act::ClearRemoved {
                                grants: ids.clone(),
                            },
                            Some(title.clone()),
                        )
                    },
                );
            });
            g_box.append(&b);
        }
        sections.append(&g_box);
    }
    if !grouped.others.is_empty() {
        sections.append(&section_header(&l("connect.other_connections")));
        sections.append(&entries(&grouped.others));
    }
    if shows_footer(manages, &grouped.tafs) {
        sections.append(&crate::sheet::footer(&l("connect.apps.disconnect_why")));
    }
    let page = screen(&head, &column);
    // A screenshot's connection: the first one drawn, its detail, and its Remove access question.
    if let Some(open) = OPEN.with(|o| o.take()) {
        let first = grouped
            .tafs
            .first()
            .and_then(|t| t.grants.first())
            .or(grouped.others.first())
            .map(|g| g.id.clone());
        if let (Some(id), "connect-connection" | "connect-remove") = (first, open) {
            if open == "connect-remove" {
                OPEN.with(|o| o.set(Some(open)));
            }
            let open_connection = open_connection.clone();
            gtk::glib::idle_add_local_once(move || open_connection(id));
        }
    }
    page
}

/// A connection's row (`entry`): as planque draws it when it says (its name, the tafs on it, its
/// health), opening its detail, where its actions are; else the app's own row.
fn entry(
    ctx: &Rc<Ctx>,
    app: &App,
    g: &Grant,
    refuse: &Refuse,
    open: &Rc<dyn Fn(String)>,
) -> gtk::Widget {
    let w = &ctx.w;
    if !g.row_from_planque() {
        return row(ctx, app, g, refuse);
    }
    let r = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
    v.set_hexpand(true);
    v.set_valign(gtk::Align::Center);
    let name = g.name().unwrap_or_else(|| app.display_name().to_string());
    v.append(&text(&name, 16, 600, &["tafs-ink"]));
    if let Ok(Some(used)) = row_used_by(w, g) {
        v.append(&text(&used, 13, 400, &["tafs-soft"]));
    }
    if let Some(h) = &g.health {
        v.append(&text(
            &h.words,
            13,
            400,
            &[if h.ok { "tafs-dim" } else { "tafs-amber-text" }],
        ));
    }
    if let Ok(Some(m)) = misplaced(w, app, g) {
        v.append(&text(&m, 13, 400, &["tafs-amber-text"]));
    }
    r.append(&v);
    let c = symbol("go-next-symbolic", 13);
    c.add_css_class("tafs-dim");
    c.set_valign(gtk::Align::Center);
    r.append(&c);
    r.set_size_request(-1, 44);
    r.set_margin_top(10);
    r.set_margin_bottom(10);
    r.set_cursor_from_name(Some("pointer"));
    r.update_property(&[gtk::accessible::Property::Label(&name)]);
    let click = gtk::GestureClick::new();
    let (open, id) = (open.clone(), g.id.clone());
    click.connect_released(move |_, _, _, _| open(id.clone()));
    r.add_controller(click);
    r.upcast()
}

/// A connection's own row (`row`): its lines, how to fix it, and Remove access, or Cancel while
/// its request or sign-in waits.
fn row(ctx: &Rc<Ctx>, app: &App, g: &Grant, refuse: &Refuse) -> gtk::Widget {
    let w = ctx.w.clone();
    let manages = ctx.manages();
    let r = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
    v.set_hexpand(true);
    v.set_valign(gtk::Align::Center);
    // What it is: planque's name for it, else the app's; planque's status words.
    v.append(&text(
        &g.name().unwrap_or_else(|| app.display_name().to_string()),
        16,
        600,
        &["tafs-ink"],
    ));
    if let Ok(Some(s)) = status_line(&w, g) {
        v.append(&text(&s, 13, 400, &["tafs-dim"]));
    }
    if let Ok(Some(m)) = misplaced(&w, app, g) {
        v.append(&text(&m, 13, 400, &["tafs-amber-text"]));
    }
    // Sign in again is signed where it says so: under the row, as the iPad's reason.
    let slot = crate::approval::refusal_slot();
    if manages && g.signs_in_through_provider() {
        // A provider's connection isn't signed in again in place: said before any Face ID.
        v.append(&text(
            &w.get("connect.through_provider", &[("vendor", g.vendor())])
                .unwrap_or_default(),
            13,
            400,
            &["tafs-soft"],
        ));
    } else if manages && !g.cancellable() {
        let fix = gtk::Button::with_label(
            &w.line(if g.takes_key() {
                "connect.replace_key"
            } else {
                "connect.sign_in_again"
            })
            .unwrap_or_default(),
        );
        fix.add_css_class("tafs-link");
        fix.set_halign(gtk::Align::Start);
        let (ctx, grant, slot) = (ctx.clone(), g.clone(), slot.clone());
        fix.connect_clicked(move |b| {
            if grant.takes_key() {
                // A new key, on the key's sheet: its button is `ApprovalGate`'s.
                let start = ConnectStart {
                    grant: grant.id.clone(),
                    needs_key: true,
                };
                approval_sheet(&ctx, &start, b.upcast_ref());
            } else {
                crate::approval::refuse(
                    &ctx.w,
                    ctx.mode(),
                    &Act::SignInAgain {
                        grant: grant.id.clone(),
                    },
                    &slot,
                );
            }
        });
        v.append(&fix);
    }
    v.append(&slot);
    r.append(&v);
    if manages {
        let end = destructive(&end_word(&w, g).unwrap_or_default());
        let (w, grant, refuse) = (w.clone(), g.clone(), refuse.clone());
        let (taf, key) = (ctx.taf_of(g), g.takes_key());
        // The other tafs whose Card draws on its tools stop too (`alsoStops`).
        let also = tafs_core::connections::also_stops(g, &ctx.using);
        end.connect_clicked(move |b| {
            let act = Act::Disconnect {
                grant: grant.id.clone(),
            };
            let refuse = refuse.clone();
            if grant.cancellable() {
                let title = cancel_title(&w, &grant).unwrap_or_default();
                ask(
                    b.upcast_ref(),
                    &title.clone(),
                    Some(
                        &w.get("connect.cancel_sign_in.body", &[("vendor", grant.vendor())])
                            .unwrap_or_default(),
                    ),
                    &cancel_word(&w, &grant).unwrap_or_default(),
                    &w.line("connect.cancel_sign_in.keep_waiting")
                        .unwrap_or_default(),
                    move || refuse(act.clone(), Some(title.clone())),
                );
            } else {
                // Asked first, naming the one taf it ends; only then the security key, in the same
                // words.
                let title =
                    disconnect_title(&w, grant.vendor(), taf.as_deref(), &also).unwrap_or_default();
                ask(
                    b.upcast_ref(),
                    &title.clone(),
                    Some(
                        &disconnect_message(&w, grant.vendor(), taf.as_deref(), &also, key)
                            .unwrap_or_default(),
                    ),
                    &w.line("connect.disconnect_button").unwrap_or_default(),
                    &w.line("common.cancel").unwrap_or_default(),
                    move || refuse(act.clone(), Some(title.clone())),
                );
            }
        });
        r.append(&end);
    }
    r.set_size_request(-1, 44);
    r.set_margin_top(10);
    r.set_margin_bottom(10);
    r.upcast()
}

/// A removed taf's connection (`removedRow`): what it is, the removed taf it was made for by the
/// name planque keeps, planque's status words, and the tafs whose Cards still use it.
fn removed_row(
    ctx: &Ctx,
    app: &App,
    g: &Grant,
    listed: Option<&tafs_core::connect::RemovedGrant>,
) -> gtk::Widget {
    let w = &ctx.w;
    let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
    v.append(&text(
        &g.name().unwrap_or_else(|| app.display_name().to_string()),
        16,
        600,
        &["tafs-ink"],
    ));
    if let Some(n) = g.agent_name.as_deref().filter(|n| !n.trim().is_empty()) {
        v.append(&text(n, 13, 400, &["tafs-soft"]));
    }
    if let Some(s) = g.status_words.as_deref().filter(|s| !s.is_empty()) {
        v.append(&text(s, 13, 400, &["tafs-dim"]));
    }
    if let Some(l) = listed {
        let names: Vec<String> = l.used_by.iter().map(|u| u.name.clone()).collect();
        if let Ok(Some(kept)) = kept_by(w, &names) {
            v.append(&text(&kept, 13, 400, &["tafs-soft"]));
        }
    }
    v.set_size_request(-1, 44);
    v.set_margin_top(10);
    v.set_margin_bottom(10);
    v.upcast()
}

/// `ConnectionDetailView`: a connection's health, the tafs on it, and only the actions planque
/// says can work for it now; Remove asks once more, in planque's words, saying what stops.
fn connection_page(
    ctx: Rc<Ctx>,
    screens: Rc<Screens>,
    id: &str,
    app_title: &str,
    back_words: &str,
) -> gtk::Widget {
    let w = ctx.w.clone();
    let g = ctx.grants.borrow().iter().find(|g| g.id == id).cloned();
    let title = g
        .as_ref()
        .and_then(Grant::name)
        .unwrap_or_else(|| app_title.to_string());
    let back = {
        let screens = screens.clone();
        Rc::new(move || screens.back("app"))
    };
    let head = head(back_words, &title, back);
    let column = gtk::Box::new(gtk::Orientation::Vertical, 10);
    let mut remove: Option<(gtk::Button, Action)> = None;
    if let Some(g) = &g {
        let first = group();
        let lines = gtk::Box::new(gtk::Orientation::Vertical, 6);
        lines.set_margin_top(12);
        lines.set_margin_bottom(12);
        if let Some(h) = &g.health {
            lines.append(&text(
                &h.words,
                15,
                600,
                &[if h.ok { "tafs-ink" } else { "tafs-amber-text" }],
            ));
        }
        if let Ok(Some(used)) = row_used_by(&w, g) {
            lines.append(&text(&used, 14, 400, &["tafs-soft"]));
        }
        first.append(&lines);
        column.append(&first);
        let actions = g.known_actions();
        let slot_column = gtk::Box::new(gtk::Orientation::Vertical, 0);
        if ctx.manages() && !actions.is_empty() {
            let acts = group();
            for (i, a) in actions.iter().enumerate() {
                if i > 0 {
                    acts.append(&separator());
                }
                let b = gtk::Button::with_label(&a.words);
                b.add_css_class("tafs-row-button");
                b.add_css_class(if a.removes() {
                    "tafs-no"
                } else {
                    "tafs-amber-text"
                });
                if let Some(c) = b.child() {
                    c.set_halign(gtk::Align::Start);
                }
                acts.append(&b);
                if a.removes() && remove.is_none() {
                    remove = Some((b.clone(), (*a).clone()));
                }
                let (ctx, grant, action) = (ctx.clone(), g.clone(), (*a).clone());
                let refuse_slot = slot_column.clone();
                b.connect_clicked(move |b| take(&ctx, &grant, &action, b, &refuse_slot));
            }
            column.append(&acts);
        }
        column.append(&slot_column);
    }
    let page = screen(&head, &column);
    if OPEN.with(|o| o.take()) == Some("connect-remove") {
        if let Some((b, _)) = remove {
            gtk::glib::idle_add_local_once(move || b.emit_clicked());
        }
    }
    page
}

/// An action of planque's (`take`): Remove asks first; Sign in again opens the key's sheet for a
/// key, else its tap would sign.
fn take(ctx: &Rc<Ctx>, g: &Grant, action: &Action, b: &gtk::Button, slot_column: &gtk::Box) {
    let refuse = {
        let (ctx, slot_column) = (ctx.clone(), slot_column.clone());
        move |act: Act, what: Option<String>| {
            while let Some(c) = slot_column.first_child() {
                slot_column.remove(&c);
            }
            let slot = crate::approval::refusal_slot();
            let panel = crate::widgets::panel(false);
            panel.append(&slot);
            slot_column.append(&panel);
            // Once the key approved and the act went, the connections are read again, as the iPad's.
            let then = ctx.reread.borrow().clone();
            crate::approval::refuse_then(&ctx.w, ctx.mode(), &act, what, vec![], &slot, then);
            panel.set_visible(slot.get_visible());
        }
    };
    let w = &ctx.w;
    if action.removes() {
        let act = Act::Disconnect {
            grant: g.id.clone(),
        };
        let what = action.words.clone();
        ask(
            b.upcast_ref(),
            &action.words,
            action.stops.as_deref(),
            &action.words,
            &w.line("common.cancel").unwrap_or_default(),
            move || refuse(act.clone(), Some(what.clone())),
        );
    } else if action.how == "key" {
        let start = ConnectStart {
            grant: g.id.clone(),
            needs_key: true,
        };
        approval_sheet(ctx, &start, b.upcast_ref());
    } else {
        refuse(
            Act::SignInAgain {
                grant: g.id.clone(),
            },
            None,
        );
    }
}

/// `ConnectApprovalSheet`: a connection's approval over the list, up to its signature. Its
/// connection's words when this computer has the request, else that it's reading it; the
/// approval's button is `ApprovalGate`'s, so this computer's line stands in its place.
pub(crate) fn approval_sheet(ctx: &Ctx, start: &ConnectStart, over: &gtk::Widget) {
    let w = &ctx.w;
    let l = |k: &str| w.line(k).unwrap_or_default();
    let grant = ctx
        .grants
        .borrow()
        .iter()
        .find(|g| g.id == start.grant)
        .cloned();
    let title = match &grant {
        Some(g) => w
            .get("coach.connect_service", &[("service", g.vendor())])
            .unwrap_or_default(),
        None => l("connect.apps.connect"),
    };
    let sheet = crate::sheet::new(&title, &l("common.cancel"), None, 460);
    let column = &sheet.column;
    match grant {
        // This computer reads no request from planque: it says it's reading it, as the iPad
        // does until its read comes.
        None => column.append(&text(&l("connect.reading_request"), 14, 400, &["tafs-dim"])),
        Some(g) => {
            // The connection's own words from the connection server.
            column.append(&text(&g.text, 15, 400, &["tafs-ink"]));
            if !g.digest_matches {
                column.append(&text(
                    &l("connect.request_does_not_match_what"),
                    13,
                    400,
                    &["tafs-no"],
                ));
            }
            let passcode = l("approval.your_passcode");
            let button = if start.needs_key {
                // Where the key goes, before it's given. No key is pasted on this computer: it
                // can't give one.
                let host = tafs_core::connections::host(&g.request.server);
                column.append(&text(
                    &w.get("connect.key_goes_to", &[("host", &host)])
                        .unwrap_or_default(),
                    15,
                    400,
                    &["tafs-ink"],
                ));
                primary(
                    &w.get("connect.give_key_with", &[("biometry", &passcode)])
                        .unwrap_or_default(),
                )
            } else {
                primary(
                    &w.get("approval.approve_with", &[("biometry", &passcode)])
                        .unwrap_or_default(),
                )
            };
            // Approve only what's shown: its digest checked, and for a key, a key given.
            button.set_sensitive(false);
            column.append(&crate::approval::gate(w, ctx.mode(), &button));
        }
    }
    sheet.dialog.present(Some(over));
}
