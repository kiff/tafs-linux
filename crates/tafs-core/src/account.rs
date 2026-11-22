//! Account (`SettingsView` in `SignInView.swift`), the KIFF account it shows
//! (`KIFFClient.Account`, `GET /v1/me`), and who is signed in (`AccountLines`).

use serde::Deserialize;

use crate::sidebar::Mode;
use crate::signin::refusal;
use crate::signin::{Answer, KiffError};
use crate::words::{WordBook, WordsError};

/// The KIFF account this session is in (`KIFFClient.Account`), and how the session approves
/// (contract §1.2: `GET /v1/me` says it as `approval_mode`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Account {
    pub tenant_id: String,
    #[serde(rename = "tenant_slug")]
    pub slug: String,
    #[serde(rename = "tenant_display_name")]
    pub display_name: Option<String>,
    pub email: Option<String>,
    #[serde(default)]
    pub time_zone: Option<String>,
    #[serde(default)]
    pub capabilities: Option<Vec<String>>,
    #[serde(default)]
    pub approval_mode: Option<String>,
}

impl Account {
    /// `GET /v1/me`'s answer; KIFF's refusal by its status and code.
    pub fn read(answer: &Answer) -> Result<Account, KiffError> {
        if !(200..300).contains(&answer.status) {
            return Err(refusal(answer, None));
        }
        serde_json::from_slice(&answer.body).map_err(|_| KiffError::Unreadable)
    }

    /// The account as the owner knows it: its name, else its slug.
    pub fn name(&self) -> String {
        match self.display_name.as_deref() {
            Some(n) if !n.is_empty() => n.to_string(),
            _ => self.slug.clone(),
        }
    }

    /// `AppModel.allows`: KIFF's capabilities, none said counting as allowed.
    pub fn allows(&self, capability: &str) -> bool {
        self.capabilities
            .as_ref()
            .is_none_or(|c| c.iter().any(|x| x == capability))
    }
}

/// `AccountLines.signedInAs`: the email under the account's name; none when the account is
/// named after it (a Solo account), so it isn't shown twice, or when KIFF sent none.
pub fn signed_in_as(account: &Account) -> Option<String> {
    let email = account.email.as_deref()?.trim();
    if email.is_empty() {
        return None;
    }
    (email.to_lowercase() != account.name().trim().to_lowercase()).then(|| email.to_string())
}

/// The languages kiff.dev has tafs's Terms and Privacy policy in, besides English, at
/// `kiff.dev/<lang>/tafs/terms` and `/<lang>/tafs/privacy`, by KIFF's code: Traditional Chinese
/// is served under "zh", as pt-BR is under "pt" (`Legal.translated`). The English pages govern.
pub const LEGAL_LANGUAGES: [&str; 24] = [
    "de", "es", "fr", "it", "nl", "pl", "pt", "ko", "ja", "et", "lt", "lv", "cs", "sk", "sl", "hu",
    "ro", "bg", "hr", "tr", "id", "vi", "th", "zh",
];

/// The code a legal page is served under, if kiff.dev has it in the app's language.
fn legal_code(language: &str) -> Option<&str> {
    let code = crate::language::kiff_code(language);
    LEGAL_LANGUAGES.contains(&code).then_some(code)
}

/// A page of tafs's on kiff.dev in the app's language where it's served, else in English at the
/// unprefixed path, never `/en/` (`Legal.page`).
fn legal_page(language: &str, name: &str) -> String {
    match legal_code(language) {
        Some(code) => format!("https://kiff.dev/{code}/tafs/{name}"),
        None => format!("https://kiff.dev/tafs/{name}"),
    }
}

/// tafs's own Terms, in the app's language where kiff.dev has it, else in English.
pub fn terms_url(language: &str) -> String {
    legal_page(language, "terms")
}

/// tafs's own Privacy policy, in the app's language where kiff.dev has it, else in English.
pub fn privacy_url(language: &str) -> String {
    legal_page(language, "privacy")
}

/// A legal link's label: as it is where kiff.dev has the page in the app's language (or in
/// English), and saying the page is in English in any other language (`Legal.label`).
pub fn legal_label(w: &WordBook, key: &str) -> Result<String, WordsError> {
    let words = w.line(key)?;
    if w.language() == "en" || legal_code(w.language()).is_some() {
        Ok(words)
    } else {
        w.get("settings.legal_in_english", &[("label", &words)])
    }
}

