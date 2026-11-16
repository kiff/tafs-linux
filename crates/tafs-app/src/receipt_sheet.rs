//! A Record line's receipt (`RecordView.swift` › `ReceiptView`), over the window: the line's state
//! with its dot, its title with the amount, its facts, what happened as KIFF signed it, and KIFF's
//! receipt checked on this computer (`tafs_core::receipt_check`). Signed in with KIFF, the receipt
//! is read once (`GET /v1/receipts/{trace_id}`); nothing is sent.

use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use tafs_core::receipt_check::{self, KIFF_SIGNER};
use tafs_core::receipt_view::{self, Panel, Step};
use tafs_core::record::{Entry, Tone};
use tafs_core::words::WordBook;

use crate::widgets::{eyebrow, outline, panel, primary, text};

/// Where a receipt is read: KIFF, as the session; a screenshot's from `--receipt`, never sent.
#[derive(Clone)]
pub struct ReceiptSource {
    pub token: String,
    pub languages: Vec<&'static str>,
    pub canned: Option<Vec<u8>>,
    /// A screenshot's: the line, by its id, whose receipt is open.
    pub open: Option<String>,
}

fn tone_class(t: Tone) -> &'static str {
    match t {
        Tone::Ok => "tafs-ok",
        Tone::Amber => "tafs-amber-text",
        Tone::No => "tafs-no",
        Tone::Dim => "tafs-dim",
    }
}

/// A fact's line: its name (13, dim, 90 wide) and value (14, ink; semibold when `bold`).
fn fact(name: &str, value: &str, bold: bool) -> gtk::Box {
    let r = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let n = text(name, 13, 400, &["tafs-dim"]);
    n.set_width_chars(12);
    n.set_size_request(90, -1);
    n.set_valign(gtk::Align::Start);
    r.append(&n);
    let v = text(value, 14, if bold { 600 } else { 400 }, &["tafs-ink"]);
    v.set_hexpand(true);
    v.set_selectable(true);
    r.append(&v);
    r
}

/// When, in the account's zone, with the date (`zonedShown(date: .abbreviated, time: .standard)`).
fn when(at: chrono::DateTime<chrono::Utc>) -> String {
    let local = tafs_core::account_time::local(&at);
    format!(
        "{}{}",
        local.format("%d %b %Y, %H:%M:%S"),
        tafs_core::account_time::label(&at)
    )
}

fn time_only(at: chrono::DateTime<chrono::Utc>) -> String {
    tafs_core::account_time::local(&at)
        .format("%H:%M:%S")
        .to_string()
}

/// The steps as a timeline (`timeline`): a ring, a line down to the next, the step's words and
/// when.
fn timeline(steps: &[Step]) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 0);
    for (i, step) in steps.iter().enumerate() {
        let r = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let mark = gtk::Box::new(gtk::Orientation::Vertical, 0);
        mark.set_size_request(14, -1);
        let ring = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        ring.add_css_class("tafs-step-ring");
        ring.set_halign(gtk::Align::Center);
        ring.set_margin_top(5);
        mark.append(&ring);
        if i + 1 < steps.len() {
            let rule = gtk::Box::new(gtk::Orientation::Vertical, 0);
            rule.add_css_class("tafs-step-rule");
            rule.set_halign(gtk::Align::Center);
            rule.set_vexpand(true);
            mark.append(&rule);
        }
        r.append(&mark);
        let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
        let mut c = step.words.chars();
        let words = match c.next() {
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            None => String::new(),
        };
        v.append(&text(&words, 14, 400, &["tafs-ink"]));
        if let Some(at) = step.at {
            let t = text(&time_only(at), 12, 400, &["tafs-dim"]);
            t.add_css_class("tafs-mono");
            v.append(&t);
        }
        v.set_margin_bottom(12);
        r.append(&v);
        b.append(&r);
    }
    b
}

/// The receipt's facts line (`receiptFacts`): the decision, and when the receipt was issued.
fn facts_line(w: &WordBook, e: &Entry, raw: &[u8]) -> gtk::Label {
    let mut lines = vec![receipt_view::decision_words(w, e).unwrap_or_default()];
    if let Some(issued) = receipt_view::field(raw, "issued_at")
        .and_then(|t| chrono::DateTime::parse_from_rfc3339(&t).ok())
    {
        let at = issued.with_timezone(&chrono::Utc);
        if let Ok(l) = w.get("receipt.issued", &[("at", &when(at))]) {
            lines.push(l);
        }
    }
    let l = text(&lines.join(" · "), 12, 400, &["tafs-dim", "tafs-mono"]);
    l.set_selectable(true);
    l
}

