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
        (Locale::En, "parent.ai.title") => Some("AI chat about your child's learning"),
        (Locale::Fa, "parent.ai.title") => Some("گفت‌وگو با هوش مصنوعی درباره وضعیت تحصیلی فرزندم"),
        (Locale::En, "parent.ai.intro") => Some("Ask about recorded assignments, grades, and practical learning suggestions."),
        (Locale::Fa, "parent.ai.intro") => Some("درباره تکلیف‌ها، نمره‌های ثبت‌شده و پیشنهادهای کمک آموزشی پرسش کنید."),
        (Locale::En, "parent.ai.privacy") => Some("The minimum necessary academic facts are sent through the school’s configured AI provider. No chat transcript is saved on the server. AI guidance may be inaccurate; check original grades and assignments."),
        (Locale::Fa, "parent.ai.privacy") => Some("حداقل داده‌های تحصیلی لازم از طریق ارائه‌دهندهٔ هوش مصنوعی تعیین‌شدهٔ مدرسه پردازش می‌شوند. گفت‌وگو در سرور ذخیره نمی‌شود. پاسخ هوش مصنوعی ممکن است خطا داشته باشد؛ نمره‌ها و تکلیف‌های اصلی را بررسی کنید."),
        (Locale::En, "parent.ai.consent") => Some("I understand that limited school records about the selected child will be processed by the school's AI provider."),
        (Locale::Fa, "parent.ai.consent") => Some("می‌دانم که سوابق تحصیلی محدود فرزند انتخاب‌شده توسط ارائه‌دهندهٔ هوش مصنوعی مدرسه پردازش می‌شوند."),
        (Locale::En, "parent.ai.open") => Some("Open academic AI chat"),
        (Locale::Fa, "parent.ai.open") => Some("باز کردن گفت‌وگوی تحصیلی"),
        (Locale::En, "parent.ai.select_child") => Some("Student"),
        (Locale::Fa, "parent.ai.select_child") => Some("دانش‌آموز"),
        (Locale::En, "parent.ai.choose_child") => Some("Choose a linked child"),
        (Locale::Fa, "parent.ai.choose_child") => Some("فرزند متصل را انتخاب کنید"),
        (Locale::En, "parent.ai.empty") => Some("Ask your first question. Only this child's current authorized records are used."),
        (Locale::Fa, "parent.ai.empty") => Some("پرسش نخست خود را بنویسید. فقط سوابق مجاز و فعلی این فرزند بررسی می‌شوند."),
        (Locale::En, "parent.ai.question") => Some("Your question"),
        (Locale::Fa, "parent.ai.question") => Some("پرسش شما"),
        (Locale::En, "parent.ai.history") => Some("Current child's chat history"),
        (Locale::Fa, "parent.ai.history") => Some("تاریخچه گفت‌وگوی فرزند انتخاب‌شده"),
        (Locale::En, "parent.ai.ask") => Some("Ask AI"),
        (Locale::Fa, "parent.ai.ask") => Some("پرسش از هوش مصنوعی"),
        (Locale::En, "parent.ai.generating") => Some("Preparing grounded answer…"),
        (Locale::Fa, "parent.ai.generating") => Some("در حال آماده‌سازی پاسخ مستند…"),
        (Locale::En, "parent.ai.unavailable") => Some("AI guidance is unavailable. Your child's recorded grades and assignments remain accessible in Children."),
        (Locale::Fa, "parent.ai.unavailable") => Some("راهنمایی هوش مصنوعی در دسترس نیست. نمره‌ها و تکلیف‌های ثبت‌شدهٔ فرزندتان در بخش فرزندان قابل مشاهده‌اند."),
        (Locale::En, "parent.ai.example_missing") => Some("Which assignments are still pending?"),
        (Locale::Fa, "parent.ai.example_missing") => Some("کدام تکلیف‌ها هنوز انجام نشده‌اند؟"),
        (Locale::En, "parent.ai.example_home") => Some("How can I help with study at home?"),
        (Locale::Fa, "parent.ai.example_home") => Some("در خانه چطور به درس خواندن کمک کنم؟"),
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
        "parent.ai.title",
        "parent.ai.intro",
        "parent.ai.privacy",
        "parent.ai.open",
        "parent.ai.consent",
        "parent.ai.select_child",
        "parent.ai.choose_child",
        "parent.ai.empty",
        "parent.ai.question",
        "parent.ai.history",
        "parent.ai.ask",
        "parent.ai.generating",
        "parent.ai.unavailable",
        "parent.ai.example_missing",
        "parent.ai.example_home",
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
