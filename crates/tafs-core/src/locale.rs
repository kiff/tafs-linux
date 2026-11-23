//! What the iPad's Foundation formatters say in each language, from the same CLDR data: dates,
//! weekdays, relative times, "and" lists, plural forms and numbers (ICU4X, compiled in). The iPad
//! reads them from the system's ICU; this app carries its own, so a computer's locale files never
//! change a line.
//!
//! A language is the words' folder ("pt-BR", "zh-Hant"). Arabic writes Latin digits, as the iPad's
//! `IOS_LOCALE` (`ar-u-ca-gregory-nu-latn`, kiff/tafs `generate.py`) has it.

use chrono::Datelike;
use icu::calendar::{Date, Iso};
use icu::datetime::fieldsets;
use icu::datetime::DateTimeFormatter;
use icu::decimal::input::Decimal;
use icu::decimal::DecimalFormatter;
use icu::list::options::ListFormatterOptions;
use icu::list::ListFormatter;
use icu::locale::Locale;
use icu::plurals::{PluralCategory, PluralRuleType, PluralRules};
use icu_experimental::relativetime::options::{Numeric, RelativeTimeFormatterOptions};
use icu_experimental::relativetime::RelativeTimeFormatter;

/// The CLDR locale a words' language is formatted in.
pub fn locale(language: &str) -> Locale {
    let tag = match language {
        "ar" => "ar-u-nu-latn",
        other => other,
    };
    tag.parse()
        .unwrap_or_else(|_| "en".parse().expect("en is a locale"))
}

/// Whether the language is written right to left.
pub fn right_to_left(language: &str) -> bool {
    matches!(base(language), "ar" | "he" | "fa" | "ur")
}

/// The language without its region or script: "pt-BR" is "pt", "zh-Hant" is "zh".
pub fn base(language: &str) -> &str {
    language.split(['-', '_']).next().unwrap_or(language)
}

fn date(day: chrono::NaiveDate) -> Option<Date<Iso>> {
    Date::try_new_iso(day.year(), day.month() as u8, day.day() as u8).ok()
}

/// `.dateTime.day().month(.abbreviated)`: "Dec 3", "3 déc.", "12月3日".
pub fn short_date(language: &str, day: chrono::NaiveDate) -> String {
    let (Some(d), Ok(f)) = (
        date(day),
        DateTimeFormatter::try_new(locale(language).into(), fieldsets::MD::medium()),
    ) else {
        return day.format("%Y-%m-%d").to_string();
    };
    f.format(&d).to_string()
}

/// `Date.shown(date: .abbreviated, time: .shortened)`: "Nov 22, 2026, 4:30 PM", "22 nov. 2026, 16:30".
pub fn date_time(language: &str, at: chrono::NaiveDateTime) -> String {
    use chrono::Timelike;
    use icu::datetime::options::TimePrecision;
    let fallback = || at.format("%Y-%m-%d %H:%M").to_string();
    let (Some(d), Ok(t)) = (
        date(at.date()),
        icu::time::Time::try_new(at.hour() as u8, at.minute() as u8, 0, 0),
    ) else {
        return fallback();
    };
    let Ok(f) = DateTimeFormatter::try_new(
        locale(language).into(),
        fieldsets::YMDT::medium().with_time_precision(TimePrecision::Minute),
    ) else {
        return fallback();
    };
    f.format(&icu::time::DateTime { date: d, time: t })
        .to_string()
}

/// `.dateTime.weekday(.abbreviated).day().month(.abbreviated)`: "Thu, Nov 19", "jeu. 19 nov.", "11月19日(木)".
pub fn weekday_day_month(language: &str, day: chrono::NaiveDate) -> String {
    let (Some(d), Ok(f)) = (
        date(day),
        DateTimeFormatter::try_new(locale(language).into(), fieldsets::MDE::medium()),
    ) else {
        return day.format("%Y-%m-%d").to_string();
    };
    f.format(&d).to_string()
}

/// `setLocalizedDateFormatFromTemplate("EEEEdMMMM")`: "Friday, November 20", "vendredi 20 novembre".
pub fn weekday_day_month_long(language: &str, day: chrono::NaiveDate) -> String {
    let (Some(d), Ok(f)) = (
        date(day),
        DateTimeFormatter::try_new(locale(language).into(), fieldsets::MDE::long()),
    ) else {
        return day.format("%Y-%m-%d").to_string();
    };
    f.format(&d).to_string()
}

