//! How this computer reads back what Account's settings keep in the person's config folder (the
//! iPad's `UserDefaults` keys): the theme (`tafs.theme`), the language (`tafs.language`) and a
//! switch (Alerts on this computer, their sound). Each is read strictly: anything else, from an
//! older app or a hand edit, reads as the setting's default, never as something else.

/// `ThemePreference`: `light` or `dark` as kept, else `system`.
pub fn theme(kept: Option<&str>) -> &'static str {
    match kept.map(str::trim) {
        Some("light") => "light",
        Some("dark") => "dark",
        _ => "system",
    }
}

/// The language kept, if it's one the app ships; else none, and this computer's language counts.
pub fn language(kept: Option<&str>) -> Option<&'static str> {
    let kept = kept?.trim();
    crate::words::SHIPPED.iter().find(|l| **l == kept).copied()
}

/// A switch kept as `on` or `off`; anything else, or nothing kept, is its default.
pub fn switch(kept: Option<&str>, default: bool) -> bool {
    match kept.map(str::trim) {
        Some("on") => true,
        Some("off") => false,
        _ => default,
    }
}

/// How a switch is kept.
pub fn switch_text(on: bool) -> &'static str {
    if on {
        "on"
    } else {
        "off"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_theme_reads_light_dark_or_the_system() {
        assert_eq!(theme(Some("light")), "light");
        assert_eq!(theme(Some("dark\n")), "dark");
        for other in [None, Some(""), Some("system"), Some("Dark"), Some("sepia")] {
            assert_eq!(theme(other), "system", "{other:?}");
        }
    }

    #[test]
    fn a_language_is_kept_only_if_the_app_ships_it() {
        assert_eq!(language(Some("fr")), Some("fr"));
        assert_eq!(language(Some(" pt-BR\n")), Some("pt-BR"));
        // Not shipped, another spelling, or nothing: this computer's language.
        for other in [None, Some(""), Some("xx"), Some("FR"), Some("pt_BR")] {
            assert_eq!(language(other), None, "{other:?}");
        }
    }

    #[test]
    fn a_switch_reads_on_off_or_its_default() {
        assert!(switch(Some("on"), false));
        assert!(!switch(Some("off\n"), true));
        for default in [true, false] {
            for other in [None, Some(""), Some("yes"), Some("1"), Some("ON")] {
                assert_eq!(switch(other, default), default, "{other:?}");
            }
        }
        // What's kept reads back as itself.
        for on in [true, false] {
            assert_eq!(switch(Some(switch_text(on)), !on), on);
        }
    }
}
