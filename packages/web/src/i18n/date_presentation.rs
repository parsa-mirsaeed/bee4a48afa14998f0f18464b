use super::Locale;
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, Timelike, Utc};

/// Format a persisted UTC instant as product date chrome.
///
/// This intentionally stays at the presentation boundary: domain/API values
/// remain structured timestamps and user-entered content is never translated.
pub fn format_product_date(value: DateTime<Utc>, locale: Locale) -> String {
    format_date_parts(value.year(), value.month(), value.day(), locale)
}

/// Format date text received from an older API boundary without leaking its
/// English month presentation into Persian product chrome.
///
/// RFC 3339 is accepted for the structured boundary we are moving toward, and
/// the legacy `%b %d, %Y` shape remains supported until the dashboard response
/// model is migrated to a timestamp. Unknown user/data text is returned as-is.
pub fn format_product_date_text(value: &str, locale: Locale) -> String {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        return format_product_date(parsed.with_timezone(&Utc), locale);
    }
    if let Ok(parsed) = NaiveDate::parse_from_str(value, "%b %d, %Y") {
        return format_date_parts(parsed.year(), parsed.month(), parsed.day(), locale);
    }
    value.to_string()
}

/// Format date-time text from a legacy API boundary as product chrome.
///
/// Teacher submission rows currently receive `%Y-%m-%d %H:%M` strings. Keep
/// that protocol detail out of the UI while also accepting RFC 3339 so the
/// server can move to a structured timestamp without another presentation
/// rewrite.
pub fn format_product_datetime_text(value: &str, locale: Locale) -> String {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        let parsed = parsed.with_timezone(&Utc);
        return format_datetime_parts(
            parsed.year(),
            parsed.month(),
            parsed.day(),
            parsed.hour(),
            parsed.minute(),
            locale,
        );
    }
    if let Ok(parsed) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M") {
        return format_datetime_parts(
            parsed.year(),
            parsed.month(),
            parsed.day(),
            parsed.hour(),
            parsed.minute(),
            locale,
        );
    }
    value.to_string()
}

fn format_datetime_parts(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    locale: Locale,
) -> String {
    let date = format_date_parts(year, month, day, locale);
    let time = format!("{hour:02}:{minute:02}");
    match locale {
        Locale::En => format!("{date} · {time}"),
        Locale::Fa => format!("{date} · {}", to_persian_digits(&time)),
    }
}

fn format_date_parts(year: i32, month: u32, day: u32, locale: Locale) -> String {
    let (english_month, persian_month) = match month {
        1 => ("Jan", "ژانویه"),
        2 => ("Feb", "فوریه"),
        3 => ("Mar", "مارس"),
        4 => ("Apr", "آوریل"),
        5 => ("May", "مه"),
        6 => ("Jun", "ژوئن"),
        7 => ("Jul", "ژوئیه"),
        8 => ("Aug", "اوت"),
        9 => ("Sep", "سپتامبر"),
        10 => ("Oct", "اکتبر"),
        11 => ("Nov", "نوامبر"),
        12 => ("Dec", "دسامبر"),
        _ => unreachable!("chrono month is always 1..=12"),
    };
    match locale {
        Locale::En => format!("{english_month} {day:02}, {year}"),
        Locale::Fa => to_persian_digits(&format!("{day} {persian_month} {year}")),
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
        assert!(!formatted
            .chars()
            .any(|character| character.is_ascii_digit()));
    }

    #[test]
    fn legacy_dashboard_date_is_localized_at_the_ui_boundary() {
        assert_eq!(
            format_product_date_text("Sep 10, 2026", Locale::Fa),
            "۱۰ سپتامبر ۲۰۲۶"
        );
        assert_eq!(
            format_product_date_text("Sep 10, 2026", Locale::En),
            "Sep 10, 2026"
        );
    }

    #[test]
    fn rfc3339_dashboard_date_is_ready_for_structured_api_migration() {
        assert_eq!(
            format_product_date_text("2026-09-10T13:45:00Z", Locale::Fa),
            "۱۰ سپتامبر ۲۰۲۶"
        );
    }

    #[test]
    fn legacy_submission_datetime_is_localized_without_protocol_chrome() {
        assert_eq!(
            format_product_datetime_text("2026-09-10 13:45", Locale::En),
            "Sep 10, 2026 · 13:45"
        );
        assert_eq!(
            format_product_datetime_text("2026-09-10 13:45", Locale::Fa),
            "۱۰ سپتامبر ۲۰۲۶ · ۱۳:۴۵"
        );
    }

    #[test]
    fn rfc3339_submission_datetime_is_ready_for_structured_api_migration() {
        assert_eq!(
            format_product_datetime_text("2026-09-10T13:45:00Z", Locale::Fa),
            "۱۰ سپتامبر ۲۰۲۶ · ۱۳:۴۵"
        );
    }
}