/// The language's own name, capitalised in it (`AppLanguage.name`, CLDR's): a regional variant
/// the app speaks beside its language is named with its region, "Português (Brasil)".
pub fn language_name(language: &str) -> &'static str {
    match language {
        "de" => "Deutsch",
        "fr" => "Français",
        "es" => "Español",
        "it" => "Italiano",
        "pt" => "Português",
        "nl" => "Nederlands",
        "pl" => "Polski",
        "ko" => "한국어",
        "ja" => "日本語",
        "et" => "Eesti",
        "lt" => "Lietuvių",
        "lv" => "Latviešu",
        "cs" => "Čeština",
        "sk" => "Slovenčina",
        "sl" => "Slovenščina",
        "hu" => "Magyar",
        "ro" => "Română",
        "bg" => "Български",
        "hr" => "Hrvatski",
        "tr" => "Türkçe",
        "id" => "Indonesia",
        "vi" => "Tiếng Việt",
        "th" => "ไทย",
        "zh-Hant" => "中文（繁體）",
        "pt-BR" => "Português (Brasil)",
        "ar" => "العربية",
        _ => "English",
    }
}

/// One of the screen's rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    /// A label and its value (`LabeledContent`).
    Value(String, String),
    /// A row that opens a screen (`NavigationLink`), with its value when it has one.
    Opens {
        title: String,
        value: Option<String>,
        screen: Screen,
    },
    /// The theme's three choices (`ThemePicker`).
    Theme {
        light: String,
        dark: String,
        system: String,
    },
    /// Words in the soft colour.
    Text(String),
    /// A link to a page, in the link colour.
    Link(String, String),
    /// A button in the amber (`Change language`, Sign in with KIFF).
    Action(String, Action),
    /// A destructive button, in red (Sign out, Clear the sample).
    Destructive(String, Action),
    /// A line in red (why signing out didn't work).
    Problem(String),
    /// `PlanRow`, once the balance is read: tafs cloud's `plan_line` (16 semibold) in
    /// `plan_tone`'s colour, and Plan & usage under it (13, amber). It opens Plan & usage.
    Plan {
        line: String,
        tone: crate::usage::Tone,
        title: String,
    },
}

/// What an Opens row opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    KiffAccount,
    People,
    Devices,
    ConnectApps,
    Usage,
    Alerts,
    TimeZone,
    /// Account › Language (`LanguagePicker`).
    Language,
    /// How tafs works: the three cards again and the legal pages (`HowTafsWorksView`).
    HowItWorks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    SignIn,
    SignOut,
    ClearSample,
    ChangeLanguage,
    /// A language picked in Account › Language, or this computer's own with `None`.
    UseLanguage(Option<&'static str>),
}

/// One of Account › Language's rows (`LanguagePicker`): the language it picks (`None`, this
/// computer's own), its name, and whether it's the one in use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageRow {
    pub code: Option<&'static str>,
    pub name: String,
    pub picked: bool,
}

/// Account › Language, as the iPad's `LanguagePicker`: this computer's language first, then the
/// app's languages in the order tafs lists them, each named in itself, the one in use marked.
/// `chosen` is the language picked in the app; none means this computer's.
pub fn language_rows(w: &WordBook, chosen: Option<&str>) -> Result<Vec<LanguageRow>, WordsError> {
    let device = w.line("device.linux")?;
    let mut rows = vec![LanguageRow {
        code: None,
        name: w.get("settings.language_device", &[("device", &device)])?,
        picked: chosen.is_none(),
    }];
    rows.extend(crate::words::SHIPPED.iter().map(|code| LanguageRow {
        code: Some(*code),
        name: language_name(code).to_string(),
        picked: chosen == Some(*code),
    }));
    Ok(rows)
}

/// One section: its header, when it has one, and its rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub header: Option<String>,
    pub rows: Vec<Row>,
    /// A part read from tafs cloud when Account opens: drawn under the header once read, and the
    /// whole section left out when it can't be, as the iPad draws none.
    pub slot: Option<Slot>,
}

/// The parts of Account read when it opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    /// Coach's "Show suggestions" (`setSuggestions`), for someone who may use Coach.
    Suggestions,
    /// The AI-data consent's row (`AIConsentRow`): given, with Withdraw; else its screen.
    Consent,
    /// Export my chats (`ExportChatsSection`), with what it exports under it.
    Export,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountLines {
    pub title: String,
    pub sections: Vec<Section>,
}

