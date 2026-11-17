//! "A new version of tafs is out": tell only. An AppImage never updates itself, so at start and
//! once a day the app reads tafs for Linux's latest **published** release from GitHub's public
//! API and, when it's newer than this app, shows one line at the sidebar's foot that opens the
//! release's page in the browser. Nothing is downloaded or replaced, and nothing runs in the
//! background between checks.
//!
//! The request sends nothing about the user or the account: no token, no cookie, only the app's
//! own User-Agent with its version (`USER_AGENT`).

use serde_json::Value;

/// Where the latest published release is read: GitHub's public API, with no token. It answers
/// only the latest release that is neither a draft nor a prerelease.
pub const LATEST: &str = "https://api.github.com/repos/kiff/tafs-linux/releases/latest";

/// The releases page: what the line opens when the answer's own page isn't one of ours.
pub const RELEASES: &str = "https://github.com/kiff/tafs-linux/releases";

/// This app's version, from its Cargo.toml.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The only thing the request says about who's asking: the app and its version.
pub const USER_AGENT: &str = concat!("tafs-linux/", env!("CARGO_PKG_VERSION"));

/// How often the app checks again while it stays open: once a day.
pub const EVERY_SECONDS: u32 = 24 * 60 * 60;

/// A newer published release: its version, as `1.0.2`, and the page the line opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewVersion {
    pub version: String,
    pub url: String,
}

/// A version as three numbers, from `v1.0.2` or `1.0.2`; anything else isn't one.
pub fn parse(v: &str) -> Option<(u64, u64, u64)> {
    let v = v.trim();
    let v = v.strip_prefix('v').unwrap_or(v);
    let mut parts = v.split('.');
    let mut next = || -> Option<u64> {
        let p = parts.next()?;
        // Digits only: no sign, no space, no "-rc1".
        if p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        p.parse().ok()
    };
    let version = (next()?, next()?, next()?);
    parts.next().is_none().then_some(version)
}

/// What GitHub's answer says, against this app's version: a newer published release, or none.
/// A draft, a prerelease, a version that isn't three numbers, the same version or an older one
/// is none. The page opened is the answer's own only when it's one of tafs for Linux's release
/// pages; otherwise the releases page.
pub fn newer(current: &str, answer: &Value) -> Option<NewVersion> {
    if answer.get("draft").and_then(Value::as_bool) != Some(false)
        || answer.get("prerelease").and_then(Value::as_bool) != Some(false)
    {
        return None;
    }
    let tag = answer.get("tag_name").and_then(Value::as_str)?;
    let theirs = parse(tag)?;
    let ours = parse(current)?;
    if theirs <= ours {
        return None;
    }
    let page = answer
        .get("html_url")
        .and_then(Value::as_str)
        .filter(|u| ours_page(u))
        .unwrap_or(RELEASES);
    Some(NewVersion {
        version: format!("{}.{}.{}", theirs.0, theirs.1, theirs.2),
        url: page.to_string(),
    })
}

/// One check: `read` asks `LATEST` for its answer. A read that fails (offline, refused, a rate
/// limit, not JSON) is no line and no error: the next check tries again.
pub fn check<E>(read: impl FnOnce(&str) -> Result<Value, E>) -> Option<NewVersion> {
    read(LATEST).ok().and_then(|answer| newer(VERSION, &answer))
}

