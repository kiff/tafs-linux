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

/// The language's own name, capitalised in it (`AppLanguage.name`).
pub fn language_name(language: &str) -> &'static str {
    match language {
        "de" => "Deutsch",
        "fr" => "Français",
        "es" => "Español",
        "it" => "Italiano",
        "pt" => "Português",
        "nl" => "Nederlands",
        "pl" => "Polski",
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
    Link(String, &'static str),
    /// A button in the amber (`Change language`, Sign in with KIFF).
    Action(String, Action),
    /// A destructive button, in red (Sign out, Clear the sample).
    Destructive(String, Action),
    /// A line in red (why signing out didn't work).
    Problem(String),
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    SignIn,
    SignOut,
    ClearSample,
    ChangeLanguage,
}

/// One section: its header, when it has one, and its rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub header: Option<String>,
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountLines {
    pub title: String,
    pub sections: Vec<Section>,
}

/// `SettingsView`, in the iPad's order, the sections whose reads are ported: what isn't read
/// yet (Coach's suggestions, the AI-data consent, Export my chats) isn't drawn, as the iPad
/// draws none until it's read.
pub fn lines(
    w: &WordBook,
    mode: Mode,
    account: Option<&Account>,
    sign_out_error: Option<&str>,
    terms: &'static str,
    privacy: &'static str,
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
        header: Some(l("settings.your_account")?),
        rows: yours,
    });

    if kiff {
        sections.push(Section {
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
            header: Some(l("settings.plan")?),
            rows: vec![Row::Opens {
                title: l("usage.title")?,
                value: None,
                screen: Screen::Usage,
            }],
        });
    }
    sections.push(Section {
        header: None,
        rows: vec![Row::Opens {
            title: l("settings.alerts")?,
            value: None,
            screen: Screen::Alerts,
        }],
    });
    sections.push(Section {
        header: Some(l("settings.theme_title")?),
        rows: vec![Row::Theme {
            light: l("settings.theme.light")?,
            dark: l("settings.theme.dark")?,
            system: l("settings.theme.system")?,
        }],
    });
    sections.push(Section {
        header: Some(l("settings.language_title")?),
        rows: vec![
            Row::Value(
                l("settings.language_title")?,
                language_name(w.language()).to_string(),
            ),
            Row::Action(l("settings.change_language")?, Action::ChangeLanguage),
        ],
    });
    if kiff {
        sections.push(Section {
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
    }
    let device = l("device.linux")?;
    sections.push(Section {
        header: Some(l("sign_in.how_works")?),
        rows: vec![Row::Text(w.get(
            "sign_in.every_action_taf_takes_goes",
            &[("device", &device)],
        )?)],
    });
    // tafs's own pages, in English only for now: outside English the rows say so.
    let legal = |key: &str| -> Result<String, WordsError> {
        let words = l(key)?;
        if w.language() == "en" {
            Ok(words)
        } else {
            w.get("settings.legal_in_english", &[("label", &words)])
        }
    };
    sections.push(Section {
        header: Some(l("settings.legal")?),
        rows: vec![
            Row::Link(legal("settings.terms")?, terms),
            Row::Link(legal("settings.privacy_policy")?, privacy),
        ],
    });
    sections.push(Section {
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
        header: Some(l("settings.account_actions")?),
        rows: actions,
    });
    Ok(AccountLines {
        title: l("settings.title")?,
        sections,
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
}
