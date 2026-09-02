use super::Locale;

pub(crate) fn platform_admin_translation(
    key: &'static str,
    locale: Locale,
) -> Option<&'static str> {
    match (locale, key) {
        (Locale::En, "admin.loading") => Some("Loading administration workspace…"),
        (Locale::Fa, "admin.loading") => Some("در حال بارگذاری فضای مدیریت سامانه…"),
        (Locale::En, "admin.review.title") => Some("Governed knowledge review"),
        (Locale::Fa, "admin.review.title") => Some("بازبینی دانش کنترل‌شده"),
        (Locale::En, "admin.review.description") => Some("Review the private source, verify OCR, embed, then publish explicitly. The next allowed lifecycle action is shown for each asset."),
        (Locale::Fa, "admin.review.description") => Some("منبع خصوصی را بازبینی کنید، متن OCR را تأیید کنید، بردارسازی را انجام دهید و سپس منبع را صریحاً منتشر کنید. اقدام مجاز بعدی برای هر منبع نمایش داده می‌شود."),
        (Locale::En, "admin.review.step1.title") => Some("Review & verify OCR"),
        (Locale::Fa, "admin.review.step1.title") => Some("بازبینی و تأیید OCR"),
        (Locale::En, "admin.review.step1.detail") => Some("Inspect the private PDF and save only verified text."),
        (Locale::Fa, "admin.review.step1.detail") => Some("PDF خصوصی را بررسی کنید و فقط متن تأییدشده را ذخیره کنید."),
        (Locale::En, "admin.review.step2.title") => Some("Embed"),
        (Locale::Fa, "admin.review.step2.title") => Some("بردارسازی"),
        (Locale::En, "admin.review.step2.detail") => Some("Queue embedding only after verified OCR exists."),
        (Locale::Fa, "admin.review.step2.detail") => Some("بردارسازی را فقط پس از وجود OCR تأییدشده در صف قرار دهید."),
        (Locale::En, "admin.review.step3.title") => Some("Publish"),
        (Locale::Fa, "admin.review.step3.title") => Some("انتشار"),
        (Locale::En, "admin.review.step3.detail") => Some("Publish explicitly only after embedding completes."),
        (Locale::Fa, "admin.review.step3.detail") => Some("فقط پس از تکمیل بردارسازی، منبع را صریحاً منتشر کنید."),
        (Locale::En, "admin.review.loading") => Some("Loading governed assets…"),
        (Locale::Fa, "admin.review.loading") => Some("در حال بارگذاری منابع کنترل‌شده…"),
        (Locale::En, "admin.review.error") => Some("Governed assets could not be loaded. Refresh and try again."),
        (Locale::Fa, "admin.review.error") => Some("بارگذاری منابع کنترل‌شده ممکن نشد. صفحه را تازه‌سازی و دوباره تلاش کنید."),
        (Locale::En, "admin.review.empty") => Some("No manager submissions are waiting."),
        (Locale::Fa, "admin.review.empty") => Some("ارسال جدیدی از مدیران در انتظار بازبینی نیست."),

        (Locale::En, "admin.status.submitted") => Some("Submitted"),
        (Locale::Fa, "admin.status.submitted") => Some("ارسال‌شده"),
        (Locale::En, "admin.status.ocr_pending") => Some("OCR review pending"),
        (Locale::Fa, "admin.status.ocr_pending") => Some("در انتظار بازبینی OCR"),
        (Locale::En, "admin.status.ocr_ready") => Some("OCR verified"),
        (Locale::Fa, "admin.status.ocr_ready") => Some("OCR تأییدشده"),
        (Locale::En, "admin.status.embedding_pending") => Some("Embedding in progress"),
        (Locale::Fa, "admin.status.embedding_pending") => Some("بردارسازی در حال انجام"),
        (Locale::En, "admin.status.embedded") => Some("Embedding complete"),
        (Locale::Fa, "admin.status.embedded") => Some("بردارسازی تکمیل‌شده"),
        (Locale::En, "admin.status.published") => Some("Published"),
        (Locale::Fa, "admin.status.published") => Some("منتشرشده"),
        (Locale::En, "admin.status.archived") => Some("Archived"),
        (Locale::Fa, "admin.status.archived") => Some("بایگانی‌شده"),
        (Locale::En, "admin.status.failed") => Some("Needs attention"),
        (Locale::Fa, "admin.status.failed") => Some("نیازمند بررسی"),
        (Locale::En, "admin.status.unknown") => Some("Status unavailable"),
        (Locale::Fa, "admin.status.unknown") => Some("وضعیت در دسترس نیست"),

        (Locale::En, "admin.school.label") => Some("School"),
        (Locale::Fa, "admin.school.label") => Some("مدرسه"),
        (Locale::En, "admin.school.unavailable") => Some("School name unavailable"),
        (Locale::Fa, "admin.school.unavailable") => Some("نام مدرسه در دسترس نیست"),
        (Locale::En, "admin.metadata.subject") => Some("Subject"),
        (Locale::Fa, "admin.metadata.subject") => Some("موضوع"),
        (Locale::En, "admin.metadata.grade") => Some("Grade"),
        (Locale::Fa, "admin.metadata.grade") => Some("پایه"),
        (Locale::En, "admin.metadata.language") => Some("Language"),
        (Locale::Fa, "admin.metadata.language") => Some("زبان"),
        (Locale::En, "admin.language.en") => Some("English"),
        (Locale::Fa, "admin.language.en") => Some("انگلیسی"),
        (Locale::En, "admin.language.fa") => Some("Persian"),
        (Locale::Fa, "admin.language.fa") => Some("فارسی"),

        (Locale::En, "admin.source.title") => Some("Source document"),
        (Locale::Fa, "admin.source.title") => Some("سند منبع"),
        (Locale::En, "admin.source.review") => Some("Review private PDF"),
        (Locale::Fa, "admin.source.review") => Some("بازبینی PDF خصوصی"),
        (Locale::En, "admin.source.unavailable") => Some("Private source review is unavailable for this legacy submission."),
        (Locale::Fa, "admin.source.unavailable") => Some("بازبینی منبع خصوصی برای این ارسال قدیمی در دسترس نیست."),
        (Locale::En, "admin.source.metadata_unavailable") => Some("Source metadata unavailable"),
        (Locale::Fa, "admin.source.metadata_unavailable") => Some("فراداده منبع در دسترس نیست"),

        (Locale::En, "admin.ocr.close") => Some("Close OCR editor"),
        (Locale::Fa, "admin.ocr.close") => Some("بستن ویرایشگر OCR"),
        (Locale::En, "admin.ocr.confirm_source") => Some("Confirm the text against the private source PDF before saving. Saving verified OCR does not publish the asset."),
        (Locale::Fa, "admin.ocr.confirm_source") => Some("پیش از ذخیره، متن را با PDF خصوصی منبع تطبیق دهید. ذخیره OCR تأییدشده باعث انتشار منبع نمی‌شود."),
        (Locale::En, "admin.ocr.loading") => Some("Loading the current verified OCR…"),
        (Locale::Fa, "admin.ocr.loading") => Some("در حال بارگذاری OCR تأییدشده فعلی…"),
        (Locale::En, "admin.ocr.source_load_error") => Some("The current governed source revision could not be loaded. Refresh and review the source again."),
        (Locale::Fa, "admin.ocr.source_load_error") => Some("نسخه فعلی منبع کنترل‌شده بارگذاری نشد. صفحه را تازه‌سازی و منبع را دوباره بازبینی کنید."),
        (Locale::En, "admin.ocr.load_error") => Some("The current verified OCR could not be loaded. Refresh and try again."),
        (Locale::Fa, "admin.ocr.load_error") => Some("OCR تأییدشده فعلی بارگذاری نشد. صفحه را تازه‌سازی و دوباره تلاش کنید."),
        (Locale::En, "admin.ocr.source_unavailable") => Some("The governed source revision is unavailable. Refresh and review the source again."),
        (Locale::Fa, "admin.ocr.source_unavailable") => Some("نسخه منبع کنترل‌شده در دسترس نیست. صفحه را تازه‌سازی و منبع را دوباره بازبینی کنید."),
        (Locale::En, "admin.ocr.save_error") => Some("OCR verification could not be saved because the source or OCR revision changed, the current source has not been reviewed, or the asset is no longer eligible. Refresh and review it again."),
        (Locale::Fa, "admin.ocr.save_error") => Some("تأیید OCR ذخیره نشد؛ ممکن است نسخه منبع یا OCR تغییر کرده باشد، منبع فعلی بازبینی نشده باشد یا منبع دیگر واجد شرایط نباشد. صفحه را تازه‌سازی و دوباره بازبینی کنید."),
        (Locale::En, "admin.ocr.save_success") => Some("Verified OCR saved. The asset is ready for embedding."),
        (Locale::Fa, "admin.ocr.save_success") => Some("OCR تأییدشده ذخیره شد. منبع آماده بردارسازی است."),
        (Locale::En, "admin.ocr.source_revision") => Some("Governed source revision"),
        (Locale::Fa, "admin.ocr.source_revision") => Some("نسخه منبع کنترل‌شده"),
        (Locale::En, "admin.ocr.source_hash") => Some("Source hash"),
        (Locale::Fa, "admin.ocr.source_hash") => Some("هش منبع"),
        (Locale::En, "admin.ocr.current_revision") => Some("Current verified revision"),
        (Locale::Fa, "admin.ocr.current_revision") => Some("نسخه تأییدشده فعلی"),
        (Locale::En, "admin.ocr.verified_at") => Some("Verified at"),
        (Locale::Fa, "admin.ocr.verified_at") => Some("زمان تأیید"),
        (Locale::En, "admin.ocr.verified_by") => Some("Verified by"),
        (Locale::Fa, "admin.ocr.verified_by") => Some("تأییدکننده"),
        (Locale::En, "admin.ocr.text_hash") => Some("Text hash"),
        (Locale::Fa, "admin.ocr.text_hash") => Some("هش متن"),
        (Locale::En, "admin.ocr.provider") => Some("OCR provider / verification process"),
        (Locale::Fa, "admin.ocr.provider") => Some("ارائه‌دهنده OCR / فرایند تأیید"),
        (Locale::En, "admin.ocr.text") => Some("Verified source text"),
        (Locale::Fa, "admin.ocr.text") => Some("متن تأییدشده منبع"),
        (Locale::En, "admin.ocr.placeholder") => Some("Paste text that has been checked against the source PDF…"),
        (Locale::Fa, "admin.ocr.placeholder") => Some("متنی را که با PDF منبع بررسی شده است وارد کنید…"),
        (Locale::En, "admin.ocr.reload") => Some("Reload current OCR and replace draft"),
        (Locale::Fa, "admin.ocr.reload") => Some("بارگذاری مجدد OCR فعلی و جایگزینی پیش‌نویس"),
        (Locale::En, "admin.ocr.cancel") => Some("Cancel"),
        (Locale::Fa, "admin.ocr.cancel") => Some("لغو"),
        (Locale::En, "admin.ocr.saving") => Some("Saving…"),
        (Locale::Fa, "admin.ocr.saving") => Some("در حال ذخیره…"),
        (Locale::En, "admin.ocr.save") => Some("Save verified OCR"),
        (Locale::Fa, "admin.ocr.save") => Some("ذخیره OCR تأییدشده"),
        (Locale::En, "admin.ocr.save_changes") => Some("Save verified OCR changes"),
        (Locale::Fa, "admin.ocr.save_changes") => Some("ذخیره تغییرات OCR تأییدشده"),
        (Locale::En, "admin.ocr.attach") => Some("Attach verified OCR"),
        (Locale::Fa, "admin.ocr.attach") => Some("افزودن OCR تأییدشده"),
        (Locale::En, "admin.ocr.update") => Some("Update verified OCR"),
        (Locale::Fa, "admin.ocr.update") => Some("به‌روزرسانی OCR تأییدشده"),

        (Locale::En, "admin.discard.unsaved") => Some("Discard unsaved OCR changes?"),
        (Locale::Fa, "admin.discard.unsaved") => Some("تغییرات ذخیره‌نشده OCR کنار گذاشته شود؟"),
        (Locale::En, "admin.discard.description") => Some("Your unsaved OCR text or verification method will be lost."),
        (Locale::Fa, "admin.discard.description") => Some("متن OCR یا روش تأیید ذخیره‌نشده از بین خواهد رفت."),
        (Locale::En, "admin.discard.confirm") => Some("Discard changes"),
        (Locale::Fa, "admin.discard.confirm") => Some("کنار گذاشتن تغییرات"),
        (Locale::En, "admin.discard.keep") => Some("Keep editing"),
        (Locale::Fa, "admin.discard.keep") => Some("ادامه ویرایش"),

        (Locale::En, "admin.archive.confirm") => Some("Archive asset"),
        (Locale::Fa, "admin.archive.confirm") => Some("بایگانی منبع"),
        (Locale::En, "admin.archive.cancel") => Some("Cancel"),
        (Locale::Fa, "admin.archive.cancel") => Some("لغو"),
        (Locale::En, "admin.archive.published_description") => Some("This withdraws the asset from teacher retrieval and cancels active ingestion work."),
        (Locale::Fa, "admin.archive.published_description") => Some("این کار منبع را از بازیابی معلمان خارج می‌کند و پردازش فعال را لغو می‌کند."),
        (Locale::En, "admin.archive.description") => Some("This archives the asset and cancels active ingestion work. Archived assets are terminal."),
        (Locale::Fa, "admin.archive.description") => Some("این کار منبع را بایگانی و پردازش فعال را لغو می‌کند. منابع بایگانی‌شده نهایی هستند."),
        (Locale::En, "admin.archive.archiving") => Some("Archiving asset…"),
        (Locale::Fa, "admin.archive.archiving") => Some("در حال بایگانی منبع…"),
        (Locale::En, "admin.archive.success") => Some("Asset archived and withdrawn from governed retrieval."),
        (Locale::Fa, "admin.archive.success") => Some("منبع بایگانی و از بازیابی کنترل‌شده خارج شد."),
        (Locale::En, "admin.archive.error") => Some("Archive failed. The asset state is unchanged; refresh and try again."),
        (Locale::Fa, "admin.archive.error") => Some("بایگانی ناموفق بود. وضعیت منبع تغییر نکرده است؛ صفحه را تازه‌سازی و دوباره تلاش کنید."),

        (Locale::En, "admin.action.retry_embedding") => Some("Retry embedding"),
        (Locale::Fa, "admin.action.retry_embedding") => Some("تلاش دوباره برای بردارسازی"),
        (Locale::En, "admin.action.queue_embedding") => Some("Queue embedding"),
        (Locale::Fa, "admin.action.queue_embedding") => Some("قرار دادن بردارسازی در صف"),
        (Locale::En, "admin.action.publish") => Some("Publish"),
        (Locale::Fa, "admin.action.publish") => Some("انتشار"),
        (Locale::En, "admin.action.withdraw_archive") => Some("Withdraw / archive"),
        (Locale::Fa, "admin.action.withdraw_archive") => Some("خروج از انتشار / بایگانی"),
        (Locale::En, "admin.action.archive") => Some("Archive"),
        (Locale::Fa, "admin.action.archive") => Some("بایگانی"),
        (Locale::En, "admin.notice.embedding_queued") => Some("Embedding queued. Publication stays blocked until embedding completes."),
        (Locale::Fa, "admin.notice.embedding_queued") => Some("بردارسازی در صف قرار گرفت. تا تکمیل آن، انتشار مسدود می‌ماند."),
        (Locale::En, "admin.notice.embedding_error") => Some("Embedding could not be queued. Refresh the asset state and try again."),
        (Locale::Fa, "admin.notice.embedding_error") => Some("بردارسازی در صف قرار نگرفت. وضعیت منبع را تازه‌سازی و دوباره تلاش کنید."),
        (Locale::En, "admin.notice.published") => Some("Asset published. It can now be selected by teachers in the same school."),
        (Locale::Fa, "admin.notice.published") => Some("منبع منتشر شد و اکنون معلمان همان مدرسه می‌توانند آن را انتخاب کنند."),
        (Locale::En, "admin.notice.publish_error") => Some("Publication failed. Refresh the asset state and try again."),
        (Locale::Fa, "admin.notice.publish_error") => Some("انتشار ناموفق بود. وضعیت منبع را تازه‌سازی و دوباره تلاش کنید."),

        (Locale::En, "admin.lifecycle.source_review.title") => Some("Step 1 · Source review"),
        (Locale::Fa, "admin.lifecycle.source_review.title") => Some("گام ۱ · بازبینی منبع"),
        (Locale::En, "admin.lifecycle.source_review.detail") => Some("Review the private PDF and attach text only after verifying it against the source."),
        (Locale::Fa, "admin.lifecycle.source_review.detail") => Some("PDF خصوصی را بازبینی کنید و فقط پس از تطبیق متن با منبع، OCR تأییدشده را اضافه کنید."),
        (Locale::En, "admin.lifecycle.embedding.title") => Some("Step 2 · Embedding"),
        (Locale::Fa, "admin.lifecycle.embedding.title") => Some("گام ۲ · بردارسازی"),
        (Locale::En, "admin.lifecycle.embedding.detail") => Some("Verified OCR is ready. Queue embedding; publication remains blocked until it completes."),
        (Locale::Fa, "admin.lifecycle.embedding.detail") => Some("OCR تأییدشده آماده است. بردارسازی را در صف قرار دهید؛ انتشار تا پایان آن مسدود می‌ماند."),
        (Locale::En, "admin.lifecycle.embedding_pending.title") => Some("Step 2 · Embedding in progress"),
        (Locale::Fa, "admin.lifecycle.embedding_pending.title") => Some("گام ۲ · بردارسازی در حال انجام"),
        (Locale::En, "admin.lifecycle.embedding_pending.detail") => Some("An embedding job is queued or running. No ingestion transition is available until it finishes or fails."),
        (Locale::Fa, "admin.lifecycle.embedding_pending.detail") => Some("یک کار بردارسازی در صف یا در حال اجراست. تا پایان یا شکست آن، گذار پردازشی دیگری مجاز نیست."),
        (Locale::En, "admin.lifecycle.publication.title") => Some("Step 3 · Publication"),
        (Locale::Fa, "admin.lifecycle.publication.title") => Some("گام ۳ · انتشار"),
        (Locale::En, "admin.lifecycle.publication.detail") => Some("Embedding is complete. Publish explicitly to make the asset available for teacher selection."),
        (Locale::Fa, "admin.lifecycle.publication.detail") => Some("بردارسازی کامل شده است. برای در دسترس قرار گرفتن منبع برای معلمان، آن را صریحاً منتشر کنید."),
        (Locale::En, "admin.lifecycle.published.title") => Some("Published"),
        (Locale::Fa, "admin.lifecycle.published.title") => Some("منتشرشده"),
        (Locale::En, "admin.lifecycle.published.detail") => Some("The asset is available for governed teacher selection. Archive it if it should no longer be used."),
        (Locale::Fa, "admin.lifecycle.published.detail") => Some("منبع برای انتخاب کنترل‌شده معلمان در دسترس است. اگر دیگر نباید استفاده شود، آن را بایگانی کنید."),
        (Locale::En, "admin.lifecycle.archived.title") => Some("Archived"),
        (Locale::Fa, "admin.lifecycle.archived.title") => Some("بایگانی‌شده"),
        (Locale::En, "admin.lifecycle.archived.detail") => Some("This asset is withdrawn and terminal. No further ingestion or publication actions are available."),
        (Locale::Fa, "admin.lifecycle.archived.detail") => Some("این منبع خارج و نهایی شده است و اقدام پردازشی یا انتشاری دیگری برای آن وجود ندارد."),
        (Locale::En, "admin.lifecycle.recovery.title") => Some("Recovery"),
        (Locale::Fa, "admin.lifecycle.recovery.title") => Some("بازیابی"),
        (Locale::En, "admin.lifecycle.recovery_with_ocr.detail") => Some("Embedding failed after verified OCR. Retry embedding or replace the verified OCR if the source text needs correction."),
        (Locale::Fa, "admin.lifecycle.recovery_with_ocr.detail") => Some("بردارسازی پس از OCR تأییدشده ناموفق بود. دوباره تلاش کنید یا اگر متن نیاز به اصلاح دارد OCR تأییدشده را جایگزین کنید."),
        (Locale::En, "admin.lifecycle.recovery_without_ocr.detail") => Some("Processing failed before verified OCR was available. Review the source and attach verified OCR before continuing."),
        (Locale::Fa, "admin.lifecycle.recovery_without_ocr.detail") => Some("پردازش پیش از آماده شدن OCR تأییدشده ناموفق بود. منبع را بازبینی و پیش از ادامه OCR تأییدشده را اضافه کنید."),
        (Locale::En, "admin.lifecycle.unknown.title") => Some("State unavailable"),
        (Locale::Fa, "admin.lifecycle.unknown.title") => Some("وضعیت در دسترس نیست"),
        (Locale::En, "admin.lifecycle.unknown.detail") => Some("Refresh the asset list before taking another lifecycle action."),
        (Locale::Fa, "admin.lifecycle.unknown.detail") => Some("پیش از اقدام بعدی، فهرست منابع را تازه‌سازی کنید."),

        (Locale::En, "admin.audit.title") => Some("Knowledge audit trail"),
        (Locale::Fa, "admin.audit.title") => Some("ردپای ممیزی دانش"),
        (Locale::En, "admin.audit.description") => Some("Recent governed-knowledge lifecycle, source-review, and retrieval events."),
        (Locale::Fa, "admin.audit.description") => Some("رویدادهای اخیر چرخه‌عمر، بازبینی منبع و بازیابی دانش کنترل‌شده."),
        (Locale::En, "admin.audit.loading") => Some("Loading audit events…"),
        (Locale::Fa, "admin.audit.loading") => Some("در حال بارگذاری رویدادهای ممیزی…"),
        (Locale::En, "admin.audit.error") => Some("Audit events could not be loaded. Refresh and try again."),
        (Locale::Fa, "admin.audit.error") => Some("بارگذاری رویدادهای ممیزی ممکن نشد. صفحه را تازه‌سازی و دوباره تلاش کنید."),
        (Locale::En, "admin.audit.empty") => Some("No audit events have been recorded."),
        (Locale::Fa, "admin.audit.empty") => Some("هنوز رویداد ممیزی ثبت نشده است."),
        (Locale::En, "admin.audit.time") => Some("Time"),
        (Locale::Fa, "admin.audit.time") => Some("زمان"),
        (Locale::En, "admin.audit.actor") => Some("Actor"),
        (Locale::Fa, "admin.audit.actor") => Some("عامل"),
        (Locale::En, "admin.audit.action") => Some("Action"),
        (Locale::Fa, "admin.audit.action") => Some("اقدام"),
        (Locale::En, "admin.audit.target") => Some("Target"),
        (Locale::Fa, "admin.audit.target") => Some("هدف"),
        (Locale::En, "admin.audit.school") => Some("School"),
        (Locale::Fa, "admin.audit.school") => Some("مدرسه"),
        (Locale::En, "admin.audit.details") => Some("Details"),
        (Locale::Fa, "admin.audit.details") => Some("جزئیات"),
        (Locale::En, "admin.audit.inspect") => Some("Inspect technical details"),
        (Locale::Fa, "admin.audit.inspect") => Some("مشاهده جزئیات فنی"),
        (Locale::En, "admin.audit.action_code") => Some("Action code"),
        (Locale::Fa, "admin.audit.action_code") => Some("کد اقدام"),
        (Locale::En, "admin.audit.target_id") => Some("Target ID"),
        (Locale::Fa, "admin.audit.target_id") => Some("شناسه هدف"),
        (Locale::En, "admin.audit.exact_utc") => Some("Exact UTC time"),
        (Locale::Fa, "admin.audit.exact_utc") => Some("زمان دقیق UTC"),
        (Locale::En, "admin.audit.actor_id") => Some("Actor ID"),
        (Locale::Fa, "admin.audit.actor_id") => Some("شناسه عامل"),
        (Locale::En, "admin.audit.school_id") => Some("School ID"),
        (Locale::Fa, "admin.audit.school_id") => Some("شناسه مدرسه"),
        (Locale::En, "admin.audit.request_id") => Some("Request ID"),
        (Locale::Fa, "admin.audit.request_id") => Some("شناسه درخواست"),
        (Locale::En, "admin.audit.structured_details") => Some("Structured details"),
        (Locale::Fa, "admin.audit.structured_details") => Some("جزئیات ساختاریافته"),
        (Locale::En, "admin.audit.unknown_actor") => Some("System or unavailable actor"),
        (Locale::Fa, "admin.audit.unknown_actor") => Some("سامانه یا عامل نامشخص"),
        (Locale::En, "admin.audit.unknown_target") => Some("Governed knowledge item"),
        (Locale::Fa, "admin.audit.unknown_target") => Some("مورد دانش کنترل‌شده"),
        (Locale::En, "admin.audit.unknown_school") => Some("School unavailable"),
        (Locale::Fa, "admin.audit.unknown_school") => Some("مدرسه نامشخص"),
        (Locale::En, "admin.audit.role.platform_admin") => Some("Platform Administrator"),
        (Locale::Fa, "admin.audit.role.platform_admin") => Some("مدیر سامانه"),
        (Locale::En, "admin.audit.role.school_manager") => Some("School Manager"),
        (Locale::Fa, "admin.audit.role.school_manager") => Some("مدیر مدرسه"),
        (Locale::En, "admin.audit.role.teacher") => Some("Teacher"),
        (Locale::Fa, "admin.audit.role.teacher") => Some("معلم"),
        (Locale::En, "admin.audit.role.system") => Some("System"),
        (Locale::Fa, "admin.audit.role.system") => Some("سامانه"),

        (Locale::En, "admin.audit.action.submitted") => Some("Knowledge source submitted"),
        (Locale::Fa, "admin.audit.action.submitted") => Some("منبع دانشی ارسال شد"),
        (Locale::En, "admin.audit.action.source_reviewed") => Some("Source document reviewed"),
        (Locale::Fa, "admin.audit.action.source_reviewed") => Some("سند منبع بازبینی شد"),
        (Locale::En, "admin.audit.action.ocr_verified") => Some("OCR text verified"),
        (Locale::Fa, "admin.audit.action.ocr_verified") => Some("متن OCR تأیید شد"),
        (Locale::En, "admin.audit.action.embedding_queued") => Some("Embedding queued"),
        (Locale::Fa, "admin.audit.action.embedding_queued") => Some("بردارسازی در صف قرار گرفت"),
        (Locale::En, "admin.audit.action.embedded") => Some("Embedding completed"),
        (Locale::Fa, "admin.audit.action.embedded") => Some("بردارسازی تکمیل شد"),
        (Locale::En, "admin.audit.action.published") => Some("Knowledge asset published"),
        (Locale::Fa, "admin.audit.action.published") => Some("منبع دانشی منتشر شد"),
        (Locale::En, "admin.audit.action.archived") => Some("Knowledge asset archived"),
        (Locale::Fa, "admin.audit.action.archived") => Some("منبع دانشی بایگانی شد"),
        (Locale::En, "admin.audit.action.retrieved") => Some("Knowledge retrieved"),
        (Locale::Fa, "admin.audit.action.retrieved") => Some("دانش بازیابی شد"),
        (Locale::En, "admin.audit.action.updated") => Some("Knowledge asset updated"),
        (Locale::Fa, "admin.audit.action.updated") => Some("منبع دانشی به‌روزرسانی شد"),
        (Locale::En, "admin.audit.action.other") => Some("Knowledge governance event"),
        (Locale::Fa, "admin.audit.action.other") => Some("رویداد حاکمیت دانش"),
        _ => None,
    }
}