/// `Date.shown(date: .long)`: "December 3, 2026", "3 décembre 2026", "2026年12月3日".
pub fn long_date(language: &str, day: chrono::NaiveDate) -> String {
    let (Some(d), Ok(f)) = (
        date(day),
        DateTimeFormatter::try_new(locale(language).into(), fieldsets::YMD::long()),
    ) else {
        return day.format("%Y-%m-%d").to_string();
    };
    f.format(&d).to_string()
}

/// `.dateTime.weekday(.wide)`: "Thursday", "jeudi", "木曜日".
pub fn weekday(language: &str, day: chrono::NaiveDate) -> String {
    let (Some(d), Ok(f)) = (
        date(day),
        DateTimeFormatter::try_new(locale(language).into(), fieldsets::E::long()),
    ) else {
        return String::new();
    };
    f.format(&d).to_string()
}

/// The units `RelativeDateTimeFormatter` picks from, smallest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Year,
}

/// `RelativeDateTimeFormatter` (full, numeric): `n` units ago when `past`, else in `n` units.
pub fn relative(language: &str, n: i64, unit: Unit, past: bool) -> String {
    let mut o = RelativeTimeFormatterOptions::default();
    o.numeric = Some(Numeric::Always);
    let p = locale(language).into();
    let f = match unit {
        Unit::Second => RelativeTimeFormatter::try_new_long_second(p, o),
        Unit::Minute => RelativeTimeFormatter::try_new_long_minute(p, o),
        Unit::Hour => RelativeTimeFormatter::try_new_long_hour(p, o),
        Unit::Day => RelativeTimeFormatter::try_new_long_day(p, o),
        Unit::Week => RelativeTimeFormatter::try_new_long_week(p, o),
        Unit::Month => RelativeTimeFormatter::try_new_long_month(p, o),
        Unit::Year => RelativeTimeFormatter::try_new_long_year(p, o),
    };
    let Ok(f) = f else {
        return n.to_string();
    };
    let signed = if past { -n.abs() } else { n.abs() };
    f.format(Decimal::from(signed)).to_string()
}

/// `ListFormatter`, CLDR's "and" list: "A, B, and C", "A, B et C", "A、B和C".
pub fn list(language: &str, names: &[String]) -> String {
    match ListFormatter::try_new_and(locale(language).into(), ListFormatterOptions::default()) {
        Ok(f) => f.format_to_string(names.iter()),
        Err(_) => names.join(", "),
    }
}

/// The plural forms a language's whole numbers take beside "other", as kiff/tafs's `generate.py`
/// lists them (`PLURALS`): the forms its files carry. A variant takes its base's.
fn forms(language: &str) -> &'static [&'static str] {
    match base(language) {
        "pl" | "ru" => &["one", "few", "many"],
        "lt" | "cs" | "sk" | "ro" | "hr" | "sr" => &["one", "few"],
        "lv" => &["zero", "one"],
        "sl" => &["one", "two", "few"],
        "he" => &["one", "two"],
        "ar" => &["zero", "one", "two", "few", "many"],
        "ja" | "ko" | "id" | "vi" | "th" | "zh" | "km" | "lo" | "ms" => &[],
        _ => &["one"],
    }
}

/// The CLDR plural form a whole number takes: "zero", "one", "two", "few", "many" or "other",
/// kept to the forms the language's files carry (`forms`), as the iPad's catalog is built.
pub fn plural_form(language: &str, n: i64) -> &'static str {
    let form = match PluralRules::try_new(locale(language).into(), PluralRuleType::Cardinal.into())
    {
        Ok(rules) => match rules.category_for(n.unsigned_abs()) {
            PluralCategory::Zero => "zero",
            PluralCategory::One => "one",
            PluralCategory::Two => "two",
            PluralCategory::Few => "few",
            PluralCategory::Many => "many",
            PluralCategory::Other => "other",
        },
        Err(_) if n.unsigned_abs() == 1 => "one",
        Err(_) => "other",
    };
    if forms(language).contains(&form) {
        form
    } else {
        "other"
    }
}

/// The CLDR ordinal plural class of a whole number ("one", "two", "few", "many", "other").
pub fn ordinal_class(language: &str, n: i64) -> &'static str {
    let Ok(rules) = PluralRules::try_new(locale(language).into(), PluralRuleType::Ordinal.into())
    else {
        return "other";
    };
    match rules.category_for(n.unsigned_abs()) {
        PluralCategory::Zero => "zero",
        PluralCategory::One => "one",
        PluralCategory::Two => "two",
        PluralCategory::Few => "few",
        PluralCategory::Many => "many",
        PluralCategory::Other => "other",
    }
}

