use super::Locale;

pub(crate) fn teacher_assignments_translation(
    key: &'static str,
    locale: Locale,
) -> Option<&'static str> {
    match (locale, key) {
        (Locale::En, "teacher.assignments.deleted_notice") => Some("Assignment deleted."),
        (Locale::Fa, "teacher.assignments.deleted_notice") => Some("تکلیف حذف شد."),
        (Locale::En, "teacher.assignments.delete_failed") => {
            Some("The assignment could not be deleted. Refresh and try again.")
        }
        (Locale::Fa, "teacher.assignments.delete_failed") => {
            Some("حذف تکلیف ممکن نشد. صفحه را تازه‌سازی کنید و دوباره تلاش کنید.")
        }
        (Locale::En, "teacher.assignments.load_error") => Some("Assignments could not be loaded."),
        (Locale::Fa, "teacher.assignments.load_error") => Some("بارگذاری تکلیف‌ها ممکن نشد."),
        (Locale::En, "teacher.assignments.try_again") => Some("Try again"),
        (Locale::Fa, "teacher.assignments.try_again") => Some("دوباره تلاش کنید"),
        (Locale::En, "teacher.assignments.empty_title") => Some("No assignments yet"),
        (Locale::Fa, "teacher.assignments.empty_title") => Some("هنوز تکلیفی وجود ندارد"),
        (Locale::En, "teacher.assignments.empty_description") => {
            Some("Create a draft for one of your assigned classes.")
        }
        (Locale::Fa, "teacher.assignments.empty_description") => {
            Some("برای یکی از کلاس‌های خود یک پیش‌نویس تکلیف ایجاد کنید.")
        }
        (Locale::En, "teacher.assignments.no_filter_matches") => {
            Some("No assignments match this filter.")
        }
        (Locale::Fa, "teacher.assignments.no_filter_matches") => {
            Some("هیچ تکلیفی با این فیلتر مطابقت ندارد.")
        }
        (Locale::En, "teacher.assignments.clear_filter") => Some("Clear filter"),
        (Locale::Fa, "teacher.assignments.clear_filter") => Some("پاک کردن فیلتر"),
        (Locale::En, "teacher.assignments.draft_created_notice") => {
            Some("Draft assignment created.")
        }
        (Locale::Fa, "teacher.assignments.draft_created_notice") => {
            Some("پیش‌نویس تکلیف ایجاد شد.")
        }
        (Locale::En, "teacher.assignments.published_notice") => Some("Assignment published."),
        (Locale::Fa, "teacher.assignments.published_notice") => Some("تکلیف منتشر شد."),
        (Locale::En, "teacher.assignments.delete_title") => Some("Delete assignment"),
        (Locale::Fa, "teacher.assignments.delete_title") => Some("حذف تکلیف"),
        (Locale::En, "teacher.assignments.delete_confirmation") => Some(
            "Delete this assignment? This action may remove its downstream assignment records and cannot be undone from this screen.",
        ),
        (Locale::Fa, "teacher.assignments.delete_confirmation") => Some(
            "این تکلیف حذف شود؟ این کار ممکن است سوابق وابسته به تکلیف را نیز حذف کند و از این صفحه قابل بازگشت نیست.",
        ),
        (Locale::En, "teacher.assignments.required_fields") => {
            Some("Title, class, due date, and instructions are required.")
        }
        (Locale::Fa, "teacher.assignments.required_fields") => {
            Some("عنوان، کلاس، تاریخ مهلت و دستورالعمل‌ها الزامی هستند.")
        }
        (Locale::En, "teacher.assignments.invalid_due_date") => Some("The due date is invalid."),
        (Locale::Fa, "teacher.assignments.invalid_due_date") => Some("تاریخ مهلت معتبر نیست."),
        (Locale::En, "teacher.assignments.create_failed") => {
            Some("The draft could not be created. Check the class and try again.")
        }
        (Locale::Fa, "teacher.assignments.create_failed") => {
            Some("ایجاد پیش‌نویس ممکن نشد. کلاس را بررسی کنید و دوباره تلاش کنید.")
        }
        (Locale::En, "teacher.assignments.title_label") => Some("Title"),
        (Locale::Fa, "teacher.assignments.title_label") => Some("عنوان"),
        (Locale::En, "teacher.assignments.class_label") => Some("Class"),
        (Locale::Fa, "teacher.assignments.class_label") => Some("کلاس"),
        (Locale::En, "teacher.assignments.select_class") => Some("Select one of your classes"),
        (Locale::Fa, "teacher.assignments.select_class") => Some("یکی از کلاس‌های خود را انتخاب کنید"),
        (Locale::En, "teacher.assignments.classes_load_error") => {
            Some("Unable to load assigned classes")
        }
        (Locale::Fa, "teacher.assignments.classes_load_error") => {
            Some("بارگذاری کلاس‌های واگذارشده ممکن نیست")
        }
        (Locale::En, "teacher.assignments.classes_loading") => Some("Loading assigned classes…"),
        (Locale::Fa, "teacher.assignments.classes_loading") => {
            Some("در حال بارگذاری کلاس‌های واگذارشده…")
        }
        (Locale::En, "teacher.assignments.due_date_label") => Some("Due date"),
        (Locale::Fa, "teacher.assignments.due_date_label") => Some("تاریخ مهلت"),
        (Locale::En, "teacher.assignments.instructions_label") => Some("Instructions"),
        (Locale::Fa, "teacher.assignments.instructions_label") => Some("دستورالعمل‌ها"),
        (Locale::En, "teacher.assignments.creating") => Some("Creating…"),
        (Locale::Fa, "teacher.assignments.creating") => Some("در حال ایجاد…"),
        (Locale::En, "teacher.assignments.create_draft") => Some("Create draft"),
        (Locale::Fa, "teacher.assignments.create_draft") => Some("ایجاد پیش‌نویس"),
        (Locale::En, "teacher.assignments.materials_legend") => {
            Some("Governed class materials (optional)")
        }
        (Locale::Fa, "teacher.assignments.materials_legend") => {
            Some("منابع تأییدشده کلاس (اختیاری)")
        }
        (Locale::En, "teacher.assignments.materials_loading") => Some("Loading materials…"),
        (Locale::Fa, "teacher.assignments.materials_loading") => Some("در حال بارگذاری منابع…"),
        (Locale::En, "teacher.assignments.materials_load_error") => Some(
            "Materials could not be loaded. You can still create the assignment without them.",
        ),
        (Locale::Fa, "teacher.assignments.materials_load_error") => Some(
            "بارگذاری منابع ممکن نشد. همچنان می‌توانید تکلیف را بدون آن‌ها ایجاد کنید.",
        ),
        (Locale::En, "teacher.assignments.materials_empty") => {
            Some("No class materials are available.")
        }
        (Locale::Fa, "teacher.assignments.materials_empty") => {
            Some("هیچ منبعی برای این کلاس در دسترس نیست.")
        }
        (Locale::En, "teacher.assignments.details_title") => Some("Assignment details"),
        (Locale::Fa, "teacher.assignments.details_title") => Some("جزئیات تکلیف"),
        (Locale::En, "teacher.assignments.details_loading") => Some("Loading assignment…"),
        (Locale::Fa, "teacher.assignments.details_loading") => Some("در حال بارگذاری تکلیف…"),
        (Locale::En, "teacher.assignments.details_load_error") => {
            Some("The assignment could not be loaded.")
        }
        (Locale::Fa, "teacher.assignments.details_load_error") => {
            Some("بارگذاری تکلیف ممکن نشد.")
        }
        (Locale::En, "teacher.assignments.details_unavailable") => {
            Some("This assignment is no longer available.")
        }
        (Locale::Fa, "teacher.assignments.details_unavailable") => {
            Some("این تکلیف دیگر در دسترس نیست.")
        }
        (Locale::En, "teacher.assignments.status_prefix") => Some("Status"),
        (Locale::Fa, "teacher.assignments.status_prefix") => Some("وضعیت"),
        (Locale::En, "teacher.assignments.publishing") => Some("Publishing…"),
        (Locale::Fa, "teacher.assignments.publishing") => Some("در حال انتشار…"),
        (Locale::En, "teacher.assignments.publish") => Some("Publish"),
        (Locale::Fa, "teacher.assignments.publish") => Some("انتشار"),
        (Locale::En, "teacher.assignments.no_eligible_students") => Some(
            "This assignment cannot be published because the class has no active enrolled students. Ask a School Manager to enroll at least one student, then try again.",
        ),
        (Locale::Fa, "teacher.assignments.no_eligible_students") => Some(
            "این تکلیف قابل انتشار نیست چون کلاس دانش‌آموز فعال ثبت‌نام‌شده‌ای ندارد. از مدیر مدرسه بخواهید دست‌کم یک دانش‌آموز را ثبت‌نام کند و سپس دوباره تلاش کنید.",
        ),
        (Locale::En, "teacher.assignments.publish_conflict") => Some(
            "The assignment or class changed while publishing. Refresh the assignment and try again.",
        ),
        (Locale::Fa, "teacher.assignments.publish_conflict") => Some(
            "تکلیف یا کلاس هنگام انتشار تغییر کرد. تکلیف را تازه‌سازی کنید و دوباره تلاش کنید.",
        ),
        (Locale::En, "teacher.assignments.publish_unavailable") => {
            Some("This assignment is no longer available to your teacher account.")
        }
        (Locale::Fa, "teacher.assignments.publish_unavailable") => {
            Some("این تکلیف دیگر برای حساب معلم شما در دسترس نیست.")
        }
        (Locale::En, "teacher.assignments.publish_failed") => {
            Some("The assignment could not be published. Refresh and try again.")
        }
        (Locale::Fa, "teacher.assignments.publish_failed") => {
            Some("انتشار تکلیف ممکن نشد. صفحه را تازه‌سازی کنید و دوباره تلاش کنید.")
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::supplemental_translation;

    const DEDICATED_KEYS: &[&str] = &[
        "teacher.assignments.deleted_notice",
        "teacher.assignments.delete_failed",
        "teacher.assignments.load_error",
        "teacher.assignments.try_again",
        "teacher.assignments.empty_title",
        "teacher.assignments.empty_description",
        "teacher.assignments.no_filter_matches",
        "teacher.assignments.clear_filter",
        "teacher.assignments.draft_created_notice",
        "teacher.assignments.published_notice",
        "teacher.assignments.delete_title",
        "teacher.assignments.delete_confirmation",
        "teacher.assignments.required_fields",
        "teacher.assignments.invalid_due_date",
        "teacher.assignments.create_failed",
        "teacher.assignments.title_label",
        "teacher.assignments.class_label",
        "teacher.assignments.select_class",
        "teacher.assignments.classes_load_error",
        "teacher.assignments.classes_loading",
        "teacher.assignments.due_date_label",
        "teacher.assignments.instructions_label",
        "teacher.assignments.creating",
        "teacher.assignments.create_draft",
        "teacher.assignments.materials_legend",
        "teacher.assignments.materials_loading",
        "teacher.assignments.materials_load_error",
        "teacher.assignments.materials_empty",
        "teacher.assignments.details_title",
        "teacher.assignments.details_loading",
        "teacher.assignments.details_load_error",
        "teacher.assignments.details_unavailable",
        "teacher.assignments.status_prefix",
        "teacher.assignments.publishing",
        "teacher.assignments.publish",
        "teacher.assignments.no_eligible_students",
        "teacher.assignments.publish_conflict",
        "teacher.assignments.publish_unavailable",
        "teacher.assignments.publish_failed",
    ];

    const EXISTING_KEYS: &[&str] = &[
        "teacher.assignments.all_filter",
        "teacher.assignments.draft_filter",
        "teacher.assignments.active_filter",
        "teacher.assignments.complete_filter",
        "teacher.assignments.create",
        "teacher.assignments.due_prefix",
        "teacher.assignments.submitted_count",
        "teacher.assignments.submission_progress",
        "teacher.assignments.view_details",
        "teacher.assignments.delete",
        "assignment.status.draft",
        "assignment.status.published",
        "assignment.status.active",
        "assignment.status.grading",
        "assignment.status.complete",
    ];

    #[test]
    fn canonical_assignment_workflow_has_english_and_farsi_parity() {
        for key in DEDICATED_KEYS {
            for locale in [Locale::En, Locale::Fa] {
                let translated = teacher_assignments_translation(key, locale);
                assert!(
                    translated.is_some(),
                    "missing {locale:?} translation for {key}"
                );
                assert_ne!(translated, Some(*key), "raw key fallback for {key}");
            }
        }
        for key in EXISTING_KEYS {
            for locale in [Locale::En, Locale::Fa] {
                let translated = supplemental_translation(key, locale);
                assert!(
                    translated.is_some(),
                    "missing {locale:?} translation for {key}"
                );
                assert_ne!(translated, Some(*key), "raw key fallback for {key}");
            }
        }
    }

    #[test]
    fn no_student_publish_guidance_remains_actionable_in_both_locales() {
        let english =
            teacher_assignments_translation("teacher.assignments.no_eligible_students", Locale::En)
                .unwrap();
        let persian =
            teacher_assignments_translation("teacher.assignments.no_eligible_students", Locale::Fa)
                .unwrap();
        assert!(english.contains("School Manager"));
        assert!(english.contains("active enrolled students"));
        assert!(persian.contains("مدیر مدرسه"));
        assert!(!persian.contains("School Manager"));
    }
}
