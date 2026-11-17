//! Sharing a taf (`ShareViews.swift`: `ShareLine`, `ShareTafSheet`): who a taf is shared with, on
//! its page, and the two steps of sharing it, who uses it, then what that means. The shares are
//! read as the session (`GET /v1/agents/{agent}/grants`); changing them is signed (`signsSetup`:
//! the grants route), so this computer draws the sheet up to the signature and sends nothing.

use crate::access::Share;
use crate::people::{managers_first, Group, Person};
use crate::signed::Act;
use crate::words::{list, WordBook, WordsError};

/// `ShareLine.sharedWith`: "Shared with Support and ana@acme.com", or not shared yet; "Sharing…"
/// while the shares aren't read. The person reading it is never named: a taf only they use is
/// "Not shared". Shared with KIFF's default group alone is its own sentence.
pub fn shared_with(
    w: &WordBook,
    shares: Option<&[Share]>,
    groups: &[Group],
    you: &dyn Fn(&str) -> bool,
    label: &dyn Fn(&str) -> String,
) -> Result<String, WordsError> {
    let Some(shares) = shares else {
        return w.line("people.sharing");
    };
    if let [only] = shares {
        if let Some(g) = &only.group {
            if groups
                .iter()
                .find(|x| &x.id == g)
                .is_some_and(|x| x.everyone)
            {
                return w.line("people.shared_with_everyone");
            }
        }
    }
    let mut names = Vec::new();
    for s in shares {
        // `s.group.flatMap { group's shownName } ?? s.subject.flatMap { … }`: a group that isn't
        // listed falls through to its subject, which a group's share has none of.
        let group = match s
            .group
            .as_ref()
            .and_then(|g| groups.iter().find(|x| &x.id == g))
        {
            Some(g) => Some(g.shown_name(w)?),
            None => None,
        };
        match (group, &s.subject) {
            (Some(name), _) => names.push(name),
            (None, Some(subject)) if !you(subject) => names.push(label(subject)),
            _ => {}
        }
    }
    if names.is_empty() {
        return w.line("people.not_shared");
    }
    Ok(format!(
        "{}{}",
        w.line("people.shared_with_prefix")?,
        list(w.language(), &names)
    ))
}

/// Whether `subject` is the person reading (`ShareLine.sharedWith`'s `you`): KIFF marks them, or
/// their email is the account's.
pub fn is_you(subject: &str, people: &[Person], email: Option<&str>) -> bool {
    let email = email.map(str::to_lowercase);
    let person = people.iter().find(|p| p.subject == subject);
    person.is_some_and(|p| p.you)
        || email.as_ref().is_some_and(|e| {
            person
                .and_then(|p| p.email.as_ref())
                .map(|m| m.to_lowercase())
                == Some(e.clone())
                || subject.to_lowercase() == *e
        })
}

/// `model.person(subject)?.label ?? subject`.
pub fn label(subject: &str, people: &[Person]) -> String {
    people
        .iter()
        .find(|p| p.subject == subject)
        .map(|p| p.label().to_string())
        .unwrap_or_else(|| subject.to_string())
}

/// `ShareTafSheet.people(of:)`: "Maria (manager), Ana, Luis", managers first; "No one yet".
pub fn group_people(w: &WordBook, group: &Group, people: &[Person]) -> Result<String, WordsError> {
    let mut names = Vec::new();
    for m in managers_first(&group.members, people) {
        let l = label(&m.subject, people);
        names.push(if m.is_manager() {
            w.get("people.name_manager", &[("name", &l)])?
        } else {
            l
        });
    }
    if names.is_empty() {
        w.line("people.no_one_yet")
    } else {
        Ok(names.join(", "))
    }
}