/// One of tafs for Linux's release pages, and nothing else: the browser opens only those.
fn ours_page(url: &str) -> bool {
    url.strip_prefix(RELEASES)
        .and_then(|rest| rest.strip_prefix("/tag/"))
        .is_some_and(|tag| {
            !tag.is_empty()
                && tag
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn release(tag: &str) -> Value {
        json!({
            "tag_name": tag,
            "draft": false,
            "prerelease": false,
            "html_url": format!("https://github.com/kiff/tafs-linux/releases/tag/{tag}"),
        })
    }

    #[test]
    fn a_newer_release_is_told() {
        assert_eq!(
            newer("1.0.1", &release("v1.0.2")),
            Some(NewVersion {
                version: "1.0.2".into(),
                url: "https://github.com/kiff/tafs-linux/releases/tag/v1.0.2".into(),
            })
        );
        // By number, never by text: 1.10.0 is after 1.9.3, and 2.0.0 after 1.99.99.
        assert!(newer("1.9.3", &release("v1.10.0")).is_some());
        assert!(newer("1.99.99", &release("v2.0.0")).is_some());
    }

    #[test]
    fn the_same_release_is_not() {
        assert_eq!(newer("1.0.1", &release("v1.0.1")), None);
        assert_eq!(newer("v1.0.1", &release("1.0.1")), None);
    }

    #[test]
    fn an_older_release_is_not() {
        assert_eq!(newer("1.0.1", &release("v1.0.0")), None);
        assert_eq!(newer("1.10.0", &release("v1.9.9")), None);
    }

    #[test]
    fn a_draft_or_a_prerelease_never_counts() {
        let mut draft = release("v9.0.0");
        draft["draft"] = json!(true);
        assert_eq!(newer("1.0.1", &draft), None);
        let mut pre = release("v9.0.0");
        pre["prerelease"] = json!(true);
        assert_eq!(newer("1.0.1", &pre), None);
        // An answer that doesn't say it's published isn't taken as published.
        let mut unsaid = release("v9.0.0");
        unsaid.as_object_mut().unwrap().remove("draft");
        assert_eq!(newer("1.0.1", &unsaid), None);
    }

    #[test]
    fn an_answer_that_isnt_a_release_is_nothing() {
        // What GitHub answers when there's no published release yet, or a rate limit.
        assert_eq!(newer("1.0.1", &json!({"message": "Not Found"})), None);
        assert_eq!(newer("1.0.1", &json!(null)), None);
        assert_eq!(newer("1.0.1", &release("v1.0.2-rc1")), None);
        assert_eq!(newer("1.0.1", &release("nightly")), None);
        assert_eq!(newer("1.0.1", &release("v1.0")), None);
        assert_eq!(newer("1.0.1", &release("v1.0.2.3")), None);
    }

    #[test]
    fn the_line_opens_only_our_release_pages() {
        let mut elsewhere = release("v1.0.2");
        elsewhere["html_url"] = json!("https://example.com/kiff/tafs-linux/releases/tag/v1.0.2");
        assert_eq!(newer("1.0.1", &elsewhere).unwrap().url, RELEASES);
        let mut sneaky = release("v1.0.2");
        sneaky["html_url"] =
            json!("https://github.com/kiff/tafs-linux/releases/tag/v1.0.2/../../../../evil");
        assert_eq!(newer("1.0.1", &sneaky).unwrap().url, RELEASES);
        let mut other_repo = release("v1.0.2");
        other_repo["html_url"] = json!("https://github.com/kiff/tafs-linuxx/releases/tag/v1.0.2");
        assert_eq!(newer("1.0.1", &other_repo).unwrap().url, RELEASES);
        let mut none = release("v1.0.2");
        none.as_object_mut().unwrap().remove("html_url");
        assert_eq!(newer("1.0.1", &none).unwrap().url, RELEASES);
    }

    #[test]
    fn offline_is_no_line_and_no_error() {
        let asked = std::cell::RefCell::new(String::new());
        let line = check(|url| {
            *asked.borrow_mut() = url.to_string();
            Err::<Value, _>("no network")
        });
        assert_eq!(line, None);
        assert_eq!(*asked.borrow(), LATEST);
        // And when it can read, it tells: a release far ahead of any version this app has.
        assert_eq!(
            check(|_| Ok::<_, ()>(release("v999.0.0"))).map(|n| n.version),
            Some("999.0.0".into())
        );
    }

    #[test]
    fn the_request_says_only_the_app_and_its_version() {
        assert_eq!(USER_AGENT, format!("tafs-linux/{VERSION}"));
        assert!(parse(VERSION).is_some(), "Cargo's version is three numbers");
    }
}
