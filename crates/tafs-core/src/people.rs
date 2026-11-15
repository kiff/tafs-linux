//! Account › People & groups (`PeopleGroupsView`): the account's groups, each with its people
//! and the tafs it uses, as KIFF lists them (`KIFFClient.Group`, `Person`; `GET /v1/groups`,
//! `GET /v1/tenant/members`). Read as strictly as the iPad's synthesized decoders.

use serde_json::Value;

use crate::account::{signed_in_as, Account};
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
}
