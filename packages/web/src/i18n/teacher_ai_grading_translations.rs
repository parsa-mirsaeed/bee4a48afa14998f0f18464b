use super::Locale;

/// Teacher review of AI grading, always bilingual and never student-visible
/// until the assigned Teacher explicitly publishes an accepted suggestion.
pub(crate) fn teacher_ai_grading_translation(
    key: &'static str,
    locale: Locale,
) -> Option<&'static str> {
    match (locale, key) {
        (Locale::En, "ai_grading.heading") => Some("AI grading assistant"),
        (Locale::Fa, "ai_grading.heading") => Some("دستیار تصحیح هوشمند"),
        (Locale::En, "ai_grading.description") => Some("AI suggestions are private until you accept and publish a grade. Text submissions only."),
        (Locale::Fa, "ai_grading.description") => Some("نمره پیشنهادی هوش مصنوعی تا زمان تأیید شما خصوصی است. فعلاً فقط پاسخ متنی پشتیبانی می‌شود."),
        (Locale::En, "ai_grading.rubric_label") => Some("Teacher rubric: one criterion per line (name | points)"),
        (Locale::Fa, "ai_grading.rubric_label") => Some("معیارهای معلم: هر سطر «عنوان | امتیاز»"),
        (Locale::En, "ai_grading.rubric_placeholder") => Some("Reasoning | 12\nAccuracy | 8"),
        (Locale::Fa, "ai_grading.rubric_placeholder") => Some("استدلال | ۱۲\nدقت | ۸"),
        (Locale::En, "ai_grading.approve") => Some("Approve rubric"),
        (Locale::Fa, "ai_grading.approve") => Some("تأیید معیارها"),
        (Locale::En, "ai_grading.generate") => Some("Generate AI suggestion"),
        (Locale::Fa, "ai_grading.generate") => Some("تولید پیشنهاد نمره"),
        (Locale::En, "ai_grading.suggestion") => Some("AI suggested grade"),
        (Locale::Fa, "ai_grading.suggestion") => Some("نمره پیشنهادی هوش مصنوعی"),
        (Locale::En, "ai_grading.evidence") => Some("Criterion evidence"),
        (Locale::Fa, "ai_grading.evidence") => Some("شواهد هر معیار"),
        (Locale::En, "ai_grading.feedback") => Some("Suggested feedback"),
        (Locale::Fa, "ai_grading.feedback") => Some("بازخورد پیشنهادی"),
        (Locale::En, "ai_grading.accept") => Some("Accept & publish grade"),
        (Locale::Fa, "ai_grading.accept") => Some("تأیید و انتشار نمره"),
        (Locale::En, "ai_grading.reject") => Some("Reject suggestion"),
        (Locale::Fa, "ai_grading.reject") => Some("رد پیشنهاد"),
        (Locale::En, "ai_grading.busy") => Some("Processing…"),
        (Locale::Fa, "ai_grading.busy") => Some("در حال پردازش…"),
        (Locale::En, "ai_grading.invalid_rubric") => Some("Use 1–20 distinct criteria. Their positive points must total the assignment grade scale."),
        (Locale::Fa, "ai_grading.invalid_rubric") => Some("بین ۱ تا ۲۰ معیار متفاوت وارد کنید؛ مجموع امتیازهای مثبت باید برابر مقیاس نمره تکلیف باشد."),
        (Locale::En, "ai_grading.failed") => Some("The AI grading request could not be completed. Review the rubric, submission and access."),
        (Locale::Fa, "ai_grading.failed") => Some("عملیات تصحیح هوشمند انجام نشد. معیارها، تکلیف و سطح دسترسی را بررسی کنید."),
        (Locale::En, "ai_grading.rejected") => Some("Suggestion rejected; the official grade remains unchanged."),
        (Locale::Fa, "ai_grading.rejected") => Some("پیشنهاد رد شد؛ نمره رسمی تغییری نکرد."),
        (Locale::En, "ai_grading.load_failed") => Some("The private grading evidence could not be loaded."),
        (Locale::Fa, "ai_grading.load_failed") => Some("بارگذاری شواهد خصوصی تصحیح ممکن نشد."),
        (Locale::En, "ai_grading.pending") => Some("Rubric approved. Generate or wait for a queued suggestion."),
        (Locale::Fa, "ai_grading.pending") => Some("معیارها تأیید شدند. پیشنهاد را تولید کنید یا منتظر صف خودکار بمانید."),
        (Locale::En, "ai_grading.confidence") => Some("Model confidence (not a guarantee)"),
        (Locale::Fa, "ai_grading.confidence") => Some("اطمینان مدل (بدون تضمین صحت)"),
        _ => None,
    }
}