/// `SettingsView`, in the iPad's order. Coach's suggestions and the AI-data consent are read when
/// Account opens (`Slot`), and drawn only once read, as the iPad draws none until then.
pub fn lines(
    w: &WordBook,
    mode: Mode,
    account: Option<&Account>,
    sign_out_error: Option<&str>,
    terms: &str,
    privacy: &str,
    version: &str,
) -> Result<AccountLines, WordsError> {
    let l = |k: &str| w.line(k);
    let kiff = mode == Mode::Kiff;
    let mut sections = Vec::new();

    let mode_words = l(match mode {
        Mode::Kiff => "settings.mode_kiff",
        Mode::Sample => "settings.mode_sample",
        Mode::SignedOut => "settings.mode_signed_out",
    })?;
    let mut yours = vec![Row::Value(l("settings.mode")?, mode_words)];
    if let (true, Some(a)) = (kiff, account) {
        yours.push(Row::Opens {
            title: l("settings.kiff_account")?,
            value: Some(a.name()),
            screen: Screen::KiffAccount,
        });
        if let Some(email) = signed_in_as(a) {
            yours.push(Row::Value(l("settings.signed_in_as")?, email));
        }
    }
    sections.push(Section {
        slot: None,
        header: Some(l("settings.your_account")?),
        rows: yours,
    });

    if kiff {
        sections.push(Section {
            slot: None,
            header: Some(l("settings.people_apps")?),
            rows: vec![
                Row::Opens {
                    title: l("people.title")?,
                    value: None,
                    screen: Screen::People,
                },
                Row::Opens {
                    title: l("settings.devices.title")?,
                    value: None,
                    screen: Screen::Devices,
                },
                Row::Opens {
                    title: l("connect.apps.title")?,
                    value: None,
                    screen: Screen::ConnectApps,
                },
            ],
        });
        // `PlanRow`: just the title until the balance is read.
        sections.push(Section {
            slot: None,
            header: Some(l("settings.plan")?),
            rows: vec![Row::Opens {
                title: l("usage.title")?,
                value: None,
                screen: Screen::Usage,
            }],
        });
        // What a new Coach conversation offers, as tafs cloud keeps it: once read, for someone
        // who may use Coach.
        sections.push(Section {
            slot: Some(Slot::Suggestions),
            header: Some(l("coach.title")?),
            rows: vec![],
        });
    }
    sections.push(Section {
        slot: None,
        header: None,
        rows: vec![Row::Opens {
            title: l("settings.alerts")?,
            value: None,
            screen: Screen::Alerts,
        }],
    });
    sections.push(Section {
        slot: None,
        header: Some(l("settings.theme_title")?),
        rows: vec![Row::Theme {
            light: l("settings.theme.light")?,
            dark: l("settings.theme.dark")?,
            system: l("settings.theme.system")?,
        }],
    });
    sections.push(Section {
        slot: None,
        header: Some(l("settings.language_title")?),
        rows: vec![
            Row::Value(
                l("settings.language_title")?,
                language_name(w.language()).to_string(),
            ),
            Row::Action(l("settings.change_language")?, Action::ChangeLanguage),
        ],
    });
    // The AI-data consent: given, with Withdraw; not given, its screen. tafs cloud's words.
    if kiff {
        sections.push(Section {
            slot: Some(Slot::Consent),
            header: None,
            rows: vec![],
        });
    }
    if kiff {
        sections.push(Section {
            slot: None,
            header: None,
            rows: vec![Row::Opens {
                title: l("settings.time_zone_title")?,
                value: Some(
                    account
                        .and_then(|a| a.time_zone.clone())
                        .unwrap_or_else(|| "UTC".into()),
                ),
                screen: Screen::TimeZone,
            }],
        });
        // Their own copy of their chats (`ExportChatsSection`).
        sections.push(Section {
            slot: Some(Slot::Export),
            header: None,
            rows: vec![],
        });
    }
    // This computer is view-only: its approvals are signed on the phone, so it says so
    // (`sign_in.every_action_taf_takes_goes_view`), never that they're signed here.
    sections.push(Section {
        slot: None,
        header: Some(l("sign_in.how_works")?),
        rows: vec![
            Row::Text(l("sign_in.every_action_taf_takes_goes_view")?),
            // The three cards again, with the privacy and terms pages (`HowTafsWorksView`).
            Row::Opens {
                title: l("onboarding.how_it_works")?,
                value: None,
                screen: Screen::HowItWorks,
            },
        ],
    });
    // tafs's own pages, in the app's language where kiff.dev has them.
    let legal = |key: &str| legal_label(w, key);
    sections.push(Section {
        slot: None,
        header: Some(l("settings.legal")?),
        rows: vec![
            Row::Link(legal("settings.terms")?, terms.to_string()),
            Row::Link(legal("settings.privacy_policy")?, privacy.to_string()),
        ],
    });
    sections.push(Section {
        slot: None,
        header: Some(l("settings.about")?),
        rows: vec![
            Row::Value(l("settings.version")?, version.to_string()),
            Row::Text(l("home.tagline")?),
        ],
    });
    let mut actions = Vec::new();
    if mode == Mode::Sample {
        actions.push(Row::Action(l("sign_in.with_kiff")?, Action::SignIn));
    }
    actions.push(if kiff {
        Row::Destructive(l("settings.sign_out")?, Action::SignOut)
    } else {
        Row::Destructive(l("settings.clear_sample")?, Action::ClearSample)
    });
    if let Some(why) = sign_out_error {
        actions.push(Row::Problem(why.to_string()));
    }
    sections.push(Section {
        slot: None,
        header: Some(l("settings.account_actions")?),
        rows: actions,
    });
    Ok(AccountLines {
        title: l("settings.title")?,
        sections,
    })
}