/// "Show the full receipt": every step KIFF signed, with its hash, and the signature.
fn full_receipt(
    w: &WordBook,
    e: &Entry,
    raw: &[u8],
    steps: &[Step],
    signer: Option<&str>,
) -> gtk::Expander {
    let trace = e
        .receipt
        .as_ref()
        .map(|r| r.trace_id.as_str())
        .unwrap_or("");
    let sig = receipt_view::attestation(raw, "signature");
    let lines = receipt_view::full_receipt(trace, signer, steps, sig.as_deref(), time_only);
    let body = text(&lines.join("\n"), 12, 400, &["tafs-dim", "tafs-mono"]);
    body.set_selectable(true);
    body.set_margin_top(8);
    let x = gtk::Expander::new(Some(&w.line("receipt.show_full").unwrap_or_default()));
    x.add_css_class("tafs-link");
    x.set_child(Some(&body));
    x
}

/// Copy, and (once checked) Share (`receiptButtons`): Share saves the receipt as it came, where
/// the person picks, as the share sheet hands it on; closing the dialog saves nothing.
fn buttons(w: &WordBook, raw: &[u8], trace: &str, share: bool) -> gtk::Box {
    let r = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let Ok(text_of) = String::from_utf8(raw.to_vec()) else {
        return r;
    };
    let copy = outline(&w.line("receipt.copy").unwrap_or_default());
    copy.set_hexpand(false);
    {
        let text_of = text_of.clone();
        copy.connect_clicked(move |b| b.clipboard().set_text(&text_of));
    }
    r.append(&copy);
    if share {
        let b = outline(&w.line("receipt.share").unwrap_or_default());
        b.set_hexpand(false);
        let (title, name) = (
            w.line("receipt.share").unwrap_or_default(),
            format!("receipt-{}.json", trace.replace(['/', '\\'], "-")),
        );
        b.connect_clicked(move |b| {
            let window = b.root().and_downcast::<gtk::Window>();
            let dialog = gtk::FileDialog::new();
            dialog.set_title(&title);
            dialog.set_initial_name(Some(&name));
            let bytes = text_of.clone().into_bytes();
            glib::spawn_future_local(async move {
                if let Ok(file) = dialog.save_future(window.as_ref()).await {
                    if let Some(path) = file.path() {
                        let _ = crate::widgets::write_private(&path, &bytes);
                    }
                }
            });
        });
        r.append(&b);
    }
    r
}

/// The receipt panel (`receiptPanel`) for what the check found.
fn receipt_panel(
    w: &WordBook,
    e: &Entry,
    p: &Panel,
    raw: Option<&[u8]>,
    steps: &[Step],
    try_again: &Rc<dyn Fn()>,
) -> gtk::Box {
    let device = w.line("device.linux").unwrap_or_default();
    let trace = e
        .receipt
        .as_ref()
        .map(|r| r.trace_id.as_str())
        .unwrap_or("");
    match p {
        Panel::Checking => {
            let b = panel(false);
            let r = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let s = gtk::Spinner::new();
            s.start();
            r.append(&s);
            r.append(&text(
                &w.get("receipt.checking_on_device", &[("device", &device)])
                    .unwrap_or_default(),
                13,
                400,
                &["tafs-soft"],
            ));
            b.append(&r);
            b
        }
        Panel::NotLoaded(why) => {
            let b = panel(false);
            b.append(&text(why, 13, 400, &["tafs-no"]));
            let again = outline(&w.line("common.try_again").unwrap_or_default());
            again.set_hexpand(false);
            again.set_halign(gtk::Align::Start);
            let try_again = try_again.clone();
            again.connect_clicked(move |_| try_again());
            b.append(&again);
            b
        }
        Panel::Checked { signer } => {
            let b = panel(false);
            b.add_css_class("edge-ok");
            b.append(&text(
                &format!(
                    "✓ {}",
                    w.get("receipt.checked_on_device", &[("device", &device)])
                        .unwrap_or_default()
                ),
                14,
                600,
                &["tafs-ok"],
            ));
            b.append(&text(
                &w.line("receipt.checked_why").unwrap_or_default(),
                13,
                400,
                &["tafs-soft"],
            ));
            if let Some(raw) = raw {
                b.append(&facts_line(w, e, raw));
                if !steps.is_empty() {
                    b.append(&full_receipt(w, e, raw, steps, Some(signer)));
                }
                b.append(&buttons(w, raw, trace, true));
            }
            b
        }
        Panel::NotSignedYet { late } => {
            let b = panel(false);
            let head = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let dot = gtk::Label::new(Some("●"));
            dot.add_css_class("tafs-amber-text");
            head.append(&dot);
            head.append(&text(
                &w.line("receipt.not_signed_yet").unwrap_or_default(),
                14,
                600,
                &["tafs-ink"],
            ));
            b.append(&head);
            b.append(&text(
                &w.line("receipt.signs_within").unwrap_or_default(),
                13,
                400,
                &["tafs-soft"],
            ));
            if *late {
                b.append(&text(
                    &w.line("receipt.if_it_stays").unwrap_or_default(),
                    13,
                    400,
                    &["tafs-soft"],
                ));
                let id = text(trace, 12, 400, &["tafs-dim", "tafs-mono"]);
                id.set_selectable(true);
                b.append(&id);
            }
            b
        }
        Panel::Failed(why) => {
            let b = panel(false);
            b.add_css_class("edge-no");
            b.append(&text(
                &format!(
                    "✗ {}",
                    w.line("record.kiff_receipt_does_not_check")
                        .unwrap_or_default()
                ),
                14,
                600,
                &["tafs-no"],
            ));
            b.append(&text(why, 13, 400, &["tafs-soft"]));
            if let Some(raw) = raw {
                b.append(&facts_line(w, e, raw));
                if !steps.is_empty() {
                    b.append(&full_receipt(w, e, raw, steps, None));
                }
                b.append(&buttons(w, raw, trace, false));
            }
            b
        }
    }
}