/// `ShareTafSheet.consequences`: what the change means, as it holds today: each person's own
/// chats, then who gets it (by group or subject, in that order), then who stops using it.
pub fn consequences(
    w: &WordBook,
    taf: &str,
    had: &[Share],
    chosen: &[Share],
    groups: &[Group],
    people: &[Person],
) -> Result<Vec<String>, WordsError> {
    let mut lines = Vec::new();
    if !chosen.is_empty() {
        lines.push(w.line("people.own_chats")?);
    }
    let key = |s: &Share| s.group.clone().or(s.subject.clone()).unwrap_or_default();
    let mut sorted: Vec<&Share> = chosen.iter().collect();
    sorted.sort_by_key(|s| key(s));
    for share in sorted {
        let group = share
            .group
            .as_ref()
            .and_then(|id| groups.iter().find(|g| &g.id == id));
        match (group, &share.subject) {
            (Some(g), _) if g.everyone => lines.push(w.get(
                "people.everyone_uses",
                &[("taf", taf), ("people", &group_people(w, g, people)?)],
            )?),
            (Some(g), _) => lines.push(w.get(
                "people.group_uses",
                &[
                    ("group", &g.shown_name(w)?),
                    ("taf", taf),
                    ("people", &group_people(w, g, people)?),
                ],
            )?),
            (None, Some(subject)) if share.group.is_none() => lines.push(w.get(
                "people.person_uses",
                &[("person", &label(subject, people)), ("taf", taf)],
            )?),
            _ => {}
        }
    }
    for share in had.iter().filter(|s| !chosen.contains(s)) {
        let name = match share
            .group
            .as_ref()
            .and_then(|g| groups.iter().find(|x| &x.id == g))
        {
            Some(g) => g.shown_name(w)?,
            None => match (&share.group, &share.subject) {
                (None, Some(subject)) => label(subject, people),
                _ => String::new(),
            },
        };
        lines.push(w.get("people.stops_using", &[("who", &name), ("taf", taf)])?);
    }
    Ok(lines)
}

/// Whether the two hold the same shares, in any order (`chosen == had`, as sets).
pub fn same(a: &[Share], b: &[Share]) -> bool {
    a.iter().all(|s| b.contains(s)) && b.iter().all(|s| a.contains(s))
}

/// Picked or not, in the sheet's first step (`chosen.insert`, `remove`).
pub fn toggle(chosen: &mut Vec<Share>, share: Share) {
    match chosen.iter().position(|s| *s == share) {
        Some(i) => {
            chosen.remove(i);
        }
        None => chosen.push(share),
    }
}

/// `save`'s calls, each signed on the iPad (one Face ID for all): each share added, then each
/// taken away. This computer makes none of them.
pub fn acts(agent: &str, had: &[Share], chosen: &[Share]) -> Vec<Act> {
    let add = chosen
        .iter()
        .filter(|s| !had.contains(s))
        .map(|s| Act::Share {
            agent: agent.to_string(),
            group: s.group.clone(),
            subject: s.subject.clone(),
        });
    let remove = had
        .iter()
        .filter(|s| !chosen.contains(s))
        .map(|s| Act::Unshare {
            agent: agent.to_string(),
            group: s.group.clone(),
            subject: s.subject.clone(),
        });
    add.chain(remove).collect()
}

/// The sheet's words (`ShareTafSheet`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetWords {
    pub title: String,
    pub step1: String,
    pub who_uses: String,
    pub groups: String,
    pub one_person: String,
    pub choose_someone: String,
    pub step2: String,
    pub who_gets: String,
    pub not_shared: String,
    pub cancel: String,
    pub back: String,
    pub next: String,
    pub share: String,
}