/// A number given in minor units, at `decimals` places, as the language writes it
/// (`numberStyle = .decimal`): its grouping (and CLDR's minimum grouping digits: Spanish and
/// Polish write 5000 but 25.000), its separators, its minus.
pub fn number(minor: i64, decimals: u32, language: &str) -> String {
    let mut d = Decimal::from(minor);
    d.multiply_pow10(-(decimals as i16));
    match DecimalFormatter::try_new(locale(language).into(), Default::default()) {
        Ok(f) => f.format(&d).to_string(),
        Err(_) => d.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_date_and_time_is_abbreviated_with_the_language_s_clock() {
        let at = chrono::NaiveDate::from_ymd_opt(2026, 11, 19)
            .unwrap()
            .and_hms_opt(16, 30, 0)
            .unwrap();
        let en = date_time("en", at);
        assert!(en.contains("Nov 19, 2026") && en.contains("4:30"), "{en}");
        let fr = date_time("fr", at);
        assert!(fr.contains("19 nov. 2026") && fr.contains("16:30"), "{fr}");
        assert!(date_time("ar", at).contains("16:30") || date_time("ar", at).contains("4:30"));
        for lang in crate::words::SHIPPED {
            assert!(!date_time(lang, at).is_empty(), "{lang}");
        }
    }

    fn day(y: i32, m: u32, d: u32) -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn dates_as_the_ipad_writes_them() {
        let d = day(2026, 12, 3);
        assert_eq!(short_date("en", d), "Dec 3");
        assert_eq!(short_date("ja", d), "12月3日");
        assert_eq!(long_date("zh-Hant", d), "2026年12月3日");
        assert_eq!(long_date("pt-BR", d), "3 de dezembro de 2026");
        assert_eq!(weekday("ar", d), "الخميس");
        // Latin digits in Arabic, as the iPad's `nu-latn`.
        assert_eq!(short_date("ar", d), "3 ديسمبر");
    }

    #[test]
    fn plural_forms_of_the_new_languages() {
        assert_eq!(plural_form("ar", 0), "zero");
        assert_eq!(plural_form("ar", 2), "two");
        assert_eq!(plural_form("ar", 3), "few");
        assert_eq!(plural_form("ar", 11), "many");
        assert_eq!(plural_form("ar", 100), "other");
        assert_eq!(plural_form("lv", 0), "zero");
        assert_eq!(plural_form("lv", 21), "one");
        assert_eq!(plural_form("sl", 2), "two");
        assert_eq!(plural_form("sl", 3), "few");
        assert_eq!(plural_form("cs", 3), "few");
        assert_eq!(plural_form("cs", 5), "other");
        assert_eq!(plural_form("ja", 1), "other");
        assert_eq!(plural_form("zh-Hant", 1), "other");
        assert_eq!(plural_form("vi", 1), "other");
        assert_eq!(plural_form("pt-BR", 0), "one");
        // Turkmen and Urdu: one is "one"; Hindi: 0 and 1 are.
        assert_eq!(plural_form("tk", 0), "other");
        assert_eq!(plural_form("tk", 1), "one");
        assert_eq!(plural_form("ur", 1), "one");
        assert_eq!(plural_form("ur", 2), "other");
        assert_eq!(plural_form("hi", 0), "one");
        assert_eq!(plural_form("hi", 2), "other");
    }

    #[test]
    fn lists_and_numbers() {
        let abc: Vec<String> = ["A", "B", "C"].iter().map(|s| s.to_string()).collect();
        assert_eq!(list("ja", &abc), "A、B、C");
        assert_eq!(list("ar", &abc), "A وB وC");
        assert_eq!(number(5000, 0, "es"), "5000");
        assert_eq!(number(25000, 0, "es"), "25.000");
        assert_eq!(number(123456789, 2, "ar"), "1,234,567.89");
        assert_eq!(relative("ja", 3, Unit::Hour, true), "3 時間前");
        // Turkmen, Hindi and Urdu come with ICU4X's own data.
        assert_eq!(list("tk", &abc), "A, B we C");
        assert_eq!(list("ur", &abc), "A، B، اور C");
        assert_eq!(number(12345678, 2, "hi"), "1,23,456.78");
        assert_eq!(number(12345678, 2, "tk"), "123\u{a0}456,78");
        assert_eq!(relative("tk", 3, Unit::Hour, true), "3 sagat öň");
        assert_eq!(relative("ur", 2, Unit::Day, true), "2 دنوں پہلے");
    }

    #[test]
    fn direction() {
        assert!(right_to_left("ar"));
        assert!(right_to_left("ur"));
        assert!(!right_to_left("zh-Hant"));
        assert_eq!(base("pt-BR"), "pt");
    }
}