pub(crate) fn platform_admin_status_label(status: &str, locale: Locale) -> &'static str {
    let key = match status {
        "submitted" => "admin.status.submitted",
        "ocr_pending" => "admin.status.ocr_pending",
        "ocr_ready" => "admin.status.ocr_ready",
        "embedding_pending" => "admin.status.embedding_pending",
        "embedded" => "admin.status.embedded",
        "published" => "admin.status.published",
        "archived" => "admin.status.archived",
        "failed" => "admin.status.failed",
        _ => "admin.status.unknown",
    };
    platform_admin_translation(key, locale).expect("platform admin status key")
}

pub(crate) fn platform_admin_language_label(language: &str, locale: Locale) -> String {
    let key = match language.trim().to_ascii_lowercase().as_str() {
        "fa" | "fa-ir" | "persian" | "farsi" => Some("admin.language.fa"),
        "en" | "en-us" | "english" => Some("admin.language.en"),
        _ => None,
    };
    key.and_then(|key| platform_admin_translation(key, locale))
        .map(str::to_owned)
        .unwrap_or_else(|| language.to_string())
}

pub(crate) fn platform_admin_actor_role_label(role: &str, locale: Locale) -> String {
    let key = match role {
        "PlatformAdmin" | "Platform Administrator" => Some("admin.audit.role.platform_admin"),
        "SchoolManager" | "School Manager" => Some("admin.audit.role.school_manager"),
        "Teacher" => Some("admin.audit.role.teacher"),
        "System" => Some("admin.audit.role.system"),
        _ => None,
    };
    key.and_then(|key| platform_admin_translation(key, locale))
        .map(str::to_owned)
        .unwrap_or_else(|| role.to_string())
}