/// What a read found: the receipt panel, the steps as signed, whether they're drawn as what
/// happened, and the bytes all of it was read from.
type Read = (Panel, Vec<Step>, bool, Option<Vec<u8>>);

/// The receipt, read and checked off the main loop (`loadReceipt`).
fn read(w: &WordBook, source: &ReceiptSource, e: &Entry) -> Read {
    let Some(trace) = e.receipt.as_ref().map(|r| r.trace_id.clone()) else {
        return (Panel::Checking, vec![], false, None);
    };
    let not_loaded = |why: String| {
        Panel::NotLoaded(
            w.get("receipt.not_loaded", &[("why", &why)])
                .unwrap_or_default(),
        )
    };
    let answer = match &source.canned {
        Some(canned) => canned.clone(),
        None => match crate::kiff::kiff_get(
            &source.token,
            &source.languages,
            &receipt_view::path(&trace),
        ) {
            Ok(a) if a.status == 200 => a.body,
            Ok(a) => {
                let why = tafs_core::signin::refusal(&a, Some(w)).words(w);
                return (not_loaded(why.unwrap_or_default()), vec![], false, None);
            }
            Err(e) => {
                return (
                    not_loaded(e.words(w).unwrap_or_default()),
                    vec![],
                    false,
                    None,
                )
            }
        },
    };
    let check = receipt_check::check(&answer, KIFF_SIGNER);
    let steps = receipt_view::steps(&answer);
    let panel = receipt_view::panel(w, e, &check, &steps, chrono::Utc::now())
        .unwrap_or(Panel::Failed(String::new()));
    let shows = receipt_view::shows_what_happened(Some(&check), receipt_view::mismatch(e, &steps));
    (panel, steps, shows, Some(answer))
}

/// Reads the receipt, then draws what happened and the receipt panel in their places.
fn load(
    w: Rc<WordBook>,
    source: Rc<ReceiptSource>,
    e: Entry,
    happened: gtk::Box,
    receipt: gtk::Box,
) {
    let again: Rc<dyn Fn()> = {
        let (w, source, e, happened, receipt) = (
            w.clone(),
            source.clone(),
            e.clone(),
            happened.clone(),
            receipt.clone(),
        );
        Rc::new(move || {
            load(
                w.clone(),
                source.clone(),
                e.clone(),
                happened.clone(),
                receipt.clone(),
            )
        })
    };
    clear(&receipt);
    receipt.append(&receipt_panel(&w, &e, &Panel::Checking, None, &[], &again));
    glib::spawn_future_local(async move {
        let (words, src, entry) = ((*w).clone(), (*source).clone(), e.clone());
        let (p, steps, shows, raw) = gio::spawn_blocking(move || read(&words, &src, &entry))
            .await
            .unwrap_or((Panel::Failed(String::new()), vec![], false, None));
        clear(&happened);
        happened.set_visible(false);
        clear(&receipt);
        // Only what KIFF signed for this line: the rest stays in the full receipt, under its panel.
        if shows && !steps.is_empty() {
            happened.append(&eyebrow(
                &w.line("receipt.what_happened").unwrap_or_default(),
                false,
            ));
            happened.append(&timeline(&steps));
            happened.set_visible(true);
        }
        receipt.append(&receipt_panel(&w, &e, &p, raw.as_deref(), &steps, &again));
    });
}