/// `PlanRow` once tafs cloud's balance is read with a `plan_line`: the Plan section's row says
/// it, in `plan_tone`'s colour, with Plan & usage under it. Without one, the title alone stays.
pub fn with_plan(
    mut lines: AccountLines,
    w: &WordBook,
    balance: Option<&crate::usage::Balance>,
) -> Result<AccountLines, WordsError> {
    let Some((line, tone)) = balance.and_then(|b| Some((b.plan_line.clone()?, &b.plan_tone)))
    else {
        return Ok(lines);
    };
    let title = w.line("usage.title")?;
    for section in &mut lines.sections {
        for row in &mut section.rows {
            if matches!(
                row,
                Row::Opens {
                    screen: Screen::Usage,
                    ..
                }
            ) {
                *row = Row::Plan {
                    line: line.clone(),
                    tone: crate::usage::tone(tone),
                    title: title.clone(),
                };
            }
        }
    }
    Ok(lines)
}

/// The KIFF account's own screen (`AccountView`): its name, its id when it isn't its name, who is
/// signed in; then its time zone (`time_zone`, UTC when KIFF sends none) and, when this computer
/// is elsewhere, the line to use this computer's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KiffAccountLines {
    pub rows: Vec<(String, String)>,
    pub zone: String,
    pub use_this_zone: Option<String>,
}

pub fn kiff_account_lines(
    w: &WordBook,
    account: &Account,
    here: &str,
) -> Result<KiffAccountLines, WordsError> {
    let mut rows = vec![(w.line("settings.kiff_account")?, account.name())];
    if account.name() != account.slug {
        rows.push((w.line("settings.account_id")?, account.slug.clone()));
    }
    if let Some(email) = signed_in_as(account) {
        rows.push((w.line("settings.signed_in_as")?, email));
    }
    let zone = account.time_zone.clone().unwrap_or_else(|| "UTC".into());
    let use_this_zone = if zone != here {
        let device = w.line("device.linux")?;
        Some(w.get(
            "settings.use_device_zone",
            &[("device", &device), ("identifier", here)],
        )?)
    } else {
        None
    };
    Ok(KiffAccountLines {
        rows,
        zone,
        use_this_zone,
    })
}

/// The sign-out's question (`SignOutWay.signOut`): its title, why, and its button.
pub fn sign_out_question(w: &WordBook) -> Result<(String, String, String), WordsError> {
    let device = w.line("device.linux")?;
    Ok((
        w.line("settings.sign_out_of_kiff.title")?,
        w.get(
            "settings.sign_out_of_kiff.body_device",
            &[("device", &device)],
        )?,
        w.line("settings.sign_out")?,
    ))
}

/// Why signing out didn't work (`notSignedOutDevice`).
pub fn not_signed_out(w: &WordBook, e: &KiffError) -> Result<String, WordsError> {
    let device = w.line("device.linux")?;
    w.get(
        "sign_in.not_signed_out_device",
        &[("why", &e.words(w)?), ("device", &device)],
    )
}

/// What signing out does at KIFF (`AppModel.signOut`): a 401 means KIFF had already ended it.
pub fn signed_out(result: Result<(), KiffError>) -> Result<(), KiffError> {
    match result {
        Ok(()) | Err(KiffError::Unauthorized) => Ok(()),
        Err(e) => Err(e),
    }
}

