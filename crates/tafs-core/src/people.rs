//! Account › People & groups (`PeopleGroupsView`): the account's groups, each with its people
//! and the tafs it uses, as KIFF lists them (`KIFFClient.Group`, `Person`; `GET /v1/groups`,
//! `GET /v1/tenant/members`). Read as strictly as the iPad's synthesized decoders.

use serde_json::Value;

use crate::account::{signed_in_as, Account};
use crate::signed::Act;
use crate::words::{list, WordBook, WordsError};

/// One of a group's people (`GroupMember`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub subject: String,
    /// manager or member.
    pub role: String,
}

impl Member {
    pub fn is_manager(&self) -> bool {
        self.role == "manager"
    }
}

/// A group (`KIFFClient.Group`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub id: String,
    pub name: String,
    /// KIFF's default group: everyone on the account.
    pub everyone: bool,
    pub members: Vec<Member>,
    /// The KIFF agents the group is shared.
    pub agents: Vec<String>,
}

/// One of the account's people (`KIFFClient.Person`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    pub subject: String,
    pub role: String,
    pub status: String,
    pub email: Option<String>,
    pub you: bool,
}

impl Person {
    /// Their email, else KIFF's id for them.
    pub fn label(&self) -> &str {
        self.email
            .as_deref()
            .filter(|e| !e.is_empty())
            .unwrap_or(&self.subject)
    }
}

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key)?.as_str().map(str::to_string)
}

/// A string, absent or null; anything else fails the read.
fn optional(v: &Value, key: &str) -> Result<Option<String>, ()> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(()),
    }
}

fn strings(v: &Value) -> Option<Vec<String>> {
    v.as_array()?
        .iter()
        .map(|s| s.as_str().map(str::to_string))
        .collect()
}

impl Group {
    fn read(v: &Value) -> Option<Group> {
        Some(Group {
            id: text(v, "id")?,
            name: text(v, "name")?,
            everyone: match v.get("everyone") {
                None | Some(Value::Null) => false,
                Some(Value::Bool(b)) => *b,
                Some(_) => return None,
            },
            // Swift's synthesized decoder requires a non-optional with a default.
            members: v
                .get("members")?
                .as_array()?
                .iter()
                .map(|m| {
                    Some(Member {
                        subject: text(m, "subject")?,
                        role: text(m, "role")?,
                    })
                })
                .collect::<Option<_>>()?,
            agents: strings(v.get("agents")?)?,
        })
    }

    /// `GET /v1/groups`: every group, or none when one can't be read.
    pub fn read_all(body: &[u8]) -> Option<Vec<Group>> {
        let v: Value = serde_json::from_slice(body).ok()?;
        v.get("groups")?
            .as_array()?
            .iter()
            .map(Group::read)
            .collect()
    }

    /// `shownName`: KIFF's default group in the person's language; any other as named.
    pub fn shown_name(&self, w: &WordBook) -> Result<String, WordsError> {
        if self.everyone {
            w.line("people.everyone_group")
        } else {
            Ok(self.name.clone())
        }
    }
}

impl Person {
    fn read(v: &Value) -> Option<Person> {
        Some(Person {
            subject: text(v, "subject")?,
            role: text(v, "role")?,
            status: text(v, "status")?,
            email: optional(v, "email").ok()?,
            you: match v.get("you") {
                None | Some(Value::Null) => false,
                Some(Value::Bool(b)) => *b,
                Some(_) => return None,
            },
        })
    }

    /// `GET /v1/tenant/members`: everyone, or none when one can't be read.
    pub fn read_all(body: &[u8]) -> Option<Vec<Person>> {
        let v: Value = serde_json::from_slice(body).ok()?;
        v.get("members")?
            .as_array()?
            .iter()
            .map(Person::read)
            .collect()
    }
}

pub const GROUPS_PATH: &str = "/v1/groups";
pub const PEOPLE_PATH: &str = "/v1/tenant/members";

/// `refreshPeople`'s order: KIFF's default group first, then by name; the active people by label.
pub fn arrange(mut groups: Vec<Group>, people: Vec<Person>) -> (Vec<Group>, Vec<Person>) {
    groups.sort_by_key(|g| (!g.everyone, g.name.to_lowercase()));
    let mut people: Vec<Person> = people
        .into_iter()
        .filter(|p| p.status == "active")
        .collect();
    people.sort_by_key(|p| p.label().to_lowercase());
    (groups, people)
}

