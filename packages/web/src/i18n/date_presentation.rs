use super::Locale;
use chrono::{DateTime, Datelike, Utc};

/// Format a persisted UTC instant as product date chrome.
///
/// This intentionally stays at the presentation boundary: domain/API values
/// remain structured timestamps and user-entered content is never translated.
pub fn format_product_date(value: DateTime<Utc>, locale: Locale) -> String {
    match locale {
        Locale::En => value.format("%b %d, %Y").to_string(),
        Locale::Fa => {
            let month = match value.month() {
                1 => "ژانویه",
                2 => "فوریه",
                3 => "مارس",
                4 => "آوریل",
                5 => "مه",
                6 => "ژوئن",
                7 => "ژوئیه",
                8 => "اوت",
                9 => "سپتامبر",
                10 => "اکتبر",
                11 => "نوامبر",
                12 => "دسامبر",
                _ => unreachable!("chrono month is always 1..=12"),
            };
            to_persian_digits(&format!("{} {} {}", value.day(), month, value.year()))
        }
    }
}

fn to_persian_digits(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '0' => '۰',
            '1' => '۱',
            '2' => '۲',
            '3' => '۳',
            '4' => '۴',
            '5' => '۵',
            '6' => '۶',
            '7' => '۷',
            '8' => '۸',
            '9' => '۹',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn english_date_uses_readable_month_name() {
        let value = Utc.with_ymd_and_hms(2026, 9, 10, 13, 45, 0).unwrap();
        assert_eq!(format_product_date(value, Locale::En), "Sep 10, 2026");
    }

    #[test]
    fn persian_date_has_no_english_month_or_ascii_digits() {
        let value = Utc.with_ymd_and_hms(2026, 9, 10, 13, 45, 0).unwrap();
        let formatted = format_product_date(value, Locale::Fa);
        assert_eq!(formatted, "۱۰ سپتامبر ۲۰۲۶");
        assert!(!formatted.contains("Sep"));
        assert!(!formatted.chars().any(|character| character.is_ascii_digit()));
    }
}
