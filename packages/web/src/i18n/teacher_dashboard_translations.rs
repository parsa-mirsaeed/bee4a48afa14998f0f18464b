use super::Locale;

pub(crate) fn teacher_dashboard_translation(
    key: &'static str,
    locale: Locale,
) -> Option<&'static str> {
    match (locale, key) {
        (Locale::En, "teacher.dashboard.overview_intro") => Some(
            "See what needs attention, then move directly into assignments, grading, classes, or governed knowledge.",
        ),
        (Locale::Fa, "teacher.dashboard.overview_intro") => Some(
            "ابتدا موارد نیازمند توجه را ببینید و سپس مستقیماً به تکلیف‌ها، ارزیابی، کلاس‌ها یا منابع دانشی بروید.",
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overview_introduction_has_bilingual_parity_without_raw_key_fallback() {
        let key = "teacher.dashboard.overview_intro";
        let english = teacher_dashboard_translation(key, Locale::En);
        let persian = teacher_dashboard_translation(key, Locale::Fa);

        assert_eq!(
            english,
            Some(
                "See what needs attention, then move directly into assignments, grading, classes, or governed knowledge."
            )
        );
        assert_eq!(
            persian,
            Some(
                "ابتدا موارد نیازمند توجه را ببینید و سپس مستقیماً به تکلیف‌ها، ارزیابی، کلاس‌ها یا منابع دانشی بروید."
            )
        );
        assert_ne!(english, Some(key));
        assert_ne!(persian, Some(key));
    }
}