/// `DeleteAccountView`'s words: the title, the warning, and what goes, each with its symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteLines {
    pub title: String,
    pub warning: String,
    pub what_goes: String,
    /// (SF symbol, words): tafs, sign-ins, Cards, records.
    pub goes: Vec<(&'static str, String)>,
    pub cancel: String,
}

pub fn delete_lines(w: &WordBook) -> Result<DeleteLines, WordsError> {
    Ok(DeleteLines {
        title: w.line("settings.delete_account.title")?,
        warning: w.line("settings.delete_account.warning")?,
        what_goes: w.line("settings.delete_account.what_goes")?,
        goes: vec![
            ("person.2", w.line("settings.delete_account.goes_tafs")?),
            ("link", w.line("settings.delete_account.goes_sign_ins")?),
            ("creditcard", w.line("settings.delete_account.goes_cards")?),
            ("doc.text", w.line("settings.delete_account.goes_records")?),
        ],
        cancel: w.line("common.cancel")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(name: Option<&str>, email: Option<&str>) -> Account {
        Account {
            tenant_id: "t".into(),
            slug: "acme".into(),
            display_name: name.map(str::to_string),
            email: email.map(str::to_string),
            time_zone: None,
            capabilities: None,
            approval_mode: None,
        }
    }

    #[test]
    fn who_is_signed_in_is_said_once() {
        assert_eq!(
            signed_in_as(&account(Some("Acme"), Some("rae@acme.test"))).as_deref(),
            Some("rae@acme.test")
        );
        assert_eq!(
            signed_in_as(&account(Some("Rae@Acme.test"), Some("rae@acme.test"))),
            None
        );
        assert_eq!(signed_in_as(&account(Some("Acme"), Some("  "))), None);
        assert_eq!(account(Some(""), None).name(), "acme");
    }

    #[test]
    fn a_401_at_sign_out_means_kiff_already_ended_it() {
        assert_eq!(signed_out(Err(KiffError::Unauthorized)), Ok(()));
        assert_eq!(signed_out(Err(KiffError::Offline)), Err(KiffError::Offline));
    }

    #[test]
    fn me_reads_its_approval_mode() {
        let a = Account::read(&Answer {
            status: 200,
            body: br#"{"tenant_id":"t1","tenant_slug":"acme","tenant_display_name":"Acme","email":"r@acme.test","approval_mode":"view","time_zone":"Europe/Paris"}"#.to_vec(),
        })
        .unwrap();
        assert_eq!(a.approval_mode.as_deref(), Some("view"));
        assert_eq!(a.name(), "Acme");
        assert!(a.allows("holds.answer"));
        assert_eq!(
            Account::read(&Answer {
                status: 401,
                body: vec![]
            }),
            Err(KiffError::Unauthorized)
        );
    }

    #[test]
    fn the_language_picker_lists_this_computer_s_then_the_app_s() {
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        let rows = language_rows(&w, None).unwrap();
        assert_eq!(rows.len(), crate::words::SHIPPED.len() + 1);
        assert_eq!(rows[0].code, None);
        assert_eq!(rows[0].name, "This computer's language");
        assert!(rows[0].picked);
        assert_eq!(
            rows[1..]
                .iter()
                .map(|r| r.code.unwrap())
                .collect::<Vec<_>>(),
            crate::words::SHIPPED.to_vec()
        );
        assert_eq!(rows[3].name, "Français");
        let rows = language_rows(&w, Some("pl")).unwrap();
        assert_eq!(
            rows.iter()
                .filter(|r| r.picked)
                .map(|r| r.code)
                .collect::<Vec<_>>(),
            vec![Some("pl")]
        );
        for lang in crate::words::SHIPPED {
            let w = WordBook::load(&WordBook::default_root(), lang).unwrap();
            assert!(!language_rows(&w, None).unwrap()[0].name.contains('{'));
        }
    }

    #[test]
    fn deleting_says_what_goes_in_every_language() {
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        let d = delete_lines(&w).unwrap();
        assert_eq!(d.title, "Delete account");
        assert_eq!(
            d.warning,
            "This deletes your KIFF account. It can't be undone."
        );
        assert_eq!(
            d.goes.iter().map(|(s, _)| *s).collect::<Vec<_>>(),
            ["person.2", "link", "creditcard", "doc.text"]
        );
        assert_eq!(d.goes[0].1, "Tafs, runs, chats and notes");
        for lang in crate::words::SHIPPED {
            delete_lines(&WordBook::load(&WordBook::default_root(), lang).unwrap()).unwrap();
        }
    }
}
