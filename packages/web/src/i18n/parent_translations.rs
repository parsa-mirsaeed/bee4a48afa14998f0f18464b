use super::Locale;

pub(crate) fn parent_translation(key: &'static str, locale: Locale) -> Option<&'static str> {
    match (locale, key) {
        (Locale::En, "parent.dashboard.intro") => {
            Some("Review your linked children, their classes, assignments, and recorded grades.")
        }
        (Locale::Fa, "parent.dashboard.intro") => {
            Some("فرزندان متصل، کلاس‌ها، تکلیف‌ها و نمره‌های ثبت‌شده آن‌ها را مرور کنید.")
        }
        (Locale::En, "parent.dashboard.loading") => Some("Loading family information…"),
        (Locale::Fa, "parent.dashboard.loading") => Some("در حال بارگذاری اطلاعات خانواده…"),
        (Locale::En, "parent.dashboard.load_error") => Some("Family information could not be loaded."),
        (Locale::Fa, "parent.dashboard.load_error") => Some("بارگذاری اطلاعات خانواده ممکن نشد."),
        (Locale::En, "parent.dashboard.empty_title") => {
            Some("No student is linked to this parent account yet")
        }
        (Locale::Fa, "parent.dashboard.empty_title") => {
            Some("هنوز دانش‌آموزی به این حساب والد متصل نشده است")
        }
        (Locale::En, "parent.dashboard.empty_description") => {
            Some("School administration must link a student before academic information appears.")
        }
        (Locale::Fa, "parent.dashboard.empty_description") => {
            Some("مدیریت مدرسه باید یک دانش‌آموز را به این حساب متصل کند تا اطلاعات تحصیلی نمایش داده شود.")
        }
        (Locale::En, "parent.dashboard.stats.enrolled_classes") => Some("Enrolled classes"),
        (Locale::Fa, "parent.dashboard.stats.enrolled_classes") => Some("کلاس‌های ثبت‌نام‌شده"),
        (Locale::En, "parent.dashboard.view_details") => Some("View details"),
        (Locale::Fa, "parent.dashboard.view_details") => Some("مشاهده جزئیات"),
        (Locale::En, "parent.child.grade_not_recorded") => Some("Grade level not recorded"),
        (Locale::Fa, "parent.child.grade_not_recorded") => Some("پایه تحصیلی ثبت نشده است"),

        (Locale::En, "parent.children.load_error") => Some("Children could not be loaded."),
        (Locale::Fa, "parent.children.load_error") => Some("بارگذاری فرزندان ممکن نشد."),
        (Locale::En, "parent.children.grades.loading") => Some("Loading recorded grades…"),
        (Locale::Fa, "parent.children.grades.loading") => Some("در حال بارگذاری نمره‌های ثبت‌شده…"),
        (Locale::En, "parent.children.grades.load_error") => {
            Some("Recorded grades could not be loaded.")
        }
        (Locale::Fa, "parent.children.grades.load_error") => {
            Some("بارگذاری نمره‌های ثبت‌شده ممکن نشد.")
        }
        (Locale::En, "parent.children.assignments.loading") => Some("Loading assignments…"),
        (Locale::Fa, "parent.children.assignments.loading") => Some("در حال بارگذاری تکلیف‌ها…"),
        (Locale::En, "parent.children.assignments.load_error") => {
            Some("Assignments could not be loaded.")
        }
        (Locale::Fa, "parent.children.assignments.load_error") => {
            Some("بارگذاری تکلیف‌ها ممکن نشد.")
        }
        (Locale::En, "parent.children.assignments.empty") => Some("No assignments available."),
        (Locale::Fa, "parent.children.assignments.empty") => Some("تکلیفی برای نمایش وجود ندارد."),
        (Locale::En, "parent.children.assignments.due_label") => Some("Due"),
        (Locale::Fa, "parent.children.assignments.due_label") => Some("مهلت"),
        _ => None,
    }
}

pub(crate) fn format_parent_class_count(count: i64, locale: Locale) -> String {
    match locale {
        Locale::En if count == 1 => "1 class".to_string(),
        Locale::En => format!("{count} classes"),
        Locale::Fa => format!("{count} کلاس"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEYS: &[&str] = &[
        "parent.dashboard.intro",
        "parent.dashboard.loading",
        "parent.dashboard.load_error",
        "parent.dashboard.empty_title",
        "parent.dashboard.empty_description",
        "parent.dashboard.stats.enrolled_classes",
        "parent.dashboard.view_details",
        "parent.child.grade_not_recorded",
        "parent.children.load_error",
        "parent.children.grades.loading",
        "parent.children.grades.load_error",
        "parent.children.assignments.loading",
        "parent.children.assignments.load_error",
        "parent.children.assignments.empty",
        "parent.children.assignments.due_label",
    ];

    #[test]
    fn parent_translation_keys_have_en_fa_parity() {
        for key in KEYS {
            let en = parent_translation(key, Locale::En);
            let fa = parent_translation(key, Locale::Fa);
            assert!(en.is_some(), "missing English {key}");
            assert!(fa.is_some(), "missing Farsi {key}");
            assert_ne!(en, Some(*key), "raw English key leaked for {key}");
            assert_ne!(fa, Some(*key), "raw Farsi key leaked for {key}");
        }
    }

    #[test]
    fn parent_class_count_has_meaningful_localized_context() {
        assert_eq!(format_parent_class_count(1, Locale::En), "1 class");
        assert_eq!(format_parent_class_count(2, Locale::En), "2 classes");
        assert_eq!(format_parent_class_count(2, Locale::Fa), "2 کلاس");
    }
}