pub(crate) fn platform_admin_audit_action_label(action: &str, locale: Locale) -> &'static str {
    let key = match action {
        "knowledge_asset.submitted" | "knowledge_asset.created" => "admin.audit.action.submitted",
        "knowledge_asset.source_reviewed" | "knowledge_asset.source_review" => "admin.audit.action.source_reviewed",
        "knowledge_asset.ocr_verified" | "knowledge_asset.ocr_updated" => "admin.audit.action.ocr_verified",
        "knowledge_asset.embedding_queued" | "knowledge_asset.embedding_started" => "admin.audit.action.embedding_queued",
        "knowledge_asset.embedded" | "knowledge_asset.embedding_completed" => "admin.audit.action.embedded",
        "knowledge_asset.published" => "admin.audit.action.published",
        "knowledge_asset.archived" | "knowledge_asset.withdrawn" => "admin.audit.action.archived",
        "knowledge_asset.retrieved" | "knowledge.retrieved" => "admin.audit.action.retrieved",
        "knowledge_asset.updated" => "admin.audit.action.updated",
        _ => "admin.audit.action.other",
    };
    platform_admin_translation(key, locale).expect("platform admin audit action key")
}

pub(crate) fn platform_admin_lifecycle_guidance(
    status: &str,
    has_verified_ocr: bool,
    locale: Locale,
) -> (&'static str, &'static str) {
    let (title_key, detail_key) = match status {
        "submitted" | "ocr_pending" => (
            "admin.lifecycle.source_review.title",
            "admin.lifecycle.source_review.detail",
        ),
        "ocr_ready" => (
            "admin.lifecycle.embedding.title",
            "admin.lifecycle.embedding.detail",
        ),
        "embedding_pending" => (
            "admin.lifecycle.embedding_pending.title",
            "admin.lifecycle.embedding_pending.detail",
        ),
        "embedded" => (
            "admin.lifecycle.publication.title",
            "admin.lifecycle.publication.detail",
        ),
        "published" => (
            "admin.lifecycle.published.title",
            "admin.lifecycle.published.detail",
        ),
        "archived" => (
            "admin.lifecycle.archived.title",
            "admin.lifecycle.archived.detail",
        ),
        "failed" if has_verified_ocr => (
            "admin.lifecycle.recovery.title",
            "admin.lifecycle.recovery_with_ocr.detail",
        ),
        "failed" => (
            "admin.lifecycle.recovery.title",
            "admin.lifecycle.recovery_without_ocr.detail",
        ),
        _ => (
            "admin.lifecycle.unknown.title",
            "admin.lifecycle.unknown.detail",
        ),
    };
    (
        platform_admin_translation(title_key, locale).expect("platform admin lifecycle title"),
        platform_admin_translation(detail_key, locale).expect("platform admin lifecycle detail"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEYS: &[&str] = &[
        "admin.review.title",
        "admin.review.description",
        "admin.review.loading",
        "admin.review.error",
        "admin.review.empty",
        "admin.status.ocr_ready",
        "admin.status.embedding_pending",
        "admin.status.published",
        "admin.school.label",
        "admin.metadata.subject",
        "admin.metadata.grade",
        "admin.metadata.language",
        "admin.source.title",
        "admin.source.review",
        "admin.ocr.provider",
        "admin.ocr.text",
        "admin.ocr.save",
        "admin.archive.confirm",
        "admin.action.queue_embedding",
        "admin.action.publish",
        "admin.audit.title",
        "admin.audit.time",
        "admin.audit.actor",
        "admin.audit.action",
        "admin.audit.target",
        "admin.audit.details",
        "admin.audit.inspect",
        "admin.audit.action_code",
        "admin.audit.target_id",
        "admin.audit.exact_utc",
        "admin.audit.structured_details",
    ];

    #[test]
    fn platform_admin_translation_keys_have_en_fa_parity() {
        for key in KEYS {
            let en = platform_admin_translation(key, Locale::En);
            let fa = platform_admin_translation(key, Locale::Fa);
            assert!(en.is_some(), "missing English {key}");
            assert!(fa.is_some(), "missing Persian {key}");
            assert_ne!(en, Some(*key), "raw English key leaked for {key}");
            assert_ne!(fa, Some(*key), "raw Persian key leaked for {key}");
        }
    }

    #[test]
    fn persisted_admin_states_never_render_as_primary_labels() {
        assert_eq!(platform_admin_status_label("ocr_ready", Locale::En), "OCR verified");
        assert_eq!(platform_admin_status_label("embedding_pending", Locale::Fa), "بردارسازی در حال انجام");
        assert_ne!(platform_admin_status_label("ocr_ready", Locale::En), "ocr_ready");
    }

    #[test]
    fn audit_action_codes_have_readable_primary_labels() {
        assert_eq!(
            platform_admin_audit_action_label("knowledge_asset.ocr_verified", Locale::En),
            "OCR text verified"
        );
        assert_ne!(
            platform_admin_audit_action_label("knowledge_asset.archived", Locale::Fa),
            "knowledge_asset.archived"
        );
    }
}
