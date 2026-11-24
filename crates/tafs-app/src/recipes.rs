//! Home with no taf yet (`HomeRecipes`, `OwnWordsRow`, `RecipeRow`, `RecipeSheet`): tafs cloud's
//! line, "Say it in your own words", a chip per category that narrows the list, and each recipe
//! led by the problem it ends with its line and its apps under it; a recipe opens its sheet. Every
//! word is tafs cloud's, in the person's language.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gdk, gio, glib};
use tafs_core::recipes::{Recipe, Recipes};

use crate::widgets::{eyebrow, panel, primary, symbol, text};

/// What the pane draws from: the recipes, and whether this person may use Coach (its "Say it in
/// your own words" row and the sheet's button).
pub struct RecipesPane {
    pub recipes: Recipes,
    pub may_use_coach: bool,
    /// The sheet's button words where the gallery says them ("Try it with Coach" while
    /// browsing); none, the sheet's own (`labels.hand`).
    pub hand_words: Option<String>,
    /// The label over a job's sample, the gallery's; none when it sends none.
    pub sample_label: Option<String>,
}

/// A recipe's category colour (`RecipeTint`), as a CSS colour for this appearance; none without
/// one.
fn tint(r: &Recipes, category: &str) -> Option<String> {
    let color = r.categories.iter().find(|c| c.id == category)?.color?;
    let dark = adw::StyleManager::default().is_dark();
    Some(format!(
        "#{:06X}",
        if dark { color.dark } else { color.light }
    ))
}

/// How wide the list is drawn, and its room: Home's pane (720, 24 by 20, 12 apart) or the
/// gallery's sheet (640, 20 by 16, 16 apart).
#[derive(Clone, Copy)]
struct Measure {
    width: i32,
    side: i32,
    top: i32,
    spacing: i32,
}

const HOME: Measure = Measure {
    width: crate::widgets::PageColumn::Reading.width(),
    side: 24,
    top: 20,
    spacing: 12,
};

const GALLERY: Measure = Measure {
    width: crate::widgets::PageColumn::Gallery.width(),
    side: 20,
    top: 16,
    spacing: 16,
};

/// Home with no taf: the recipes take the pane beside the sidebar.
pub fn page(pane: Rc<RecipesPane>) -> gtk::Widget {
    list_page(pane, HOME, &[])
}

/// The list in a scroller, `head` over it: what the gallery leads with.
fn list_page(pane: Rc<RecipesPane>, m: Measure, head: &[gtk::Widget]) -> gtk::Widget {
    let picked: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let outer = gtk::Box::new(gtk::Orientation::Vertical, m.spacing);
    outer.set_margin_start(m.side);
    outer.set_margin_end(m.side);
    outer.set_margin_top(m.top);
    outer.set_margin_bottom(m.top);
    for w in head {
        outer.append(w);
    }
    let column = gtk::Box::new(gtk::Orientation::Vertical, 12);
    outer.append(&column);
    // At most 720 wide (640 in the gallery), in the middle of the pane, as the iPad's.
    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(m.width);
    clamp.set_tightening_threshold(m.width);
    clamp.set_child(Some(&outer));

    let list = gtk::Box::new(gtk::Orientation::Vertical, 12);
    let draw_list: Rc<dyn Fn()> = {
        let (list, pane, picked) = (list.clone(), pane.clone(), picked.clone());
        Rc::new(move || {
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }
            for (category, recipes) in pane.recipes.groups(picked.borrow().as_deref()) {
                list.append(&eyebrow(&category.title, false));
                for recipe in recipes {
                    list.append(&row(&pane, recipe));
                }
            }
        })
    };

    if let Some(words) = pane.recipes.home_words() {
        column.append(&text(words, 17, 600, &["tafs-ink"]));
    }
    // "Say it in your own words": Coach, in a new conversation (`startNewTaf`); only for someone
    // who may use Coach.
    if let (true, Some(own)) = (pane.may_use_coach, pane.recipes.home_own_words()) {
        let b = own_words_row(&own.title, &own.line);
        b.connect_clicked(|_| crate::coach_start::new_taf());
        column.append(&b);
    }
    let all = pane.recipes.by_category();
    if all.len() > 1 {
        let chips = gtk::FlowBox::new();
        chips.set_selection_mode(gtk::SelectionMode::None);
        chips.set_column_spacing(8);
        chips.set_row_spacing(8);
        chips.set_homogeneous(false);
        chips.set_max_children_per_line(20);
        let buttons: Rc<RefCell<Vec<(String, gtk::ToggleButton)>>> = Rc::default();
        for (category, _) in &all {
            let chip = gtk::ToggleButton::with_label(&category.title);
            chip.add_css_class("tafs-chip-button");
            if let Some(color) = tint(&pane.recipes, &category.id) {
                chip.set_widget_name(&crate::style::tint_name("chip"));
                crate::style::tint_chip(&chip, &color);
            }
            let (id, picked, draw_list, buttons2) = (
                category.id.clone(),
                picked.clone(),
                draw_list.clone(),
                buttons.clone(),
            );
            chip.connect_clicked(move |b| {
                // Picked again, every recipe shows.
                let on = picked.borrow().as_deref() != Some(id.as_str());
                *picked.borrow_mut() = on.then(|| id.clone());
                for (other, button) in buttons2.borrow().iter() {
                    if button != b {
                        button.set_active(on && other == &id);
                    }
                }
                b.set_active(on);
                draw_list();
            });
            buttons
                .borrow_mut()
                .push((category.id.clone(), chip.clone()));
            chips.insert(&chip, -1);
        }
        column.append(&chips);
    }
    column.append(&list);
    draw_list();

    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_child(Some(&clamp));
    scroller.set_vexpand(true);
    crate::widgets::ground(&scroller);
    scroller.upcast()
}