/// `AccountLines.people`: "Acme · 3 people"; the count alone for an account named after its
/// email (a Solo account).
pub fn account_people(w: &WordBook, account: &Account, count: usize) -> Result<String, WordsError> {
    let people = w.count("people.count", count as i64, &[])?;
    let solo =
        signed_in_as(account).is_none() && account.email.as_deref().is_some_and(|e| !e.is_empty());
    Ok(if solo {
        people
    } else {
        format!("{} · {people}", account.name())
    })
}

/// `PeopleGroupsView.uses`: "Uses Refunds and Invoices", the names sorted; or none yet.
pub fn uses(w: &WordBook, group: &Group, names: &[String]) -> Result<String, WordsError> {
    if names.is_empty() {
        return w.line(if group.everyone {
            "people.everyone_uses_none"
        } else {
            "people.uses_none"
        });
    }
    let mut sorted = names.to_vec();
    sorted.sort();
    w.get("people.uses", &[("tafs", &list(w.language(), &sorted))])
}

/// `managersFirst`: managers, then members, each by their label.
pub fn managers_first<'a>(members: &'a [Member], people: &[Person]) -> Vec<&'a Member> {
    let label = |m: &Member| -> String {
        people
            .iter()
            .find(|p| p.subject == m.subject)
            .map(|p| p.label().to_lowercase())
            .unwrap_or_else(|| m.subject.to_lowercase())
    };
    let mut v: Vec<&Member> = members.iter().collect();
    v.sort_by_key(|m| (!m.is_manager(), label(m)));
    v
}

/// `managesGroup`: whoever manages people, or the group's own manager.
pub fn manages_group(group: &Group, can_manage_people: bool, me: Option<&str>) -> bool {
    can_manage_people
        || group
            .members
            .iter()
            .any(|m| Some(m.subject.as_str()) == me && m.is_manager())
}

/// `mayRemove`: no one is removed from KIFF's default group; a group's manager removes its
/// members, and whoever manages people removes anyone.
pub fn may_remove(
    member: &Member,
    group: &Group,
    can_manage_people: bool,
    manages_group: bool,
) -> bool {
    !group.everyone && (can_manage_people || (manages_group && !member.is_manager()))
}

/// `GroupNameSheet`'s words: a new group's name, or a group's new one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameSheet {
    pub title: String,
    /// The field's placeholder.
    pub hint: String,
    /// What naming it does, under the field.
    pub note: String,
    pub cancel: String,
    /// Create for a new group, Save for a renamed one.
    pub confirm: String,
}

pub fn name_sheet(w: &WordBook, group: Option<&Group>) -> Result<NameSheet, WordsError> {
    Ok(NameSheet {
        title: match group {
            None => w.line("people.new_group")?,
            Some(g) => w.get("people.rename_group_title", &[("group", &g.shown_name(w)?)])?,
        },
        hint: w.line("people.group_name_hint")?,
        note: w.line(if group.is_none() {
            "people.new_group_hint"
        } else {
            "people.rename_hint"
        })?,
        cancel: w.line("common.cancel")?,
        confirm: w.line(if group.is_none() {
            "people.create"
        } else {
            "common.save"
        })?,
    })
}

/// `GroupNameSheet.save`: the act once a name is typed, trimmed; none while it's empty.
pub fn name_act(group: Option<&Group>, name: &str) -> Option<Act> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    Some(match group {
        None => Act::CreateGroup { name: name.into() },
        Some(g) => Act::RenameGroup {
            group: g.id.clone(),
            name: name.into(),
        },
    })
}

/// The question before a group is deleted: its title, what it does, and its button.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteQuestion {
    pub title: String,
    pub body: String,
    pub button: String,
    pub cancel: String,
}

pub fn delete_question(w: &WordBook, group: &Group) -> Result<DeleteQuestion, WordsError> {
    let name = group.shown_name(w)?;
    Ok(DeleteQuestion {
        title: w.get("people.delete_group_title", &[("group", &name)])?,
        body: w.line("people.delete_group_body")?,
        button: w.get("people.delete_group_button", &[("group", &name)])?,
        cancel: w.line("common.cancel")?,
    })
}

/// A group's menu (`groupHeader`), for whoever manages people, never on KIFF's default group:
/// Rename, and Delete in red.
pub fn group_menu(
    w: &WordBook,
    group: &Group,
    can_manage_people: bool,
) -> Result<Option<(String, String)>, WordsError> {
    if !can_manage_people || group.everyone {
        return Ok(None);
    }
    Ok(Some((w.line("people.rename")?, w.line("people.delete")?)))
}

