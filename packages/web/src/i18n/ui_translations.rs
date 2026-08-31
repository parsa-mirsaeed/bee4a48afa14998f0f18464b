use super::Locale;
use crate::domain::SystemRole;

/// Small, explicit UI translation table for copy introduced after the original
/// monolithic translation map. Keeping these keys here prevents the UI from
/// exposing raw translation identifiers while the legacy table is gradually
/// decomposed into feature modules.
pub(crate) fn supplemental_translation(key: &'static str, locale: Locale) -> Option<&'static str> {
    match (locale, key) {
        (Locale::En, "common.refresh") => Some("Refresh"),
        (Locale::Fa, "common.refresh") => Some("تازه‌سازی"),
        (Locale::En, "common.search") => Some("Search"),
        (Locale::Fa, "common.search") => Some("جستجو"),
        (Locale::En, "common.clear") => Some("Clear"),
        (Locale::Fa, "common.clear") => Some("پاک کردن"),
        (Locale::En, "common.data") => Some("Data"),
        (Locale::Fa, "common.data") => Some("داده‌ها"),
        (Locale::En, "common.go_home") => Some("Go home"),
        (Locale::Fa, "common.go_home") => Some("رفتن به خانه"),
        (Locale::En, "common.go_back") => Some("Go back"),
        (Locale::Fa, "common.go_back") => Some("بازگشت"),

        (Locale::En, "navigation.close") => Some("Close navigation"),
        (Locale::Fa, "navigation.close") => Some("بستن ناوبری"),
        (Locale::En, "navigation.open") => Some("Open navigation"),
        (Locale::Fa, "navigation.open") => Some("باز کردن ناوبری"),
        (Locale::En, "navigation.primary") => Some("Primary navigation"),
        (Locale::Fa, "navigation.primary") => Some("ناوبری اصلی"),
        (Locale::En, "navigation.redirecting_dashboard") => Some("Opening your dashboard."),
        (Locale::Fa, "navigation.redirecting_dashboard") => Some("در حال باز کردن داشبورد شما."),

        (Locale::En, "roles.teacher") => Some("Teacher"),
        (Locale::Fa, "roles.teacher") => Some("معلم"),
        (Locale::En, "roles.student") => Some("Student"),
        (Locale::Fa, "roles.student") => Some("دانش‌آموز"),
        (Locale::En, "roles.parent") => Some("Parent"),
        (Locale::Fa, "roles.parent") => Some("والد"),
        (Locale::En, "roles.school_manager") => Some("School Manager"),
        (Locale::Fa, "roles.school_manager") => Some("مدیر مدرسه"),
        (Locale::En, "roles.platform_administrator") => Some("Platform Administrator"),
        (Locale::Fa, "roles.platform_administrator") => Some("مدیر سامانه"),

        (Locale::En, "nav.knowledge_assets") => Some("Knowledge assets"),
        (Locale::Fa, "nav.knowledge_assets") => Some("منابع دانشی"),
        (Locale::En, "nav.knowledge_audit") => Some("Knowledge audit"),
        (Locale::Fa, "nav.knowledge_audit") => Some("ممیزی دانش"),
        (Locale::En, "nav.knowledge_submissions") => Some("Knowledge submissions"),
        (Locale::Fa, "nav.knowledge_submissions") => Some("ارسال منابع دانشی"),

        (Locale::En, "errors.destination_unavailable") => {
            Some("This destination is not available for your current role or enabled product capabilities.")
        }
        (Locale::Fa, "errors.destination_unavailable") => {
            Some("این بخش برای نقش فعلی شما یا قابلیت‌های فعال سامانه در دسترس نیست.")
        }
        (Locale::En, "errors.generic_title") => Some("Something went wrong"),
        (Locale::Fa, "errors.generic_title") => Some("مشکلی پیش آمد"),
        (Locale::En, "errors.generic_description") => {
            Some("The page could not complete the requested operation. Try again.")
        }
        (Locale::Fa, "errors.generic_description") => {
            Some("صفحه نتوانست عملیات درخواستی را کامل کند. دوباره تلاش کنید.")
        }
        (Locale::En, "errors.network_title") => Some("Connection problem"),
        (Locale::Fa, "errors.network_title") => Some("مشکل اتصال"),
        (Locale::En, "errors.network_description") => {
            Some("The server could not be reached. Check the connection and try again.")
        }
        (Locale::Fa, "errors.network_description") => {
            Some("ارتباط با سرور برقرار نشد. اتصال را بررسی کنید و دوباره تلاش کنید.")
        }
        (Locale::En, "errors.not_found_title") => Some("Page not found"),
        (Locale::Fa, "errors.not_found_title") => Some("صفحه پیدا نشد"),
        (Locale::En, "errors.not_found_description") => {
            Some("This destination does not exist or is no longer available.")
        }
        (Locale::Fa, "errors.not_found_description") => {
            Some("این بخش وجود ندارد یا دیگر در دسترس نیست.")
        }

        (Locale::En, "session.checking") => Some("Checking your session and access."),
        (Locale::Fa, "session.checking") => Some("در حال بررسی نشست و سطح دسترسی شما."),
        (Locale::En, "session.unavailable_title") => Some("Unable to load your account"),
        (Locale::Fa, "session.unavailable_title") => Some("بارگذاری حساب شما ممکن نیست"),
        (Locale::En, "session.unavailable_description") => {
            Some("Refresh the page or sign in again if the problem continues.")
        }
        (Locale::Fa, "session.unavailable_description") => {
            Some("صفحه را تازه‌سازی کنید و اگر مشکل ادامه داشت دوباره وارد شوید.")
        }
        (Locale::En, "session.sign_in_required") => Some("Sign in required"),
        (Locale::Fa, "session.sign_in_required") => Some("ورود لازم است"),
        (Locale::En, "session.sign_in_required_description") => {
            Some("Sign in to continue to the dashboard.")
        }
        (Locale::Fa, "session.sign_in_required_description") => {
            Some("برای ادامه به داشبورد وارد شوید.")
        }

        (Locale::En, "notifications.loading") => Some("Loading notifications…"),
        (Locale::Fa, "notifications.loading") => Some("در حال بارگذاری اعلان‌ها…"),
        (Locale::En, "notifications.failed_load") => Some("Notifications could not be loaded."),
        (Locale::Fa, "notifications.failed_load") => Some("بارگذاری اعلان‌ها ناموفق بود."),
        (Locale::En, "notifications.action_failed") => {
            Some("The notification update could not be saved. Try again.")
        }
        (Locale::Fa, "notifications.action_failed") => {
            Some("به‌روزرسانی اعلان ذخیره نشد. دوباره تلاش کنید.")
        }

        (Locale::En, "validation.valid") => Some("Valid"),
        (Locale::Fa, "validation.valid") => Some("معتبر"),
        (Locale::En, "validation.all_valid") => Some("All fields are valid."),
        (Locale::Fa, "validation.all_valid") => Some("همه فیلدها معتبر هستند."),
        (Locale::En, "validation.fix_errors") => Some("Review the highlighted fields"),
        (Locale::Fa, "validation.fix_errors") => Some("فیلدهای مشخص‌شده را بررسی کنید"),

        (Locale::En, "auth.context_headline") => Some("A focused workspace for every school role."),
        (Locale::Fa, "auth.context_headline") => Some("فضای کاری متمرکز برای هر نقش مدرسه."),
        (Locale::En, "auth.context_copy") => Some("Sign in to the same private school workspace used for classes, assignments, grading and governed knowledge."),
        (Locale::Fa, "auth.context_copy") => Some("برای دسترسی به همان فضای خصوصی مدرسه برای کلاس‌ها، تکلیف‌ها، ارزیابی و دانش کنترل‌شده وارد شوید."),
        (Locale::En, "auth.offline_note") => Some("Designed for the school's private deployment."),
        (Locale::Fa, "auth.offline_note") => Some("طراحی‌شده برای استقرار خصوصی مدرسه."),
        (Locale::En, "auth.reveal_password") => Some("Show password"),
        (Locale::Fa, "auth.reveal_password") => Some("نمایش رمز عبور"),
        (Locale::En, "auth.hide_password") => Some("Hide password"),
        (Locale::Fa, "auth.hide_password") => Some("پنهان کردن رمز عبور"),
        (Locale::En, "auth.email_required") => Some("Email is required."),
        (Locale::Fa, "auth.email_required") => Some("ایمیل الزامی است."),
        (Locale::En, "auth.email_invalid") => Some("Enter a valid email address."),
        (Locale::Fa, "auth.email_invalid") => Some("یک نشانی ایمیل معتبر وارد کنید."),
        (Locale::En, "auth.password_required") => Some("Password is required."),
        (Locale::Fa, "auth.password_required") => Some("رمز عبور الزامی است."),
        (Locale::En, "auth.account_inactive") => Some("This account is inactive. Contact your administrator."),
        (Locale::Fa, "auth.account_inactive") => Some("این حساب غیرفعال است. با مدیر سامانه تماس بگیرید."),
        (Locale::En, "auth.account_locked") => Some("This account is locked. Contact your administrator."),
        (Locale::Fa, "auth.account_locked") => Some("این حساب قفل شده است. با مدیر سامانه تماس بگیرید."),
        (Locale::En, "auth.email_not_confirmed") => Some("This account is not ready for sign-in. Contact your administrator."),
        (Locale::Fa, "auth.email_not_confirmed") => Some("این حساب هنوز برای ورود آماده نیست. با مدیر سامانه تماس بگیرید."),
        (Locale::En, "auth.account_requires_admin") => Some("This account requires administrator assistance before sign-in."),
        (Locale::Fa, "auth.account_requires_admin") => Some("این حساب پیش از ورود به راهنمایی مدیر سامانه نیاز دارد."),
        (Locale::En, "auth.service_unavailable") => Some("Sign-in is temporarily unavailable. Try again shortly."),
        (Locale::Fa, "auth.service_unavailable") => Some("ورود موقتاً در دسترس نیست. کمی بعد دوباره تلاش کنید."),
        (Locale::En, "auth.recovery_unavailable_title") => Some("Password recovery is unavailable"),
        (Locale::Fa, "auth.recovery_unavailable_title") => Some("بازیابی رمز عبور در دسترس نیست"),
        (Locale::En, "auth.recovery_unavailable_description") => Some("Email password reset is not enabled in this release. Contact your administrator for assistance."),
        (Locale::Fa, "auth.recovery_unavailable_description") => Some("بازنشانی رمز عبور از طریق ایمیل در این نسخه فعال نیست. برای راهنمایی با مدیر سامانه تماس بگیرید."),

        (Locale::En, "submissions.review_description") => {
            Some("Review and grade student submissions")
        }
        (Locale::En, "submissions.pending_filter") => Some("Pending"),
        (Locale::En, "submissions.all_filter") => Some("All"),
        (Locale::En, "submissions.failed_load") => Some("Failed to load submissions: "),
        (Locale::En, "submissions.caught_up_title") => Some("All Caught Up!"),
        (Locale::En, "submissions.caught_up_desc") => Some("No pending submissions to grade"),
        (Locale::En, "submissions.update_grade") => Some("Update Grade"),
        (Locale::En, "submissions.grade_btn") => Some("Grade Submission"),
        (Locale::En, "submissions.grade_modal_title") => Some("Grade Submission"),
        (Locale::En, "submissions.validation_range") => Some("Please enter a valid numeric grade"),
        (Locale::En, "submissions.save_failed") => Some("Failed to save grade. Please try again."),
        (Locale::En, "submissions.student_work_label") => Some("Student's Work"),
        (Locale::En, "submissions.grade_range_label") => Some("Grade (0-100)"),
        (Locale::En, "submissions.feedback_placeholder") => {
            Some("Great work! Consider improving...")
        }
        (Locale::En, "submissions.saving_btn") => Some("Saving..."),
        (Locale::En, "submissions.save_btn") => Some("Save Grade"),

        // Persian uses a 0–20 display scale. Override the legacy 0–100 copy so
        // validation and labels agree with Locale::max_grade().
        (Locale::Fa, "submissions.validation_range") => Some("لطفاً یک نمره عددی معتبر وارد کنید"),
        (Locale::Fa, "submissions.grade_range_label") => Some("نمره (۰-۲۰)"),
        (Locale::Fa, "submissions.grade_label") => Some("نمره"),

        (Locale::En, "teachers.students.description") => Some("View student progress and contact details."),
        (Locale::Fa, "teachers.students.description") => Some("پیشرفت و اطلاعات تماس دانش‌آموزان را مشاهده کنید."),
        (Locale::En, "teachers.students.search_placeholder") => Some("Search students"),
        (Locale::Fa, "teachers.students.search_placeholder") => Some("جستجوی دانش‌آموزان"),
        (Locale::En, "teachers.students.load_error") => Some("Students could not be loaded. Please try again."),
        (Locale::Fa, "teachers.students.load_error") => Some("بارگذاری دانش‌آموزان ممکن نیست. دوباره تلاش کنید."),
        (Locale::En, "teachers.students.no_students_found") => Some("No students found"),
        (Locale::Fa, "teachers.students.no_students_found") => Some("دانش‌آموزی پیدا نشد"),
        (Locale::En, "teachers.students.no_students_desc") => Some("Students will appear here when they are enrolled in your classes."),
        (Locale::Fa, "teachers.students.no_students_desc") => Some("وقتی دانش‌آموزان در کلاس‌های شما ثبت‌نام شوند، اینجا نمایش داده می‌شوند."),
        (Locale::En, "teachers.students.submitted_label") => Some("submitted"),
        (Locale::Fa, "teachers.students.submitted_label") => Some("ارسال‌شده"),
        (Locale::En, "teachers.students.profile_btn") => Some("View profile"),
        (Locale::Fa, "teachers.students.profile_btn") => Some("مشاهده نمایه"),
        (Locale::En, "teachers.students.grades_btn") => Some("View grades"),
        (Locale::Fa, "teachers.students.grades_btn") => Some("مشاهده نمره‌ها"),
        (Locale::En, "teachers.students.profile_title_suffix") => Some("'s profile"),
        (Locale::Fa, "teachers.students.profile_title_suffix") => Some(" — نمایه"),
        (Locale::En, "teachers.students.grades_title_suffix") => Some("'s grades"),
        (Locale::Fa, "teachers.students.grades_title_suffix") => Some(" — نمره‌ها"),
        (Locale::En, "teachers.students.average_label") => Some("Average"),
        (Locale::Fa, "teachers.students.average_label") => Some("میانگین"),
        (Locale::En, "teachers.students.submitted_stat") => Some("Submitted"),
        (Locale::Fa, "teachers.students.submitted_stat") => Some("ارسال‌شده"),
        (Locale::En, "teachers.students.classes_stat") => Some("Classes"),
        (Locale::Fa, "teachers.students.classes_stat") => Some("کلاس‌ها"),
        (Locale::En, "teachers.students.enrolled_classes") => Some("Enrolled classes"),
        (Locale::Fa, "teachers.students.enrolled_classes") => Some("کلاس‌های ثبت‌نام‌شده"),
        (Locale::En, "teachers.students.average_grade") => Some("Average grade"),
        (Locale::Fa, "teachers.students.average_grade") => Some("میانگین نمره"),
        (Locale::En, "teachers.students.loading_grades") => Some("Loading grades…"),
        (Locale::Fa, "teachers.students.loading_grades") => Some("در حال بارگذاری نمره‌ها…"),
        (Locale::En, "teachers.students.grades_failed") => Some("Grades could not be loaded. Please try again."),
        (Locale::Fa, "teachers.students.grades_failed") => Some("بارگذاری نمره‌ها ممکن نیست. دوباره تلاش کنید."),
        (Locale::En, "teachers.students.no_grades") => Some("No grades are available yet."),
        (Locale::Fa, "teachers.students.no_grades") => Some("هنوز نمره‌ای در دسترس نیست."),

        (Locale::En, "teacher.knowledge_assets.title") => Some("Knowledge assets"),
        (Locale::Fa, "teacher.knowledge_assets.title") => Some("منابع دانشی"),
        (Locale::En, "teacher.knowledge_assets.description") => {
            Some("Choose approved school sources to use across your classes.")
        }
        (Locale::Fa, "teacher.knowledge_assets.description") => {
            Some("منابع تأییدشده مدرسه را برای استفاده در کلاس‌های خود انتخاب کنید.")
        }
        (Locale::En, "teacher.knowledge_assets.loading") => Some("Loading knowledge assets..."),
        (Locale::Fa, "teacher.knowledge_assets.loading") => Some("در حال بارگذاری منابع دانشی..."),
        (Locale::En, "teacher.knowledge_assets.load_error") => {
            Some("Knowledge assets could not be loaded. Please try again.")
        }
        (Locale::Fa, "teacher.knowledge_assets.load_error") => {
            Some("بارگذاری منابع دانشی ممکن نیست. دوباره تلاش کنید.")
        }
        (Locale::En, "teacher.knowledge_assets.empty") => {
            Some("No knowledge assets are available for your school.")
        }
        (Locale::Fa, "teacher.knowledge_assets.empty") => {
            Some("هیچ منبع دانشی برای مدرسه شما در دسترس نیست.")
        }
        (Locale::En, "teacher.knowledge_assets.school_approved") => Some("Approved school source"),
        (Locale::Fa, "teacher.knowledge_assets.school_approved") => Some("منبع تأییدشده مدرسه"),
        (Locale::En, "teacher.knowledge_assets.enabled") => Some("Enabled"),
        (Locale::Fa, "teacher.knowledge_assets.enabled") => Some("فعال"),
        (Locale::En, "teacher.knowledge_assets.available") => Some("Available"),
        (Locale::Fa, "teacher.knowledge_assets.available") => Some("در دسترس"),
        (Locale::En, "teacher.knowledge_assets.disabled") => Some("Disabled"),
        (Locale::Fa, "teacher.knowledge_assets.disabled") => Some("غیرفعال"),
        (Locale::En, "teacher.knowledge_assets.enable_action") => Some("Enable for generation"),
        (Locale::Fa, "teacher.knowledge_assets.enable_action") => Some("فعال‌سازی برای تولید"),
        (Locale::En, "teacher.knowledge_assets.disable_action") => Some("Disable for generation"),
        (Locale::Fa, "teacher.knowledge_assets.disable_action") => {
            Some("غیرفعال‌سازی برای تولید")
        }
        (Locale::En, "teacher.knowledge_assets.update_error") => {
            Some("Update failed. Please try again.")
        }
        (Locale::Fa, "teacher.knowledge_assets.update_error") => {
            Some("به‌روزرسانی ناموفق بود. دوباره تلاش کنید.")
        }

        _ => None,
    }
}

/// Product role labels are locale-aware presentation, never domain display text.
pub fn role_label(role: SystemRole, locale: Locale) -> &'static str {
    let key = match role {
        SystemRole::Teacher => "roles.teacher",
        SystemRole::Student => "roles.student",
        SystemRole::Parent => "roles.parent",
        SystemRole::SchoolManager => "roles.school_manager",
        SystemRole::PlatformAdmin => "roles.platform_administrator",
    };
    supplemental_translation(key, locale).unwrap_or(role.display_name())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pr2_shared_copy_is_localized_in_both_languages() {
        for key in [
            "common.search",
            "common.clear",
            "common.data",
            "common.go_home",
            "common.go_back",
            "navigation.close",
            "navigation.open",
            "navigation.primary",
            "navigation.redirecting_dashboard",
            "roles.teacher",
            "roles.student",
            "roles.parent",
            "roles.school_manager",
            "roles.platform_administrator",
            "teachers.students.description",
            "teachers.students.search_placeholder",
            "teachers.students.load_error",
            "teachers.students.no_students_found",
            "teachers.students.no_students_desc",
            "teachers.students.submitted_label",
            "teachers.students.profile_btn",
            "teachers.students.grades_btn",
            "teachers.students.profile_title_suffix",
            "teachers.students.grades_title_suffix",
            "teachers.students.average_label",
            "teachers.students.submitted_stat",
            "teachers.students.classes_stat",
            "teachers.students.enrolled_classes",
            "teachers.students.average_grade",
            "teachers.students.loading_grades",
            "teachers.students.grades_failed",
            "teachers.students.no_grades",
            "nav.knowledge_assets",
            "nav.knowledge_audit",
            "nav.knowledge_submissions",
            "errors.destination_unavailable",
            "errors.generic_title",
            "errors.generic_description",
            "errors.network_title",
            "errors.network_description",
            "errors.not_found_title",
            "errors.not_found_description",
            "session.checking",
            "session.unavailable_title",
            "session.unavailable_description",
            "session.sign_in_required",
            "session.sign_in_required_description",
            "notifications.loading",
            "notifications.failed_load",
            "notifications.action_failed",
            "validation.valid",
            "validation.all_valid",
            "validation.fix_errors",
            "auth.context_headline",
            "auth.context_copy",
            "auth.offline_note",
            "auth.reveal_password",
            "auth.hide_password",
            "auth.email_required",
            "auth.email_invalid",
            "auth.password_required",
            "auth.account_inactive",
            "auth.account_locked",
            "auth.email_not_confirmed",
            "auth.account_requires_admin",
            "auth.service_unavailable",
            "auth.recovery_unavailable_title",
            "auth.recovery_unavailable_description",
            "teacher.knowledge_assets.title",
            "teacher.knowledge_assets.description",
            "teacher.knowledge_assets.loading",
            "teacher.knowledge_assets.load_error",
            "teacher.knowledge_assets.empty",
            "teacher.knowledge_assets.school_approved",
            "teacher.knowledge_assets.enabled",
            "teacher.knowledge_assets.available",
            "teacher.knowledge_assets.disabled",
            "teacher.knowledge_assets.enable_action",
            "teacher.knowledge_assets.disable_action",
            "teacher.knowledge_assets.update_error",
        ] {
            assert!(
                supplemental_translation(key, Locale::En).is_some(),
                "missing English {key}"
            );
            assert!(
                supplemental_translation(key, Locale::Fa).is_some(),
                "missing Farsi {key}"
            );
        }
    }

    #[test]
    fn roles_are_presented_in_the_active_locale() {
        assert_eq!(role_label(SystemRole::Teacher, Locale::En), "Teacher");
        assert_eq!(role_label(SystemRole::Teacher, Locale::Fa), "معلم");
        assert_eq!(
            role_label(SystemRole::PlatformAdmin, Locale::Fa),
            "مدیر سامانه"
        );
    }

    #[test]
    fn teacher_submission_copy_is_localized_in_english() {
        for key in [
            "common.refresh",
            "submissions.review_description",
            "submissions.pending_filter",
            "submissions.all_filter",
            "submissions.grade_btn",
            "submissions.grade_modal_title",
            "submissions.save_btn",
        ] {
            assert_ne!(
                supplemental_translation(key, Locale::En),
                None,
                "missing {key}"
            );
        }
    }

    #[test]
    fn persian_grading_copy_uses_the_twenty_point_scale() {
        assert_eq!(
            supplemental_translation("submissions.grade_range_label", Locale::Fa),
            Some("نمره (۰-۲۰)")
        );
    }
}