/// `OwnWordsRow`: its title (16 semibold), its line (14, soft) and a chevron, in a panel. It
/// opens Coach with nothing in its box.
pub fn own_words_row(title: &str, line: &str) -> gtk::Button {
    let p = panel(false);
    let h = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let v = gtk::Box::new(gtk::Orientation::Vertical, 4);
    v.set_hexpand(true);
    v.append(&text(title, 16, 600, &["tafs-ink"]));
    if !line.is_empty() {
        v.append(&text(line, 14, 400, &["tafs-soft"]));
    }
    h.append(&v);
    let chevron = symbol("go-next-symbolic", 13);
    chevron.add_css_class("tafs-dim");
    h.append(&chevron);
    p.append(&h);
    let b = gtk::Button::new();
    b.set_child(Some(&p));
    b.add_css_class("tafs-plain");
    b.set_widget_name("gallery.own-words");
    b
}

thread_local! {
    /// Each logo read once a launch, by its URL: its picture, or none when it didn't load.
    static LOGOS: RefCell<std::collections::HashMap<String, Option<gdk::Texture>>> =
        RefCell::default();
}

/// An app's logo (`size`: 18 on a recipe, 24 in Connect apps), loaded from tafs cloud (`AppMark`'s
/// `AsyncImage`): nothing while it loads or when it fails, so the name stands alone.
pub fn logo(url: String, size: i32) -> gtk::Picture {
    // Hidden from a screen reader, as the iPad's: the name says it.
    let picture = gtk::Picture::builder()
        .accessible_role(gtk::AccessibleRole::Presentation)
        .build();
    picture.set_size_request(size, size);
    picture.set_can_shrink(true);
    picture.set_content_fit(gtk::ContentFit::Contain);
    picture.set_visible(false);
    let show = |p: &gtk::Picture, t: &Option<gdk::Texture>| {
        if let Some(t) = t {
            p.set_paintable(Some(t));
            p.set_visible(true);
        }
    };
    if let Some(known) = LOGOS.with(|l| l.borrow().get(&url).cloned()) {
        show(&picture, &known);
        return picture;
    }
    let p = picture.clone();
    glib::spawn_future_local(async move {
        let fetched = {
            let url = url.clone();
            gio::spawn_blocking(move || crate::kiff::logo(&url))
                .await
                .ok()
                .flatten()
        };
        let texture =
            fetched.and_then(|b| gdk::Texture::from_bytes(&glib::Bytes::from_owned(b)).ok());
        LOGOS.with(|l| l.borrow_mut().insert(url, texture.clone()));
        show(&p, &texture);
    });
    picture
}

/// `AppMarks`: the apps a job uses, each after its logo where tafs cloud serves one (`AppMark`):
/// the name (12, dim) stands alone without one, while it loads, or when it fails.
fn app_marks(recipe: &Recipe) -> gtk::FlowBox {
    let marks = gtk::FlowBox::new();
    marks.set_selection_mode(gtk::SelectionMode::None);
    marks.set_column_spacing(8);
    marks.set_homogeneous(false);
    marks.set_max_children_per_line(20);
    for app in &recipe.apps {
        let mark = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        if let Some(url) = tafs_core::recipes::logo_url(app.logo.as_deref()) {
            mark.append(&logo(url, 18));
        }
        mark.append(&text(&app.name, 12, 400, &["tafs-dim"]));
        marks.insert(&mark, -1);
    }
    marks
}