/// A person's menu in a group (`memberRow`'s context menu): making them the manager or a member,
/// for whoever manages people; removing them, where `may_remove` lets them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemberMenu {
    pub role: Option<(String, Act)>,
    pub remove: Option<(String, Act)>,
}

pub fn member_menu(
    w: &WordBook,
    member: &Member,
    group: &Group,
    can_manage_people: bool,
    manages_group: bool,
) -> Result<MemberMenu, WordsError> {
    let mut menu = MemberMenu::default();
    if can_manage_people {
        menu.role = Some((
            w.line(if member.is_manager() {
                "people.make_member"
            } else {
                "people.make_manager"
            })?,
            Act::SetGroupMember {
                group: group.id.clone(),
                subject: member.subject.clone(),
                role: if member.is_manager() {
                    "member"
                } else {
                    "manager"
                }
                .into(),
            },
        ));
    }
    if may_remove(member, group, can_manage_people, manages_group) {
        menu.remove = Some((
            w.get("people.remove_from", &[("group", &group.shown_name(w)?)])?,
            Act::RemoveGroupMember {
                group: group.id.clone(),
                subject: member.subject.clone(),
            },
        ));
    }
    Ok(menu)
}

/// `AddToGroupSheet`'s words: its title; As the group's manager and what a manager does, for
/// whoever manages people; the list's header; and that everyone is in it already.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddSheet {
    pub title: String,
    pub as_manager: Option<(String, String)>,
    pub header: String,
    pub everyone_in: String,
    pub cancel: String,
}

pub fn add_sheet(
    w: &WordBook,
    group: &Group,
    can_manage_people: bool,
) -> Result<AddSheet, WordsError> {
    Ok(AddSheet {
        title: w.get("people.add_to", &[("group", &group.shown_name(w)?)])?,
        as_manager: if can_manage_people {
            Some((
                w.line("people.as_manager")?,
                w.line("people.manager_explains")?,
            ))
        } else {
            None
        },
        header: w.line("people.account_people_title")?,
        everyone_in: w.line("people.everyone_in")?,
        cancel: w.line("common.cancel")?,
    })
}

/// `AddToGroupSheet.outside`: the account's people not in the group yet.
pub fn outside<'a>(group: &Group, people: &'a [Person]) -> Vec<&'a Person> {
    people
        .iter()
        .filter(|p| !group.members.iter().any(|m| m.subject == p.subject))
        .collect()
}

