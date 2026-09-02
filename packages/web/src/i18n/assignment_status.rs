use super::Locale;

/// Localize canonical assignment lifecycle and derived progress states at the
/// presentation boundary while leaving stored/domain values unchanged.
pub fn assignment_status_label(value: &str, locale: Locale) -> String {
    let normalized = value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>();

    let (en, fa) = match normalized.as_str() {
        "draft" => ("Draft", "پیش‌نویس"),
        "published" => ("Published", "منتشرشده"),
        "inprogress" => ("In progress", "در حال انجام"),
        "pending" => ("Pending", "در انتظار"),
        "overdue" => ("Overdue", "عقب‌افتاده"),
        "submitted" => ("Submitted", "ارسال‌شده"),
        "graded" => ("Graded", "نمره‌گذاری‌شده"),
        "archived" => ("Archived", "بایگانی‌شده"),
        "active" => ("Active", "فعال"),
        "grading" => ("Grading", "در حال نمره‌دهی"),
        "complete" | "completed" => ("Complete", "تکمیل‌شده"),
        _ => return value.to_string(),
    };

    match locale {
        Locale::En => en.to_string(),
        Locale::Fa => fa.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::ui_translations::supplemental_translation;

    #[test]
    fn every_canonical_assignment_state_is_localized_in_both_languages() {
        for (value, english, persian) in [
            ("Draft", "Draft", "پیش‌نویس"),
            ("Published", "Published", "منتشرشده"),
            ("InProgress", "In progress", "در حال انجام"),
            ("Pending", "Pending", "در انتظار"),
            ("Overdue", "Overdue", "عقب‌افتاده"),
            ("Submitted", "Submitted", "ارسال‌شده"),
            ("Graded", "Graded", "نمره‌گذاری‌شده"),
            ("Archived", "Archived", "بایگانی‌شده"),
            ("Active", "Active", "فعال"),
            ("Grading", "Grading", "در حال نمره‌دهی"),
            ("Complete", "Complete", "تکمیل‌شده"),
        ] {
            assert_eq!(assignment_status_label(value, Locale::En), english);
            assert_eq!(assignment_status_label(value, Locale::Fa), persian);
        }
    }

    #[test]
    fn status_normalization_accepts_protocol_and_display_spellings() {
        assert_eq!(
            assignment_status_label(" in_progress ", Locale::Fa),
            "در حال انجام"
        );
        assert_eq!(
            assignment_status_label("IN-PROGRESS", Locale::En),
            "In progress"
        );
        assert_eq!(assignment_status_label("completed", Locale::Fa), "تکمیل‌شده");
    }

    #[test]
    fn unknown_status_is_preserved_as_data_instead_of_inventing_a_label() {
        assert_eq!(
            assignment_status_label("School custom state", Locale::Fa),
            "School custom state"
        );
    }

    #[test]
    fn legacy_assignment_status_translation_keys_have_bilingual_parity() {
        for key in [
            "assignment.status.draft",
            "assignment.status.published",
            "assignment.status.active",
            "assignment.status.grading",
            "assignment.status.complete",
        ] {
            let en = supplemental_translation(key, Locale::En);
            let fa = supplemental_translation(key, Locale::Fa);
            assert!(en.is_some(), "missing English {key}");
            assert!(fa.is_some(), "missing Farsi {key}");
            assert_ne!(en, Some(key), "raw English key leaked for {key}");
            assert_ne!(fa, Some(key), "raw Farsi key leaked for {key}");
        }
    }
}