/// `RecipeRow`: the problem it ends (16 semibold), its line (14, soft), its apps, on an edge in
/// its category's colour; it opens its sheet.
fn row(pane: &Rc<RecipesPane>, recipe: &Recipe) -> gtk::Button {
    let p = panel(false);
    p.append(&text(recipe.lead(), 16, 600, &["tafs-ink"]));
    if !recipe.line.is_empty() {
        p.append(&text(&recipe.line, 14, 400, &["tafs-soft"]));
    }
    if !recipe.apps.is_empty() {
        p.append(&app_marks(recipe));
    }
    if let Some(color) = tint(&pane.recipes, &recipe.category) {
        p.set_widget_name(&crate::style::tint_name("recipe"));
        crate::style::tint_edge(&p, &color);
    }
    let b = gtk::Button::new();
    b.set_child(Some(&p));
    b.add_css_class("tafs-plain");
    let (pane, recipe) = (pane.clone(), recipe.clone());
    // Over the window it's in.
    b.connect_clicked(move |b| sheet(&pane, &recipe, b));
    b
}

/// `RecipeSheet`: the problem it ends (24 bold), its line (15), what it costs today (14, soft),
/// each zone of its Card in a panel, the proof run's line and what it reported, and the button
/// that hands it to Coach, for someone who may use Coach (`handRecipeToCoach`): the sheet closes,
/// and the gallery under it, and Coach opens over Home in a new conversation with the job's words
/// in its box. Nothing is sent.
pub fn sheet(pane: &RecipesPane, recipe: &Recipe, parent: &impl IsA<gtk::Widget>) {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 12);
    column.set_margin_start(20);
    column.set_margin_end(20);
    column.set_margin_top(8);
    column.set_margin_bottom(16);
    column.append(&text(recipe.lead(), 24, 700, &["tafs-ink"]));
    if !recipe.line.is_empty() {
        column.append(&text(&recipe.line, 15, 400, &["tafs-ink"]));
    }
    if !recipe.today.is_empty() {
        column.append(&text(&recipe.today, 14, 400, &["tafs-soft"]));
    }
    for zone in pane.recipes.zones(recipe) {
        let p = panel(false);
        if !zone.title.is_empty() {
            p.append(&eyebrow(&zone.title, false));
        }
        for line in &zone.lines {
            p.append(&text(line, 14, 400, &["tafs-ink"]));
        }
        column.append(&p);
    }
    if !recipe.proved.is_empty() {
        let h = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let seal = symbol("emblem-ok-symbolic", 14);
        seal.add_css_class("tafs-soft");
        h.append(&seal);
        h.append(&text(&recipe.proved, 14, 600, &["tafs-ink"]));
        column.append(&h);
    }
    if !recipe.sample.is_empty() {
        let p = panel(false);
        if let Some(label) = &pane.sample_label {
            p.append(&eyebrow(label, false));
        }
        p.append(&text(&recipe.sample, 14, 400, &["tafs-ink"]));
        column.append(&p);
    }
    let words = pane
        .hand_words
        .as_deref()
        .or_else(|| pane.recipes.hand_words());
    let dialog = adw::Dialog::new();
    if let (true, Some(words)) = (pane.may_use_coach, words) {
        let hand = primary(words);
        hand.set_widget_name("recipe.hand");
        let (say, sheet, under) = (
            recipe.say.clone(),
            dialog.downgrade(),
            parent
                .ancestor(adw::Dialog::static_type())
                .and_then(|d| d.downcast::<adw::Dialog>().ok()),
        );
        // Coach opens over Home, not over a sheet: the gallery under this one closes too.
        hand.connect_clicked(move |_| {
            if let Some(sheet) = sheet.upgrade() {
                sheet.close();
            }
            if let Some(gallery) = &under {
                gallery.close();
            }
            crate::coach_start::hand(&say);
        });
        column.append(&hand);
    }
    let clamp = crate::widgets::page_column(&column, crate::widgets::PageColumn::Gallery);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_child(Some(&clamp));
    scroller.set_vexpand(true);
    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&gtk::Label::new(None)));
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&scroller));
    crate::widgets::ground(&toolbar);
    dialog.set_child(Some(&toolbar));
    dialog.set_content_width(560);
    dialog.set_content_height(640);
    dialog.set_title(recipe.lead());
    dialog.present(Some(parent));
}

/// The jobs' gallery (`RecipeGallerySheet`), and what it draws from.
pub struct GalleryPane {
    pub gallery: tafs_core::recipes::Gallery,
    /// The sheet's words from `/v1/recipes`, for a gallery that sends none.
    pub labels: Option<tafs_core::recipes::Labels>,
    pub may_use_coach: bool,
    /// The sheet's Close, in the app's words.
    pub close: String,
}

/// Why the gallery is open (`GalleryMode`): from + New, to start a taf; or to browse the jobs
/// (Home's "Explore jobs" and the row under the strip), with nothing created until "Try it with
/// Coach" is tapped on one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GalleryMode {
    Create,
    Browse,
}

