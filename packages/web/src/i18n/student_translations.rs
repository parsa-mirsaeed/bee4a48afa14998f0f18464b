use super::Locale;

pub(crate) fn student_translation(key: &'static str, locale: Locale) -> Option<&'static str> {
    match (locale, key) {
        (Locale::En, "student.dashboard.intro") => {
            Some("Start with work that needs attention, then review your classes and grades.")
        }
        (Locale::Fa, "student.dashboard.intro") => {
            Some("ابتدا کارهایی را ببینید که نیاز به اقدام دارند، سپس کلاس‌ها و نمره‌های خود را مرور کنید.")
        }
        (Locale::En, "student.dashboard.view_all") => Some("View all"),
        (Locale::Fa, "student.dashboard.view_all") => Some("مشاهده همه"),
        (Locale::En, "student.dashboard.loading_assignments") => Some("Loading assignments…"),
        (Locale::Fa, "student.dashboard.loading_assignments") => {
            Some("در حال بارگذاری تکلیف‌ها…")
        }
        (Locale::En, "student.dashboard.assignments_load_error") => {
            Some("Unable to load assignments.")
        }
        (Locale::Fa, "student.dashboard.assignments_load_error") => {
            Some("بارگذاری تکلیف‌ها ناموفق بود.")
        }
        (Locale::En, "student.dashboard.loading_classes") => Some("Loading classes…"),
        (Locale::Fa, "student.dashboard.loading_classes") => Some("در حال بارگذاری کلاس‌ها…"),
        (Locale::En, "student.dashboard.classes_load_error") => Some("Unable to load classes."),
        (Locale::Fa, "student.dashboard.classes_load_error") => {
            Some("بارگذاری کلاس‌ها ناموفق بود.")
        }
        (Locale::En, "student.dashboard.no_upcoming_assignments") => {
            Some("No upcoming assignments.")
        }
        (Locale::Fa, "student.dashboard.no_upcoming_assignments") => {
            Some("تکلیف آینده‌ای وجود ندارد.")
        }

        (Locale::En, "student.assignments.filter_all") => Some("All"),
        (Locale::Fa, "student.assignments.filter_all") => Some("همه"),
        (Locale::En, "student.assignments.filter_pending") => Some("Pending"),
        (Locale::Fa, "student.assignments.filter_pending") => Some("در انتظار"),
        (Locale::En, "student.assignments.filter_overdue") => Some("Overdue"),
        (Locale::Fa, "student.assignments.filter_overdue") => Some("عقب‌افتاده"),
        (Locale::En, "student.assignments.filter_submitted") => Some("Submitted"),
        (Locale::Fa, "student.assignments.filter_submitted") => Some("ارسال‌شده"),
        (Locale::En, "student.assignments.filter_graded") => Some("Graded"),
        (Locale::Fa, "student.assignments.filter_graded") => Some("نمره‌گذاری‌شده"),
        (Locale::En, "student.assignments.load_error") => Some("Assignments could not be loaded."),
        (Locale::Fa, "student.assignments.load_error") => Some("بارگذاری تکلیف‌ها ممکن نشد."),
        (Locale::En, "student.assignments.try_again") => Some("Try again"),
        (Locale::Fa, "student.assignments.try_again") => Some("دوباره تلاش کنید"),
        (Locale::En, "student.assignments.empty_title") => Some("No assignments yet"),
        (Locale::Fa, "student.assignments.empty_title") => Some("هنوز تکلیفی وجود ندارد"),
        (Locale::En, "student.assignments.empty_description") => {
            Some("Published work from your enrolled classes will appear here.")
        }
        (Locale::Fa, "student.assignments.empty_description") => {
            Some("تکلیف‌های منتشرشده کلاس‌هایی که در آن‌ها ثبت‌نام کرده‌اید اینجا نمایش داده می‌شوند.")
        }
        (Locale::En, "student.assignments.no_filter_matches") => {
            Some("No assignments match this filter.")
        }
        (Locale::Fa, "student.assignments.no_filter_matches") => {
            Some("هیچ تکلیفی با این فیلتر مطابقت ندارد.")
        }
        (Locale::En, "student.assignments.clear_filter") => Some("Clear filter"),
        (Locale::Fa, "student.assignments.clear_filter") => Some("پاک کردن فیلتر"),
        (Locale::En, "student.assignments.points_label") => Some("points"),
        (Locale::Fa, "student.assignments.points_label") => Some("امتیاز"),
        (Locale::En, "student.assignments.points_unspecified") => Some("Points not specified"),
        (Locale::Fa, "student.assignments.points_unspecified") => Some("امتیاز مشخص نشده است"),
        (Locale::En, "student.assignments.due_label") => Some("Due"),
        (Locale::Fa, "student.assignments.due_label") => Some("مهلت"),
        (Locale::En, "student.assignments.grade_label") => Some("Grade"),
        (Locale::Fa, "student.assignments.grade_label") => Some("نمره"),
        (Locale::En, "student.assignments.action_start") => Some("Start assignment"),
        (Locale::Fa, "student.assignments.action_start") => Some("شروع تکلیف"),
        (Locale::En, "student.assignments.action_late") => Some("Submit late"),
        (Locale::Fa, "student.assignments.action_late") => Some("ارسال با تأخیر"),
        (Locale::En, "student.assignments.action_submission") => Some("View submission"),
        (Locale::Fa, "student.assignments.action_submission") => Some("مشاهده ارسال"),
        (Locale::En, "student.assignments.action_feedback") => Some("View feedback"),
        (Locale::Fa, "student.assignments.action_feedback") => Some("مشاهده بازخورد"),
        (Locale::En, "student.assignments.details_title") => Some("Assignment details"),
        (Locale::Fa, "student.assignments.details_title") => Some("جزئیات تکلیف"),
        (Locale::En, "student.assignments.details_loading") => Some("Loading assignment…"),
        (Locale::Fa, "student.assignments.details_loading") => Some("در حال بارگذاری تکلیف…"),
        (Locale::En, "student.assignments.details_load_error") => {
            Some("The assignment could not be loaded.")
        }
        (Locale::Fa, "student.assignments.details_load_error") => {
            Some("بارگذاری تکلیف ممکن نشد.")
        }
        (Locale::En, "student.assignments.details_unavailable") => {
            Some("This assignment is no longer available.")
        }
        (Locale::Fa, "student.assignments.details_unavailable") => {
            Some("این تکلیف دیگر در دسترس نیست.")
        }
        (Locale::En, "student.assignments.status_label") => Some("Status"),
        (Locale::Fa, "student.assignments.status_label") => Some("وضعیت"),
        (Locale::En, "student.assignments.no_written_feedback") => {
            Some("Your teacher has not added written feedback.")
        }
        (Locale::Fa, "student.assignments.no_written_feedback") => {
            Some("معلم شما هنوز بازخورد نوشتاری اضافه نکرده است.")
        }
        (Locale::En, "student.assignments.feedback_unavailable") => {
            Some("Feedback is not available yet.")
        }
        (Locale::Fa, "student.assignments.feedback_unavailable") => {
            Some("بازخورد هنوز در دسترس نیست.")
        }
        (Locale::En, "student.assignments.feedback_loading") => Some("Loading feedback…"),
        (Locale::Fa, "student.assignments.feedback_loading") => Some("در حال بارگذاری بازخورد…"),
        (Locale::En, "student.assignments.open_submission") => Some("Open my submission"),
        (Locale::Fa, "student.assignments.open_submission") => Some("باز کردن ارسال من"),
        (Locale::En, "student.assignments.work_title") => Some("My submission"),
        (Locale::Fa, "student.assignments.work_title") => Some("ارسال من"),
        (Locale::En, "student.assignments.enter_work") => {
            Some("Enter your work before submitting.")
        }
        (Locale::Fa, "student.assignments.enter_work") => {
            Some("پیش از ارسال، پاسخ خود را وارد کنید.")
        }
        (Locale::En, "student.assignments.save_failed") => {
            Some("Your work was not saved. The text is still here; try again.")
        }
        (Locale::Fa, "student.assignments.save_failed") => {
            Some("کار شما ذخیره نشد. متن همچنان اینجاست؛ دوباره تلاش کنید.")
        }
        (Locale::En, "student.assignments.saved_work_loading") => Some("Loading saved work…"),
        (Locale::Fa, "student.assignments.saved_work_loading") => {
            Some("در حال بارگذاری کار ذخیره‌شده…")
        }
        (Locale::En, "student.assignments.saved_work_load_error") => Some(
            "Saved work could not be loaded. Refresh before overwriting if you previously submitted.",
        ),
        (Locale::Fa, "student.assignments.saved_work_load_error") => Some(
            "کار ذخیره‌شده بارگذاری نشد. اگر قبلاً ارسال کرده‌اید، پیش از جایگزینی صفحه را تازه‌سازی کنید.",
        ),
        (Locale::En, "student.assignments.submitting") => Some("Submitting…"),
        (Locale::Fa, "student.assignments.submitting") => Some("در حال ارسال…"),
        (Locale::En, "student.assignments.submit_work") => Some("Submit work"),
        (Locale::Fa, "student.assignments.submit_work") => Some("ارسال کار"),

        (Locale::En, "student.grades.recorded_title") => Some("Recorded grades"),
        (Locale::Fa, "student.grades.recorded_title") => Some("نمره‌های ثبت‌شده"),
        (Locale::En, "student.grades.recorded_description") => {
            Some("Grades recorded by your teachers appear here.")
        }
        (Locale::Fa, "student.grades.recorded_description") => {
            Some("نمره‌هایی که معلمان شما ثبت کرده‌اند اینجا نمایش داده می‌شوند.")
        }
        (Locale::En, "student.grades.load_error") => Some("Unable to load grades."),
        (Locale::Fa, "student.grades.load_error") => Some("بارگذاری نمره‌ها ممکن نشد."),
        (Locale::En, "student.grades.detail_load_error") => {
            Some("Unable to load recorded grades.")
        }
        (Locale::Fa, "student.grades.detail_load_error") => {
            Some("بارگذاری نمره‌های ثبت‌شده ممکن نشد.")
        }

        (Locale::En, "student.classes.load_error") => Some("Classes could not be loaded."),
        (Locale::Fa, "student.classes.load_error") => Some("بارگذاری کلاس‌ها ممکن نشد."),
        (Locale::En, "student.classes.tasks_load_error") => {
            Some("Class assignments could not be loaded.")
        }
        (Locale::Fa, "student.classes.tasks_load_error") => {
            Some("بارگذاری تکلیف‌های کلاس ممکن نشد.")
        }
        (Locale::En, "student.classes.grades_load_error") => {
            Some("Class grades could not be loaded.")
        }
        (Locale::Fa, "student.classes.grades_load_error") => {
            Some("بارگذاری نمره‌های کلاس ممکن نشد.")
        }
        (Locale::En, "student.classes.materials_load_error") => {
            Some("Class materials could not be loaded.")
        }
        (Locale::Fa, "student.classes.materials_load_error") => {
            Some("بارگذاری منابع کلاس ممکن نشد.")
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CANONICAL_KEYS: &[&str] = &[
        "student.dashboard.intro",
        "student.dashboard.view_all",
        "student.dashboard.loading_assignments",
        "student.dashboard.assignments_load_error",
        "student.dashboard.loading_classes",
        "student.dashboard.classes_load_error",
        "student.dashboard.no_upcoming_assignments",
        "student.assignments.filter_all",
        "student.assignments.filter_pending",
        "student.assignments.filter_overdue",
        "student.assignments.filter_submitted",
        "student.assignments.filter_graded",
        "student.assignments.load_error",
        "student.assignments.try_again",
        "student.assignments.empty_title",
        "student.assignments.empty_description",
        "student.assignments.no_filter_matches",
        "student.assignments.clear_filter",
        "student.assignments.points_label",
        "student.assignments.points_unspecified",
        "student.assignments.due_label",
        "student.assignments.grade_label",
        "student.assignments.action_start",
        "student.assignments.action_late",
        "student.assignments.action_submission",
        "student.assignments.action_feedback",
        "student.assignments.details_title",
        "student.assignments.details_loading",
        "student.assignments.details_load_error",
        "student.assignments.details_unavailable",
        "student.assignments.status_label",
        "student.assignments.no_written_feedback",
        "student.assignments.feedback_unavailable",
        "student.assignments.feedback_loading",
        "student.assignments.open_submission",
        "student.assignments.work_title",
        "student.assignments.enter_work",
        "student.assignments.save_failed",
        "student.assignments.saved_work_loading",
        "student.assignments.saved_work_load_error",
        "student.assignments.submitting",
        "student.assignments.submit_work",
        "student.grades.recorded_title",
        "student.grades.recorded_description",
        "student.grades.load_error",
        "student.grades.detail_load_error",
        "student.classes.load_error",
        "student.classes.tasks_load_error",
        "student.classes.grades_load_error",
        "student.classes.materials_load_error",
    ];

    #[test]
    fn canonical_student_copy_has_english_and_farsi_parity() {
        for key in CANONICAL_KEYS {
            for locale in [Locale::En, Locale::Fa] {
                let translated = student_translation(key, locale);
                assert!(
                    translated.is_some(),
                    "missing {locale:?} translation for {key}"
                );
                assert_ne!(translated, Some(*key), "raw key fallback for {key}");
            }
        }
    }
}
