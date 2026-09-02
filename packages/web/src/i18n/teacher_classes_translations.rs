use super::Locale;

pub(crate) fn teacher_classes_translation(
    key: &'static str,
    locale: Locale,
) -> Option<&'static str> {
    match (locale, key) {
        (Locale::En, "teachers.classes.assignment.unknown_title") => Some("Untitled assignment"),
        (Locale::Fa, "teachers.classes.assignment.unknown_title") => Some("تکلیف بدون عنوان"),
        (Locale::En, "teachers.classes.vectorization.checking") => Some("Checking status…"),
        (Locale::Fa, "teachers.classes.vectorization.checking") => Some("در حال بررسی وضعیت…"),
        (Locale::En, "teachers.classes.vectorization.processing_prefix") => Some("AI processing"),
        (Locale::Fa, "teachers.classes.vectorization.processing_prefix") => {
            Some("پردازش هوش مصنوعی")
        }
        (Locale::En, "teachers.classes.vectorization.cancelling") => Some("Cancelling…"),
        (Locale::Fa, "teachers.classes.vectorization.cancelling") => Some("در حال لغو…"),
        (Locale::En, "teachers.classes.vectorization.progress_label") => {
            Some("Content processing progress")
        }
        (Locale::Fa, "teachers.classes.vectorization.progress_label") => {
            Some("پیشرفت پردازش محتوا")
        }
        (Locale::En, "teachers.classes.vectorization.processing_content") => {
            Some("Processing content…")
        }
        (Locale::Fa, "teachers.classes.vectorization.processing_content") => {
            Some("در حال پردازش محتوا…")
        }
        (Locale::En, "teachers.classes.vectorization.remaining") => Some("remaining"),
        (Locale::Fa, "teachers.classes.vectorization.remaining") => Some("باقی‌مانده"),
        (Locale::En, "teachers.classes.vectorization.seconds") => Some("s"),
        (Locale::Fa, "teachers.classes.vectorization.seconds") => Some("ثانیه"),
        (Locale::En, "teachers.classes.vectorization.minutes") => Some("m"),
        (Locale::Fa, "teachers.classes.vectorization.minutes") => Some("دقیقه"),
        (Locale::En, "teachers.classes.vectorization.hours") => Some("h"),
        (Locale::Fa, "teachers.classes.vectorization.hours") => Some("ساعت"),
        (Locale::En, "teachers.classes.vectorization.complete") => Some("AI analysis complete!"),
        (Locale::Fa, "teachers.classes.vectorization.complete") => {
            Some("تحلیل هوش مصنوعی کامل شد!")
        }
        (Locale::En, "teachers.classes.vectorization.cancelled") => Some("Analysis cancelled"),
        (Locale::Fa, "teachers.classes.vectorization.cancelled") => Some("تحلیل لغو شد"),
        (Locale::En, "teachers.classes.vectorization.failed") => {
            Some("Content processing failed. Please try again.")
        }
        (Locale::Fa, "teachers.classes.vectorization.failed") => {
            Some("پردازش محتوا ناموفق بود. دوباره تلاش کنید.")
        }
        _ => None,
    }
}

pub(crate) fn format_teacher_vectorization_duration(seconds: i32, locale: Locale) -> String {
    let seconds_label = teacher_classes_translation(
        "teachers.classes.vectorization.seconds",
        locale,
    )
    .expect("vectorization seconds unit has EN/FA parity");
    let minutes_label = teacher_classes_translation(
        "teachers.classes.vectorization.minutes",
        locale,
    )
    .expect("vectorization minutes unit has EN/FA parity");
    let hours_label = teacher_classes_translation(
        "teachers.classes.vectorization.hours",
        locale,
    )
    .expect("vectorization hours unit has EN/FA parity");

    if seconds < 60 {
        format!(
            "{} {seconds_label}",
            localized_digits(&seconds.to_string(), locale)
        )
    } else if seconds < 3600 {
        format!(
            "{} {minutes_label} {} {seconds_label}",
            localized_digits(&(seconds / 60).to_string(), locale),
            localized_digits(&(seconds % 60).to_string(), locale),
        )
    } else {
        format!(
            "{} {hours_label} {} {minutes_label}",
            localized_digits(&(seconds / 3600).to_string(), locale),
            localized_digits(&((seconds % 3600) / 60).to_string(), locale),
        )
    }
}

fn localized_digits(value: &str, locale: Locale) -> String {
    if locale == Locale::En {
        return value.to_string();
    }
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
    use crate::i18n::{supplemental_translation, t};

    const CLASS_KEYS: &[&str] = &[
        "classes.my_classes",
        "teachers.classes.manage_description",
        "classes.failed_load",
        "teachers.classes.no_classes_yet",
        "teachers.classes.no_classes_assigned_desc",
        "teachers.classes.enrolled_suffix",
        "classes.subject",
        "classes.term",
        "common.view",
        "students.title",
        "nav.materials",
        "teachers.classes.actions.grading",
        "teachers.classes.modal.overview_suffix",
        "teachers.classes.enrolled_students_label",
        "classes.class_name",
        "teachers.classes.modal.students_suffix",
        "students.loading",
        "students.failed_load",
        "students.no_enrolled_class",
        "students.total",
        "students.submitted_count",
        "students.graded_count",
        "teachers.classes.modal.grading_suffix",
        "assignments.loading",
        "grades.failed_load",
        "assignments.no_class_assignments",
        "assignments.due_prefix",
        "teachers.classes.assignments.to_grade_suffix",
        "teachers.classes.assignments.total_assigned",
        "materials.loading",
        "materials.failed_load",
        "materials.no_materials_title",
        "teachers.classes.assignment.unknown_title",
        "teachers.classes.vectorization.checking",
        "teachers.classes.vectorization.processing_prefix",
        "teachers.classes.vectorization.cancelling",
        "teachers.classes.vectorization.progress_label",
        "teachers.classes.vectorization.processing_content",
        "teachers.classes.vectorization.remaining",
        "teachers.classes.vectorization.seconds",
        "teachers.classes.vectorization.minutes",
        "teachers.classes.vectorization.hours",
        "teachers.classes.vectorization.complete",
        "teachers.classes.vectorization.cancelled",
        "teachers.classes.vectorization.failed",
    ];

    fn resolve(key: &'static str, locale: Locale) -> String {
        teacher_classes_translation(key, locale)
            .or_else(|| supplemental_translation(key, locale))
            .map(str::to_owned)
            .unwrap_or_else(|| t(key, locale))
    }

    #[test]
    fn canonical_teacher_class_copy_has_english_and_farsi_parity() {
        for key in CLASS_KEYS {
            for locale in [Locale::En, Locale::Fa] {
                let translated = resolve(key, locale);
                assert_ne!(translated, *key, "raw {locale:?} key fallback for {key}");
            }
        }
    }

    #[test]
    fn vectorization_duration_is_locale_presented_without_feature_copy() {
        assert_eq!(
            format_teacher_vectorization_duration(125, Locale::En),
            "2 m 5 s"
        );
        let persian = format_teacher_vectorization_duration(125, Locale::Fa);
        assert_eq!(persian, "۲ دقیقه ۵ ثانیه");
        assert!(!persian
            .chars()
            .any(|character| character.is_ascii_digit()));
    }
}