impl GalleryPane {
    /// The gallery as Home's list: browsing, each job's sheet with "Try it with Coach" as its one
    /// action; from + New, with the sheet's own words.
    fn list(&self, mode: GalleryMode) -> RecipesPane {
        let recipes = self.gallery.as_recipes(self.labels.as_ref());
        RecipesPane {
            hand_words: self
                .gallery
                .hand_words(mode == GalleryMode::Browse, recipes.labels.as_ref())
                .map(str::to_string),
            sample_label: self.gallery.sample_label().map(str::to_string),
            recipes,
            may_use_coach: self.may_use_coach,
        }
    }

    /// The own-words row's words, from + New, for someone who may use Coach; none otherwise.
    fn own_words(&self, mode: GalleryMode) -> Option<(&str, &str)> {
        let w = &self.gallery.words;
        (mode == GalleryMode::Create && self.may_use_coach && !w.own_words.is_empty())
            .then_some((w.own_words.as_str(), w.own_words_line.as_str()))
    }
}

/// `RecipeGallerySheet`: its title ("Explore jobs" browsing, else the gallery's own); from + New,
/// "Say it in your own words" with board 8 (a new taf) beside it; "All jobs" over every job, and
/// Home's list of them; Close at the top. A job's sheet hands it to Coach, and the own
/// words open Coach in a new conversation, once the gallery has closed: Coach opens over Home.
pub fn gallery(pane: &GalleryPane, mode: GalleryMode, parent: &impl IsA<gtk::Widget>) {
    let dialog = adw::Dialog::new();
    let mut head: Vec<gtk::Widget> = vec![];
    if let Some((title, line)) = pane.own_words(mode) {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let board = crate::illustrations::new_taf();
        board.set_hexpand(false);
        board.set_valign(gtk::Align::Center);
        row.append(&board);
        let own = own_words_row(title, line);
        own.set_hexpand(true);
        let gallery = dialog.downgrade();
        own.connect_clicked(move |_| {
            if let Some(gallery) = gallery.upgrade() {
                gallery.close();
            }
            crate::coach_start::new_taf();
        });
        row.append(&own);
        head.push(row.upcast());
    }
    if let Some(all) = Some(pane.gallery.words.all.trim()).filter(|a| !a.is_empty()) {
        head.push(eyebrow(all, false).upcast());
    }
    let list = list_page(Rc::new(pane.list(mode)), GALLERY, &head);
    list.set_vexpand(true);
    let title = pane.gallery.title(mode == GalleryMode::Browse);
    // Its bar as the app's sheets' (`cancellationAction`): Close at the start, in words, the title
    // in the middle, and no ✕ of the window's.
    let header = adw::HeaderBar::new();
    header.set_show_end_title_buttons(false);
    header.set_show_start_title_buttons(false);
    let heading = gtk::Label::new(Some(title));
    heading.add_css_class("heading");
    header.set_title_widget(Some(&heading));
    let close = gtk::Button::with_label(&pane.close);
    close.add_css_class("tafs-unsent-act");
    close.add_css_class("tafs-bar-text");
    close.set_margin_start(8);
    close.set_widget_name("gallery.close");
    {
        let d = dialog.clone();
        close.connect_clicked(move |_| {
            d.close();
        });
    }
    header.pack_start(&close);
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&list));
    crate::widgets::ground(&toolbar);
    dialog.set_child(Some(&toolbar));
    dialog.set_content_width(680);
    dialog.set_content_height(720);
    dialog.set_title(title);
    dialog.set_widget_name(match mode {
        GalleryMode::Create => "gallery.create",
        GalleryMode::Browse => "gallery.browse",
    });
    dialog.present(Some(parent));
}

/// A job from the strip opens its sheet (`openRecipe`): the gallery's jobs, with the sheet's own
/// words to hand it to Coach.
pub fn strip_sheet(pane: &GalleryPane, recipe: &Recipe, parent: &impl IsA<gtk::Widget>) {
    let recipes = pane.gallery.as_recipes(pane.labels.as_ref());
    let sheet_pane = RecipesPane {
        recipes,
        may_use_coach: pane.may_use_coach,
        hand_words: None,
        sample_label: pane.gallery.sample_label().map(str::to_string),
    };
    sheet(&sheet_pane, recipe, parent);
}

/// The category colour of a strip's job, for its 4-wide bar; the line's colour without one.
pub fn strip_tint(pane: &GalleryPane, recipe: &Recipe) -> Option<String> {
    let color = pane.gallery.category_of(recipe)?.color?;
    let dark = adw::StyleManager::default().is_dark();
    Some(format!(
        "#{:06X}",
        if dark { color.dark } else { color.light }
    ))
}