fn clear(b: &gtk::Box) {
    while let Some(c) = b.first_child() {
        b.remove(&c);
    }
}

/// The sheet for `e`, whose taf is `taf_name` (`ReceiptView`). `source`: signed in with KIFF,
/// where its receipt is read; none in the sample, which reads nothing. `open_taf`: the taf's page,
/// by its key.
pub fn present(
    w: Rc<WordBook>,
    source: Option<Rc<ReceiptSource>>,
    e: Entry,
    taf_name: Option<String>,
    open_taf: Rc<dyn Fn(String)>,
    parent: &impl IsA<gtk::Widget>,
) {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 16);
    column.set_margin_start(20);
    column.set_margin_end(20);
    column.set_margin_top(20);
    column.set_margin_bottom(20);
    let state = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let dot = gtk::Label::new(Some("●"));
    dot.add_css_class(tone_class(e.kind.tone()));
    state.append(&dot);
    state.append(&text(
        &receipt_view::state_label(&w, &e).unwrap_or_default(),
        14,
        600,
        &[tone_class(e.kind.tone())],
    ));
    column.append(&state);
    column.append(&text(
        &e.title_with_amount(&w).unwrap_or_else(|_| e.title.clone()),
        30,
        700,
        &["tafs-ink"],
    ));
    let l = |k: &str| w.line(k).unwrap_or_default();
    let facts = panel(false);
    facts.append(&fact(
        &l("receipt.fact.taf"),
        taf_name.as_deref().unwrap_or("–"),
        false,
    ));
    facts.append(&fact(&l("receipt.fact.at"), &when(e.at), false));
    if let Some(on) = &e.on_value {
        facts.append(&fact(
            &l("receipt.fact.on"),
            &format!(
                "{} {on}",
                e.on_arg.clone().unwrap_or_default().replace('_', " ")
            ),
            false,
        ));
    }
    facts.append(&fact(
        &l("receipt.fact.outcome"),
        &receipt_view::outcome(&w, &e).unwrap_or_default(),
        true,
    ));
    // KIFF's words, never its codes.
    if let Some(reason) = &e.reason {
        facts.append(&fact(&l("receipt.fact.why"), reason, false));
    }
    facts.append(&fact(
        &l("receipt.fact.decided_by"),
        &receipt_view::decided_by(&w, &e, source.is_some()).unwrap_or_default(),
        false,
    ));
    column.append(&facts);
    // What happened, and the receipt: read when the sheet opens, if the line has one. Signed
    // out of KIFF (the sample) nothing is read, and the panel says it's checking, as the iPad.
    let happened = gtk::Box::new(gtk::Orientation::Vertical, 8);
    happened.set_visible(false);
    column.append(&happened);
    let receipt = gtk::Box::new(gtk::Orientation::Vertical, 0);
    column.append(&receipt);
    if e.receipt.is_some() {
        match source {
            Some(source) => load(w.clone(), source, e.clone(), happened, receipt),
            None => {
                let none: Rc<dyn Fn()> = Rc::new(|| {});
                receipt.append(&receipt_panel(&w, &e, &Panel::Checking, None, &[], &none));
            }
        }
    }
    let open = primary(&l("record.open_taf_chat"));
    column.append(&open);

    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_child(Some(&column));
    scroller.set_vexpand(true);
    let header = adw::HeaderBar::new();
    header.set_show_end_title_buttons(false);
    header.set_show_start_title_buttons(false);
    let done = gtk::Button::with_label(&l("record.filter.done"));
    done.add_css_class("flat");
    header.pack_end(&done);
    header.set_title_widget(Some(&gtk::Label::new(None)));
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&scroller));
    crate::widgets::ground(&toolbar);
    let dialog = adw::Dialog::new();
    dialog.set_child(Some(&toolbar));
    dialog.set_content_width(560);
    dialog.set_content_height(680);
    dialog.set_title(&e.title);
    {
        let dialog = dialog.clone();
        done.connect_clicked(move |_| {
            dialog.close();
        });
    }
    {
        let (dialog, taf) = (dialog.clone(), e.taf.clone());
        open.connect_clicked(move |_| {
            open_taf(taf.clone());
            dialog.close();
        });
    }
    dialog.present(Some(parent));
}