/// Adding someone: as the manager only when asked and the person adding manages people.
pub fn add_act(group: &Group, person: &Person, as_manager: bool, can_manage_people: bool) -> Act {
    Act::SetGroupMember {
        group: group.id.clone(),
        subject: person.subject.clone(),
        role: if as_manager && can_manage_people {
            "manager"
        } else {
            "member"
        }
        .into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_and_people_read_as_strictly_as_the_ipad() {
        let g = Group::read_all(
            br#"{"groups":[{"id":"g1","name":"Finance","members":[{"subject":"s1","role":"manager"}],"agents":["a1"]},{"id":"g0","name":"Everyone","everyone":true,"members":[],"agents":[]}]}"#,
        )
        .unwrap();
        assert_eq!(g.len(), 2);
        // `members` and `agents` are required, as Swift's synthesized decoder has them.
        assert!(Group::read_all(br#"{"groups":[{"id":"g","name":"n","agents":[]}]}"#).is_none());
        assert!(Group::read_all(
            br#"{"groups":[{"id":"g","name":"n","members":[],"agents":[1]}]}"#
        )
        .is_none());
        let p = Person::read_all(
            br#"{"members":[{"subject":"s1","role":"admin","status":"active","email":"rae@acme.test","you":true},{"subject":"s2","role":"member","status":"invited"}]}"#,
        )
        .unwrap();
        assert_eq!(p[0].label(), "rae@acme.test");
        assert_eq!(p[1].label(), "s2");
        assert!(Person::read_all(br#"{"members":[{"subject":"s","role":"r"}]}"#).is_none());
        let (groups, people) = arrange(g, p);
        assert!(groups[0].everyone);
        assert_eq!(people.len(), 1);
    }

    #[test]
    fn who_may_do_what() {
        let group = Group {
            id: "g".into(),
            name: "n".into(),
            everyone: false,
            members: vec![
                Member {
                    subject: "b".into(),
                    role: "member".into(),
                },
                Member {
                    subject: "a".into(),
                    role: "manager".into(),
                },
            ],
            agents: vec![],
        };
        let order: Vec<&str> = managers_first(&group.members, &[])
            .iter()
            .map(|m| m.subject.as_str())
            .collect();
        assert_eq!(order, ["a", "b"]);
        assert!(manages_group(&group, false, Some("a")));
        assert!(!manages_group(&group, false, Some("b")));
        assert!(may_remove(&group.members[0], &group, false, true));
        assert!(!may_remove(&group.members[1], &group, false, true));
        let everyone = Group {
            everyone: true,
            ..group.clone()
        };
        assert!(!may_remove(&everyone.members[0], &everyone, true, true));
    }

    fn finance() -> Group {
        Group {
            id: "g1".into(),
            name: "Finance".into(),
            everyone: false,
            members: vec![
                Member {
                    subject: "s1".into(),
                    role: "manager".into(),
                },
                Member {
                    subject: "s2".into(),
                    role: "member".into(),
                },
            ],
            agents: vec![],
        }
    }

    fn person(subject: &str) -> Person {
        Person {
            subject: subject.into(),
            role: "member".into(),
            status: "active".into(),
            email: Some(format!("{subject}@acme.test")),
            you: false,
        }
    }

    #[test]
    fn naming_a_group_up_to_its_signature() {
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        let new = name_sheet(&w, None).unwrap();
        assert_eq!(
            (new.title.as_str(), new.confirm.as_str(), new.hint.as_str()),
            ("New group", "Create", "Support, Finance…")
        );
        let g = finance();
        let rename = name_sheet(&w, Some(&g)).unwrap();
        assert_eq!(rename.title, "Rename Finance");
        assert_eq!(rename.confirm, "Save");
        assert_eq!(
            rename.note,
            "Only the name changes: its people and tafs stay."
        );
        assert_eq!(name_act(None, "  \n "), None);
        assert_eq!(
            name_act(None, "  Support "),
            Some(Act::CreateGroup {
                name: "Support".into()
            })
        );
        assert_eq!(
            name_act(Some(&g), "Money"),
            Some(Act::RenameGroup {
                group: "g1".into(),
                name: "Money".into()
            })
        );
        let d = delete_question(&w, &g).unwrap();
        assert_eq!(
            (d.title.as_str(), d.button.as_str()),
            ("Delete Finance?", "Delete Finance")
        );
        assert!(group_menu(&w, &g, true).unwrap().is_some());
        assert!(group_menu(&w, &g, false).unwrap().is_none());
        let everyone = Group {
            everyone: true,
            ..g.clone()
        };
        assert!(group_menu(&w, &everyone, true).unwrap().is_none());
    }

    #[test]
    fn a_persons_menu_and_adding_someone() {
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        let g = finance();
        let manager = &g.members[0];
        let member = &g.members[1];
        let m = member_menu(&w, manager, &g, true, true).unwrap();
        assert_eq!(m.role.as_ref().unwrap().0, "Make a member");
        assert_eq!(m.remove.as_ref().unwrap().0, "Remove from Finance");
        // A group's own manager removes its members, never a manager, and names no manager.
        let m = member_menu(&w, manager, &g, false, true).unwrap();
        assert_eq!(m, MemberMenu::default());
        let m = member_menu(&w, member, &g, false, true).unwrap();
        assert!(m.role.is_none() && m.remove.is_some());
        let people = vec![person("s1"), person("s3")];
        let out = outside(&g, &people);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].subject, "s3");
        let sheet = add_sheet(&w, &g, false).unwrap();
        assert_eq!(sheet.title, "Add to Finance");
        assert!(sheet.as_manager.is_none());
        assert!(add_sheet(&w, &g, true).unwrap().as_manager.is_some());
        let role = |a: Act| match a {
            Act::SetGroupMember { role, .. } => role,
            _ => unreachable!(),
        };
        assert_eq!(role(add_act(&g, out[0], true, true)), "manager");
        assert_eq!(role(add_act(&g, out[0], true, false)), "member");
        assert_eq!(role(add_act(&g, out[0], false, true)), "member");
    }

    #[test]
    fn every_change_to_people_is_signed() {
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        let g = finance();
        let m = member_menu(&w, &g.members[1], &g, true, true).unwrap();
        let acts = [
            name_act(None, "Support").unwrap(),
            name_act(Some(&g), "Money").unwrap(),
            Act::DeleteGroup {
                group: g.id.clone(),
            },
            m.role.unwrap().1,
            m.remove.unwrap().1,
            add_act(&g, &person("s3"), false, true),
        ];
        for act in acts {
            let (method, path) = act.route();
            assert!(crate::signed::Unsigned::new(method, path, vec![]).is_none());
        }
    }
}