pub fn sheet_words(w: &WordBook, taf: &str) -> Result<SheetWords, WordsError> {
    Ok(SheetWords {
        title: w.get("people.share.title", &[("taf", taf)])?,
        step1: w.line("people.share.step1")?,
        who_uses: w.get("people.share.who_uses", &[("taf", taf)])?,
        groups: w.line("people.groups")?,
        one_person: w.line("people.one_person")?,
        choose_someone: w.line("people.share.choose_someone")?,
        step2: w.line("people.share.step2")?,
        who_gets: w.get("people.share.who_gets", &[("taf", taf)])?,
        not_shared: w.get("people.share.not_shared", &[("taf", taf)])?,
        cancel: w.line("common.cancel")?,
        back: w.line("common.back")?,
        next: w.line("common.next")?,
        share: w.line("people.share.action")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::people::Member;

    fn words() -> WordBook {
        WordBook::load(&WordBook::default_root(), "en").unwrap()
    }

    fn group(id: &str, name: &str, everyone: bool, members: &[(&str, &str)]) -> Group {
        Group {
            id: id.into(),
            name: name.into(),
            everyone,
            members: members
                .iter()
                .map(|(s, r)| Member {
                    subject: s.to_string(),
                    role: r.to_string(),
                })
                .collect(),
            agents: vec![],
        }
    }

    fn person(subject: &str, email: &str, you: bool) -> Person {
        Person {
            subject: subject.into(),
            role: "member".into(),
            status: "active".into(),
            email: Some(email.into()),
            you,
        }
    }

    fn g(id: &str) -> Share {
        Share {
            group: Some(id.into()),
            subject: None,
        }
    }

    fn s(subject: &str) -> Share {
        Share {
            group: None,
            subject: Some(subject.into()),
        }
    }

    #[test]
    fn the_line_names_who_it_is_shared_with_but_never_the_reader() {
        let w = words();
        let groups = vec![
            group("g1", "Support", false, &[]),
            group("all", "Everyone", true, &[]),
        ];
        let people = vec![
            person("s1", "ana@acme.com", false),
            person("me", "me@acme.com", true),
        ];
        let you = |x: &str| is_you(x, &people, Some("ME@acme.com"));
        let label = |x: &str| label(x, &people);
        let line =
            |shares: Option<&[Share]>| shared_with(&w, shares, &groups, &you, &label).unwrap();
        assert_eq!(line(None), "Sharing…");
        assert_eq!(line(Some(&[])), "Not shared");
        assert_eq!(line(Some(&[s("me")])), "Not shared");
        assert_eq!(line(Some(&[g("all")])), "Shared with everyone");
        assert_eq!(
            line(Some(&[g("g1"), s("s1"), s("me")])),
            "Shared with Support and ana@acme.com"
        );
        // A subject KIFF doesn't list stands as its id.
        assert_eq!(line(Some(&[s("auth0|x")])), "Shared with auth0|x");
        // The reader by email alone, any case.
        assert!(is_you("ME@ACME.COM", &[], Some("me@acme.com")));
        assert!(!is_you("s1", &people, None));
    }

    #[test]
    fn the_second_step_says_what_the_change_means() {
        let w = words();
        let groups = vec![
            group(
                "g1",
                "Support",
                false,
                &[("s2", "member"), ("s1", "manager")],
            ),
            group("all", "Everyone", true, &[]),
        ];
        let people = vec![
            person("s1", "maria@acme.com", false),
            person("s2", "ana@acme.com", false),
        ];
        assert_eq!(
            group_people(&w, &groups[0], &people).unwrap(),
            "maria@acme.com (manager), ana@acme.com"
        );
        assert_eq!(group_people(&w, &groups[1], &people).unwrap(), "No one yet");
        let lines = consequences(
            &w,
            "Refunds",
            &[s("s2")],
            &[g("g1"), s("s1")],
            &groups,
            &people,
        )
        .unwrap();
        assert_eq!(
            lines,
            [
                "Each person gets their own chats and notes. No one else sees them in the app.",
                "Support uses Refunds: maria@acme.com (manager), ana@acme.com.",
                "maria@acme.com uses Refunds.",
                "ana@acme.com stops using Refunds. Their chats stay theirs.",
            ]
        );
        let none = consequences(&w, "Refunds", &[g("all")], &[], &groups, &people).unwrap();
        assert_eq!(
            none,
            ["Everyone stops using Refunds. Their chats stay theirs."]
        );
        assert_eq!(
            consequences(&w, "Refunds", &[], &[g("all")], &groups, &people).unwrap()[1],
            "Everyone uses Refunds: No one yet."
        );
    }

    #[test]
    fn picking_and_whether_anything_changed() {
        let mut chosen = vec![g("g1")];
        toggle(&mut chosen, s("s1"));
        toggle(&mut chosen, g("g1"));
        assert_eq!(chosen, [s("s1")]);
        assert!(same(&[g("g1"), s("s1")], &[s("s1"), g("g1")]));
        assert!(!same(&[g("g1")], &[]));
    }

    /// Sharing is signed: each change is a grant this computer never sends.
    #[test]
    fn no_share_is_sent_from_here() {
        let acts = acts("agent/1", &[s("auth0|rae")], &[g("g1")]);
        assert_eq!(acts.len(), 2);
        let routes: Vec<(&str, String)> = acts.iter().map(Act::route).collect();
        assert_eq!(
            routes[0],
            ("POST", "/v1/agents/agent%2F1/grants".to_string())
        );
        assert_eq!(
            routes[1],
            (
                "DELETE",
                "/v1/agents/agent%2F1/grants?subject=auth0%7Crae".to_string()
            )
        );
        for (method, path) in routes {
            assert!(crate::signed::signed(method, &path), "{method} {path}");
            assert!(crate::signed::Unsigned::new(method, path, vec![]).is_none());
        }
        assert!(acts_none_when_unchanged());
    }

    fn acts_none_when_unchanged() -> bool {
        acts("a", &[g("g1")], &[g("g1")]).is_empty()
    }

    #[test]
    fn every_language_has_the_sheet_s_words() {
        for lang in crate::words::SHIPPED {
            let w = WordBook::load(&WordBook::default_root(), lang).unwrap();
            sheet_words(&w, "Refunds").unwrap();
        }
    }
}
