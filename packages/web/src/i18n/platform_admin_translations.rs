use super::Locale;

pub(crate) fn platform_admin_translation(
    key: &'static str,
    locale: Locale,
) -> Option<&'static str> {
    match (locale, key) {
        (Locale::En, "platform_admin.loading") => Some("Loading platform administration…"),
        (Locale::Fa, "platform_admin.loading") => Some("در حال بارگذاری مدیریت سامانه…"),

        (Locale::En, "platform_admin.review.title") => Some("Governed knowledge review"),
        (Locale::Fa, "platform_admin.review.title") => Some("بازبینی دانش کنترل‌شده"),
        (Locale::En, "platform_admin.review.description") => Some("Review the private source, verify OCR, embed, then publish explicitly. Each asset shows its next valid lifecycle action."),
        (Locale::Fa, "platform_admin.review.description") => Some("منبع خصوصی را بررسی کنید، متن OCR را تأیید کنید، بردارسازی را انجام دهید و سپس منبع را به‌صورت صریح منتشر کنید. برای هر منبع، اقدام معتبر بعدی نمایش داده می‌شود."),
        (Locale::En, "platform_admin.review.step1.title") => Some("Review & verify OCR"),
        (Locale::Fa, "platform_admin.review.step1.title") => Some("بازبینی و تأیید OCR"),
        (Locale::En, "platform_admin.review.step1.detail") => Some("Inspect the private PDF and save only verified text."),
        (Locale::Fa, "platform_admin.review.step1.detail") => Some("PDF خصوصی را بررسی کنید و فقط متن تأییدشده را ذخیره کنید."),
        (Locale::En, "platform_admin.review.step2.title") => Some("Embed"),
        (Locale::Fa, "platform_admin.review.step2.title") => Some("بردارسازی"),
        (Locale::En, "platform_admin.review.step2.detail") => Some("Queue embedding only after verified OCR exists."),
        (Locale::Fa, "platform_admin.review.step2.detail") => Some("بردارسازی را فقط پس از وجود متن OCR تأییدشده در صف قرار دهید."),
        (Locale::En, "platform_admin.review.step3.title") => Some("Publish"),
        (Locale::Fa, "platform_admin.review.step3.title") => Some("انتشار"),
        (Locale::En, "platform_admin.review.step3.detail") => Some("Publish explicitly only after embedding completes."),
        (Locale::Fa, "platform_admin.review.step3.detail") => Some("فقط پس از تکمیل بردارسازی، منبع را به‌صورت صریح منتشر کنید."),
        (Locale::En, "platform_admin.review.loading") => Some("Loading governed assets…"),
        (Locale::Fa, "platform_admin.review.loading") => Some("در حال بارگذاری منابع کنترل‌شده…"),
        (Locale::En, "platform_admin.review.load_error") => Some("Governed assets could not be loaded. Refresh and try again."),
        (Locale::Fa, "platform_admin.review.load_error") => Some("بارگذاری منابع کنترل‌شده ممکن نشد. صفحه را تازه‌سازی کنید و دوباره تلاش کنید."),
        (Locale::En, "platform_admin.review.empty") => Some("No manager submissions are waiting."),
        (Locale::Fa, "platform_admin.review.empty") => Some("هیچ ارسال مدیری در انتظار بازبینی نیست."),

        (Locale::En, "platform_admin.metadata.school") => Some("School"),
        (Locale::Fa, "platform_admin.metadata.school") => Some("مدرسه"),
        (Locale::En, "platform_admin.metadata.school_reference") => Some("School reference"),
        (Locale::Fa, "platform_admin.metadata.school_reference") => Some("شناسه مرجع مدرسه"),
        (Locale::En, "platform_admin.metadata.subject") => Some("Subject"),
        (Locale::Fa, "platform_admin.metadata.subject") => Some("درس"),
        (Locale::En, "platform_admin.metadata.grade") => Some("Grade"),
        (Locale::Fa, "platform_admin.metadata.grade") => Some("پایه"),
        (Locale::En, "platform_admin.metadata.language") => Some("Language"),
        (Locale::Fa, "platform_admin.metadata.language") => Some("زبان"),
        (Locale::En, "platform_admin.language.fa") => Some("Persian"),
        (Locale::Fa, "platform_admin.language.fa") => Some("فارسی"),
        (Locale::En, "platform_admin.language.en") => Some("English"),
        (Locale::Fa, "platform_admin.language.en") => Some("انگلیسی"),

        (Locale::En, "platform_admin.status.submitted") => Some("Submitted"),
        (Locale::Fa, "platform_admin.status.submitted") => Some("ارسال‌شده"),
        (Locale::En, "platform_admin.status.ocr_pending") => Some("OCR pending"),
        (Locale::Fa, "platform_admin.status.ocr_pending") => Some("در انتظار OCR"),
        (Locale::En, "platform_admin.status.ocr_ready") => Some("OCR verified"),
        (Locale::Fa, "platform_admin.status.ocr_ready") => Some("OCR تأییدشده"),
        (Locale::En, "platform_admin.status.embedding_pending") => Some("Embedding in progress"),
        (Locale::Fa, "platform_admin.status.embedding_pending") => Some("بردارسازی در حال انجام"),
        (Locale::En, "platform_admin.status.embedded") => Some("Embedded"),
        (Locale::Fa, "platform_admin.status.embedded") => Some("بردارسازی‌شده"),
        (Locale::En, "platform_admin.status.published") => Some("Published"),
        (Locale::Fa, "platform_admin.status.published") => Some("منتشرشده"),
        (Locale::En, "platform_admin.status.archived") => Some("Archived"),
        (Locale::Fa, "platform_admin.status.archived") => Some("بایگانی‌شده"),
        (Locale::En, "platform_admin.status.failed") => Some("Needs attention"),
        (Locale::Fa, "platform_admin.status.failed") => Some("نیازمند بررسی"),
        (Locale::En, "platform_admin.status.unknown") => Some("Status unavailable"),
        (Locale::Fa, "platform_admin.status.unknown") => Some("وضعیت در دسترس نیست"),

        (Locale::En, "platform_admin.guidance.source.title") => Some("Step 1 · Source review"),
        (Locale::Fa, "platform_admin.guidance.source.title") => Some("مرحله ۱ · بازبینی منبع"),
        (Locale::En, "platform_admin.guidance.source.detail") => Some("Review the private PDF and attach text only after verifying it against the source."),
        (Locale::Fa, "platform_admin.guidance.source.detail") => Some("PDF خصوصی را بررسی کنید و فقط پس از تطبیق متن با منبع، متن تأییدشده را پیوست کنید."),
        (Locale::En, "platform_admin.guidance.embed.title") => Some("Step 2 · Embedding"),
        (Locale::Fa, "platform_admin.guidance.embed.title") => Some("مرحله ۲ · بردارسازی"),
        (Locale::En, "platform_admin.guidance.embed.detail") => Some("Verified OCR is ready. Queue embedding; publication remains blocked until it completes."),
        (Locale::Fa, "platform_admin.guidance.embed.detail") => Some("OCR تأییدشده آماده است. بردارسازی را در صف قرار دهید؛ تا تکمیل آن انتشار مسدود می‌ماند."),
        (Locale::En, "platform_admin.guidance.embedding.title") => Some("Step 2 · Embedding in progress"),
        (Locale::Fa, "platform_admin.guidance.embedding.title") => Some("مرحله ۲ · بردارسازی در حال انجام"),
        (Locale::En, "platform_admin.guidance.embedding.detail") => Some("An embedding job is queued or running. Wait for completion or failure before another ingestion transition."),
        (Locale::Fa, "platform_admin.guidance.embedding.detail") => Some("یک کار بردارسازی در صف یا در حال اجرا است. پیش از تغییر بعدی، منتظر تکمیل یا شکست آن بمانید."),
        (Locale::En, "platform_admin.guidance.publish.title") => Some("Step 3 · Publication"),
        (Locale::Fa, "platform_admin.guidance.publish.title") => Some("مرحله ۳ · انتشار"),
        (Locale::En, "platform_admin.guidance.publish.detail") => Some("Embedding is complete. Publish explicitly to make the asset available for teacher selection."),
        (Locale::Fa, "platform_admin.guidance.publish.detail") => Some("بردارسازی کامل شده است. برای در دسترس قرار گرفتن منبع برای انتخاب معلم، آن را صریحاً منتشر کنید."),
        (Locale::En, "platform_admin.guidance.published.title") => Some("Published"),
        (Locale::Fa, "platform_admin.guidance.published.title") => Some("منتشرشده"),
        (Locale::En, "platform_admin.guidance.published.detail") => Some("The asset is available for governed teacher selection. Archive it to withdraw it from use."),
        (Locale::Fa, "platform_admin.guidance.published.detail") => Some("منبع برای انتخاب کنترل‌شده معلم در دسترس است. برای خارج کردن آن از استفاده، بایگانی‌اش کنید."),
        (Locale::En, "platform_admin.guidance.archived.title") => Some("Archived"),
        (Locale::Fa, "platform_admin.guidance.archived.title") => Some("بایگانی‌شده"),
        (Locale::En, "platform_admin.guidance.archived.detail") => Some("This asset is withdrawn and terminal. No further ingestion or publication actions are available."),
        (Locale::Fa, "platform_admin.guidance.archived.detail") => Some("این منبع از چرخه استفاده خارج و نهایی شده است و اقدام پردازش یا انتشار دیگری ندارد."),
        (Locale::En, "platform_admin.guidance.recovery_ocr.title") => Some("Recovery"),
        (Locale::Fa, "platform_admin.guidance.recovery_ocr.title") => Some("بازیابی"),
        (Locale::En, "platform_admin.guidance.recovery_ocr.detail") => Some("Embedding failed after verified OCR. Retry embedding or update verified OCR if the source text needs correction."),
        (Locale::Fa, "platform_admin.guidance.recovery_ocr.detail") => Some("بردارسازی پس از تأیید OCR ناموفق شد. بردارسازی را دوباره اجرا کنید یا در صورت نیاز متن OCR تأییدشده را اصلاح کنید."),
        (Locale::En, "platform_admin.guidance.recovery_source.title") => Some("Recovery"),
        (Locale::Fa, "platform_admin.guidance.recovery_source.title") => Some("بازیابی"),
        (Locale::En, "platform_admin.guidance.recovery_source.detail") => Some("Processing failed before verified OCR was available. Review the source and attach verified OCR before continuing."),
        (Locale::Fa, "platform_admin.guidance.recovery_source.detail") => Some("پردازش پیش از آماده شدن OCR تأییدشده ناموفق شد. منبع را بررسی و OCR تأییدشده را پیوست کنید."),
        (Locale::En, "platform_admin.guidance.unknown.title") => Some("State unavailable"),
        (Locale::Fa, "platform_admin.guidance.unknown.title") => Some("وضعیت در دسترس نیست"),
        (Locale::En, "platform_admin.guidance.unknown.detail") => Some("Refresh the asset list before taking another lifecycle action."),
        (Locale::Fa, "platform_admin.guidance.unknown.detail") => Some("پیش از اقدام بعدی، فهرست منابع را تازه‌سازی کنید."),

        (Locale::En, "platform_admin.source.title") => Some("Source document"),
        (Locale::Fa, "platform_admin.source.title") => Some("سند منبع"),
        (Locale::En, "platform_admin.source.review") => Some("Review private PDF"),
        (Locale::Fa, "platform_admin.source.review") => Some("بازبینی PDF خصوصی"),
        (Locale::En, "platform_admin.source.unavailable") => Some("Private source review is unavailable for this legacy submission."),
        (Locale::Fa, "platform_admin.source.unavailable") => Some("بازبینی منبع خصوصی برای این ارسال قدیمی در دسترس نیست."),
        (Locale::En, "platform_admin.source.metadata_unavailable") => Some("Source metadata unavailable"),
        (Locale::Fa, "platform_admin.source.metadata_unavailable") => Some("فراداده منبع در دسترس نیست"),

        (Locale::En, "platform_admin.action.update_ocr") => Some("Update verified OCR"),
        (Locale::Fa, "platform_admin.action.update_ocr") => Some("به‌روزرسانی OCR تأییدشده"),
        (Locale::En, "platform_admin.action.attach_ocr") => Some("Attach verified OCR"),
        (Locale::Fa, "platform_admin.action.attach_ocr") => Some("پیوست OCR تأییدشده"),
        (Locale::En, "platform_admin.action.retry_embedding") => Some("Retry embedding"),
        (Locale::Fa, "platform_admin.action.retry_embedding") => Some("تلاش دوباره برای بردارسازی"),
        (Locale::En, "platform_admin.action.queue_embedding") => Some("Queue embedding"),
        (Locale::Fa, "platform_admin.action.queue_embedding") => Some("قرار دادن بردارسازی در صف"),
        (Locale::En, "platform_admin.action.publish") => Some("Publish"),
        (Locale::Fa, "platform_admin.action.publish") => Some("انتشار"),
        (Locale::En, "platform_admin.action.withdraw_archive") => Some("Withdraw / archive"),
        (Locale::Fa, "platform_admin.action.withdraw_archive") => Some("خروج از استفاده / بایگانی"),
        (Locale::En, "platform_admin.action.archive") => Some("Archive"),
        (Locale::Fa, "platform_admin.action.archive") => Some("بایگانی"),

        (Locale::En, "platform_admin.notice.ocr_saved") => Some("Verified OCR saved. The asset is ready for embedding."),
        (Locale::Fa, "platform_admin.notice.ocr_saved") => Some("OCR تأییدشده ذخیره شد. منبع برای بردارسازی آماده است."),
        (Locale::En, "platform_admin.notice.embedding_queued") => Some("Embedding queued. Publication stays blocked until embedding completes."),
        (Locale::Fa, "platform_admin.notice.embedding_queued") => Some("بردارسازی در صف قرار گرفت. تا تکمیل آن، انتشار مسدود می‌ماند."),
        (Locale::En, "platform_admin.notice.embedding_failed") => Some("Embedding could not be queued. Refresh the asset state and try again."),
        (Locale::Fa, "platform_admin.notice.embedding_failed") => Some("بردارسازی در صف قرار نگرفت. وضعیت منبع را تازه‌سازی کنید و دوباره تلاش کنید."),
        (Locale::En, "platform_admin.notice.published") => Some("Asset published. It can now be selected by teachers in the same school."),
        (Locale::Fa, "platform_admin.notice.published") => Some("منبع منتشر شد و اکنون معلمان همان مدرسه می‌توانند آن را انتخاب کنند."),
        (Locale::En, "platform_admin.notice.publish_failed") => Some("Publication failed. Refresh the asset state and try again."),
        (Locale::Fa, "platform_admin.notice.publish_failed") => Some("انتشار ناموفق بود. وضعیت منبع را تازه‌سازی کنید و دوباره تلاش کنید."),
        (Locale::En, "platform_admin.notice.archiving") => Some("Archiving asset…"),
        (Locale::Fa, "platform_admin.notice.archiving") => Some("در حال بایگانی منبع…"),
        (Locale::En, "platform_admin.notice.archived") => Some("Asset archived and withdrawn from governed retrieval."),
        (Locale::Fa, "platform_admin.notice.archived") => Some("منبع بایگانی و از بازیابی کنترل‌شده خارج شد."),
        (Locale::En, "platform_admin.notice.archive_failed") => Some("Archive failed. The asset state is unchanged; refresh and try again."),
        (Locale::Fa, "platform_admin.notice.archive_failed") => Some("بایگانی ناموفق بود و وضعیت منبع تغییر نکرد. تازه‌سازی کنید و دوباره تلاش کنید."),

        (Locale::En, "platform_admin.ocr.update_title") => Some("Update verified OCR"),
        (Locale::Fa, "platform_admin.ocr.update_title") => Some("به‌روزرسانی OCR تأییدشده"),
        (Locale::En, "platform_admin.ocr.attach_title") => Some("Attach verified OCR"),
        (Locale::Fa, "platform_admin.ocr.attach_title") => Some("پیوست OCR تأییدشده"),
        (Locale::En, "platform_admin.ocr.close") => Some("Close OCR editor"),
        (Locale::Fa, "platform_admin.ocr.close") => Some("بستن ویرایشگر OCR"),
        (Locale::En, "platform_admin.ocr.helper") => Some("Confirm the text against the private source PDF before saving. Saving verified OCR does not publish the asset."),
        (Locale::Fa, "platform_admin.ocr.helper") => Some("پیش از ذخیره، متن را با PDF خصوصی منبع تطبیق دهید. ذخیره OCR تأییدشده به‌معنای انتشار منبع نیست."),
        (Locale::En, "platform_admin.ocr.loading") => Some("Loading the current verified OCR…"),
        (Locale::Fa, "platform_admin.ocr.loading") => Some("در حال بارگذاری OCR تأییدشده فعلی…"),
        (Locale::En, "platform_admin.ocr.source_load_error") => Some("The current governed source revision could not be loaded. Refresh and review the source again."),
        (Locale::Fa, "platform_admin.ocr.source_load_error") => Some("نسخه فعلی منبع کنترل‌شده بارگذاری نشد. تازه‌سازی کنید و منبع را دوباره بررسی کنید."),
        (Locale::En, "platform_admin.ocr.load_error") => Some("The current verified OCR could not be loaded. Refresh and try again."),
        (Locale::Fa, "platform_admin.ocr.load_error") => Some("OCR تأییدشده فعلی بارگذاری نشد. تازه‌سازی کنید و دوباره تلاش کنید."),
        (Locale::En, "platform_admin.ocr.source_revision_unavailable") => Some("The governed source revision is unavailable. Refresh and review the source again."),
        (Locale::Fa, "platform_admin.ocr.source_revision_unavailable") => Some("نسخه منبع کنترل‌شده در دسترس نیست. تازه‌سازی کنید و منبع را دوباره بررسی کنید."),
        (Locale::En, "platform_admin.ocr.save_conflict") => Some("OCR could not be saved because the source or OCR revision changed, the source is not reviewed, or the asset is no longer eligible. Refresh and review it again."),
        (Locale::Fa, "platform_admin.ocr.save_conflict") => Some("OCR ذخیره نشد؛ ممکن است نسخه منبع یا OCR تغییر کرده باشد، منبع هنوز بازبینی نشده باشد یا منبع دیگر واجد شرایط نباشد. تازه‌سازی و دوباره بازبینی کنید."),
        (Locale::En, "platform_admin.ocr.source_revision") => Some("Governed source revision"),
        (Locale::Fa, "platform_admin.ocr.source_revision") => Some("نسخه منبع کنترل‌شده"),
        (Locale::En, "platform_admin.ocr.source_hash") => Some("Source hash"),
        (Locale::Fa, "platform_admin.ocr.source_hash") => Some("هش منبع"),
        (Locale::En, "platform_admin.ocr.current_revision") => Some("Current verified revision"),
        (Locale::Fa, "platform_admin.ocr.current_revision") => Some("نسخه تأییدشده فعلی"),
        (Locale::En, "platform_admin.ocr.verified_at") => Some("Verified at"),
        (Locale::Fa, "platform_admin.ocr.verified_at") => Some("زمان تأیید"),
        (Locale::En, "platform_admin.ocr.verified_by") => Some("Verified by"),
        (Locale::Fa, "platform_admin.ocr.verified_by") => Some("تأییدکننده"),
        (Locale::En, "platform_admin.ocr.text_hash") => Some("Text hash"),
        (Locale::Fa, "platform_admin.ocr.text_hash") => Some("هش متن"),
        (Locale::En, "platform_admin.ocr.provider") => Some("OCR provider / verification process"),
        (Locale::Fa, "platform_admin.ocr.provider") => Some("ارائه‌دهنده OCR / فرایند تأیید"),
        (Locale::En, "platform_admin.ocr.text") => Some("Verified source text"),
        (Locale::Fa, "platform_admin.ocr.text") => Some("متن تأییدشده منبع"),
        (Locale::En, "platform_admin.ocr.placeholder") => Some("Paste text that has been checked against the source PDF…"),
        (Locale::Fa, "platform_admin.ocr.placeholder") => Some("متنی را که با PDF منبع تطبیق داده شده است وارد کنید…"),
        (Locale::En, "platform_admin.ocr.reload") => Some("Reload current OCR and replace draft"),
        (Locale::Fa, "platform_admin.ocr.reload") => Some("بارگذاری دوباره OCR فعلی و جایگزینی پیش‌نویس"),
        (Locale::En, "platform_admin.ocr.cancel") => Some("Cancel"),
        (Locale::Fa, "platform_admin.ocr.cancel") => Some("انصراف"),
        (Locale::En, "platform_admin.ocr.saving") => Some("Saving…"),
        (Locale::Fa, "platform_admin.ocr.saving") => Some("در حال ذخیره…"),
        (Locale::En, "platform_admin.ocr.save_changes") => Some("Save verified OCR changes"),
        (Locale::Fa, "platform_admin.ocr.save_changes") => Some("ذخیره تغییرات OCR تأییدشده"),
        (Locale::En, "platform_admin.ocr.save") => Some("Save verified OCR"),
        (Locale::Fa, "platform_admin.ocr.save") => Some("ذخیره OCR تأییدشده"),
        (Locale::En, "platform_admin.ocr.discard_open") => Some("Discard OCR changes and open"),
        (Locale::Fa, "platform_admin.ocr.discard_open") => Some("کنار گذاشتن تغییرات OCR و باز کردن"),
        (Locale::En, "platform_admin.ocr.discard_title") => Some("Discard unsaved OCR changes?"),
        (Locale::Fa, "platform_admin.ocr.discard_title") => Some("تغییرات ذخیره‌نشده OCR کنار گذاشته شود؟"),
        (Locale::En, "platform_admin.ocr.discard_description") => Some("Your unsaved OCR text or verification method will be lost."),
        (Locale::Fa, "platform_admin.ocr.discard_description") => Some("متن OCR یا روش تأیید ذخیره‌نشده از دست خواهد رفت."),
        (Locale::En, "platform_admin.ocr.discard_confirm") => Some("Discard changes"),
        (Locale::Fa, "platform_admin.ocr.discard_confirm") => Some("کنار گذاشتن تغییرات"),
        (Locale::En, "platform_admin.ocr.keep_editing") => Some("Keep editing"),
        (Locale::Fa, "platform_admin.ocr.keep_editing") => Some("ادامه ویرایش"),

        (Locale::En, "platform_admin.archive.title") => Some("Archive asset"),
        (Locale::Fa, "platform_admin.archive.title") => Some("بایگانی منبع"),
        (Locale::En, "platform_admin.archive.published_description") => Some("This withdraws the published asset from teacher retrieval and cancels active ingestion work."),
        (Locale::Fa, "platform_admin.archive.published_description") => Some("این کار منبع منتشرشده را از بازیابی معلم خارج و پردازش فعال را لغو می‌کند."),
        (Locale::En, "platform_admin.archive.description") => Some("This archives the asset and cancels active ingestion work. Archived assets are terminal."),
        (Locale::Fa, "platform_admin.archive.description") => Some("این کار منبع را بایگانی و پردازش فعال را لغو می‌کند. منابع بایگانی‌شده نهایی هستند."),
        (Locale::En, "platform_admin.archive.confirm") => Some("Archive asset"),
        (Locale::Fa, "platform_admin.archive.confirm") => Some("بایگانی منبع"),
        (Locale::En, "platform_admin.archive.cancel") => Some("Cancel"),
        (Locale::Fa, "platform_admin.archive.cancel") => Some("انصراف"),

        (Locale::En, "platform_admin.audit.title") => Some("Knowledge audit trail"),
        (Locale::Fa, "platform_admin.audit.title") => Some("ردپای ممیزی دانش"),
        (Locale::En, "platform_admin.audit.description") => Some("Recent governed-knowledge lifecycle, source-review, and retrieval events."),
        (Locale::Fa, "platform_admin.audit.description") => Some("رویدادهای اخیر چرخه عمر، بازبینی منبع و بازیابی دانش کنترل‌شده."),
        (Locale::En, "platform_admin.audit.loading") => Some("Loading audit events…"),
        (Locale::Fa, "platform_admin.audit.loading") => Some("در حال بارگذاری رویدادهای ممیزی…"),
        (Locale::En, "platform_admin.audit.load_error") => Some("Audit events could not be loaded. Refresh and try again."),
        (Locale::Fa, "platform_admin.audit.load_error") => Some("رویدادهای ممیزی بارگذاری نشد. تازه‌سازی کنید و دوباره تلاش کنید."),
        (Locale::En, "platform_admin.audit.empty") => Some("No audit events have been recorded."),
        (Locale::Fa, "platform_admin.audit.empty") => Some("هیچ رویداد ممیزی ثبت نشده است."),
        (Locale::En, "platform_admin.audit.time") => Some("Time"),
        (Locale::Fa, "platform_admin.audit.time") => Some("زمان"),
        (Locale::En, "platform_admin.audit.actor") => Some("Actor"),
        (Locale::Fa, "platform_admin.audit.actor") => Some("عامل"),
        (Locale::En, "platform_admin.audit.action") => Some("Action"),
        (Locale::Fa, "platform_admin.audit.action") => Some("اقدام"),
        (Locale::En, "platform_admin.audit.target") => Some("Target"),
        (Locale::Fa, "platform_admin.audit.target") => Some("هدف"),
        (Locale::En, "platform_admin.audit.details") => Some("Details"),
        (Locale::Fa, "platform_admin.audit.details") => Some("جزئیات"),
        (Locale::En, "platform_admin.audit.view_details") => Some("View details"),
        (Locale::Fa, "platform_admin.audit.view_details") => Some("مشاهده جزئیات"),
        (Locale::En, "platform_admin.audit.detail_title") => Some("Audit event details"),
        (Locale::Fa, "platform_admin.audit.detail_title") => Some("جزئیات رویداد ممیزی"),
        (Locale::En, "platform_admin.audit.friendly_time") => Some("Displayed time"),
        (Locale::Fa, "platform_admin.audit.friendly_time") => Some("زمان نمایشی"),
        (Locale::En, "platform_admin.audit.exact_utc") => Some("Exact UTC timestamp"),
        (Locale::Fa, "platform_admin.audit.exact_utc") => Some("زمان دقیق UTC"),
        (Locale::En, "platform_admin.audit.action_code") => Some("Exact action code"),
        (Locale::Fa, "platform_admin.audit.action_code") => Some("کد دقیق اقدام"),
        (Locale::En, "platform_admin.audit.target_type") => Some("Target type"),
        (Locale::Fa, "platform_admin.audit.target_type") => Some("نوع هدف"),
        (Locale::En, "platform_admin.audit.target_id") => Some("Exact target ID"),
        (Locale::Fa, "platform_admin.audit.target_id") => Some("شناسه دقیق هدف"),
        (Locale::En, "platform_admin.audit.school") => Some("School context"),
        (Locale::Fa, "platform_admin.audit.school") => Some("زمینه مدرسه"),
        (Locale::En, "platform_admin.audit.school_id") => Some("Exact school ID"),
        (Locale::Fa, "platform_admin.audit.school_id") => Some("شناسه دقیق مدرسه"),
        (Locale::En, "platform_admin.audit.actor_role") => Some("Actor role"),
        (Locale::Fa, "platform_admin.audit.actor_role") => Some("نقش عامل"),
        (Locale::En, "platform_admin.audit.actor_id") => Some("Exact actor ID"),
        (Locale::Fa, "platform_admin.audit.actor_id") => Some("شناسه دقیق عامل"),
        (Locale::En, "platform_admin.audit.request_id") => Some("Request ID"),
        (Locale::Fa, "platform_admin.audit.request_id") => Some("شناسه درخواست"),
        (Locale::En, "platform_admin.audit.structured_details") => Some("Technical event details"),
        (Locale::Fa, "platform_admin.audit.structured_details") => Some("جزئیات فنی رویداد"),
        (Locale::En, "platform_admin.audit.none") => Some("Not recorded"),
        (Locale::Fa, "platform_admin.audit.none") => Some("ثبت نشده"),
        (Locale::En, "platform_admin.audit.action.submitted") => Some("Knowledge source submitted"),
        (Locale::Fa, "platform_admin.audit.action.submitted") => Some("منبع دانشی ارسال شد"),
        (Locale::En, "platform_admin.audit.action.source_reviewed") => Some("Private source reviewed"),
        (Locale::Fa, "platform_admin.audit.action.source_reviewed") => Some("منبع خصوصی بازبینی شد"),
        (Locale::En, "platform_admin.audit.action.ocr_verified") => Some("OCR text verified"),
        (Locale::Fa, "platform_admin.audit.action.ocr_verified") => Some("متن OCR تأیید شد"),
        (Locale::En, "platform_admin.audit.action.embedding_queued") => Some("Embedding queued"),
        (Locale::Fa, "platform_admin.audit.action.embedding_queued") => Some("بردارسازی در صف قرار گرفت"),
        (Locale::En, "platform_admin.audit.action.embedded") => Some("Embedding completed"),
        (Locale::Fa, "platform_admin.audit.action.embedded") => Some("بردارسازی تکمیل شد"),
        (Locale::En, "platform_admin.audit.action.published") => Some("Knowledge asset published"),
        (Locale::Fa, "platform_admin.audit.action.published") => Some("منبع دانشی منتشر شد"),
        (Locale::En, "platform_admin.audit.action.archived") => Some("Knowledge asset archived"),
        (Locale::Fa, "platform_admin.audit.action.archived") => Some("منبع دانشی بایگانی شد"),
        (Locale::En, "platform_admin.audit.action.other") => Some("Governance event"),
        (Locale::Fa, "platform_admin.audit.action.other") => Some("رویداد حاکمیتی"),
        (Locale::En, "platform_admin.audit.actor.platform_admin") => Some("Platform administrator"),
        (Locale::Fa, "platform_admin.audit.actor.platform_admin") => Some("مدیر سامانه"),
        (Locale::En, "platform_admin.audit.actor.school_manager") => Some("School manager"),
        (Locale::Fa, "platform_admin.audit.actor.school_manager") => Some("مدیر مدرسه"),
        (Locale::En, "platform_admin.audit.actor.teacher") => Some("Teacher"),
        (Locale::Fa, "platform_admin.audit.actor.teacher") => Some("معلم"),
        (Locale::En, "platform_admin.audit.actor.system") => Some("System actor"),
        (Locale::Fa, "platform_admin.audit.actor.system") => Some("عامل سامانه"),
        (Locale::En, "platform_admin.audit.target.asset") => Some("Knowledge asset"),
        (Locale::Fa, "platform_admin.audit.target.asset") => Some("منبع دانشی"),
        (Locale::En, "platform_admin.audit.target.source") => Some("Source document"),
        (Locale::Fa, "platform_admin.audit.target.source") => Some("سند منبع"),
        (Locale::En, "platform_admin.audit.target.ocr") => Some("Verified OCR"),
        (Locale::Fa, "platform_admin.audit.target.ocr") => Some("OCR تأییدشده"),
        (Locale::En, "platform_admin.audit.target.other") => Some("Governance record"),
        (Locale::Fa, "platform_admin.audit.target.other") => Some("رکورد حاکمیتی"),

        _ => None,
    }
}

pub(crate) fn platform_admin_status_label(status: &str, locale: Locale) -> String {
    let key = match status {
        "submitted" => "platform_admin.status.submitted",
        "ocr_pending" => "platform_admin.status.ocr_pending",
        "ocr_ready" => "platform_admin.status.ocr_ready",
        "embedding_pending" => "platform_admin.status.embedding_pending",
        "embedded" => "platform_admin.status.embedded",
        "published" => "platform_admin.status.published",
        "archived" => "platform_admin.status.archived",
        "failed" => "platform_admin.status.failed",
        _ => "platform_admin.status.unknown",
    };
    platform_admin_translation(key, locale)
        .unwrap_or("Status unavailable")
        .to_string()
}

pub(crate) fn platform_admin_language_label(language: &str, locale: Locale) -> String {
    let normalized = language.trim().to_ascii_lowercase();
    let key = if normalized == "fa" || normalized.starts_with("fa-") {
        Some("platform_admin.language.fa")
    } else if normalized == "en" || normalized.starts_with("en-") {
        Some("platform_admin.language.en")
    } else {
        None
    };
    key.and_then(|key| platform_admin_translation(key, locale))
        .map(str::to_owned)
        .unwrap_or_else(|| language.to_string())
}

pub(crate) fn platform_admin_audit_action_label(action: &str, locale: Locale) -> String {
    let key = match action {
        "knowledge_asset.submitted" | "knowledge_source.submitted" => {
            "platform_admin.audit.action.submitted"
        }
        "knowledge_asset.source_reviewed" | "knowledge_source.reviewed" => {
            "platform_admin.audit.action.source_reviewed"
        }
        "knowledge_asset.ocr_verified" | "knowledge_ocr.verified" => {
            "platform_admin.audit.action.ocr_verified"
        }
        "knowledge_asset.embedding_queued" | "knowledge_asset.embed_queued" => {
            "platform_admin.audit.action.embedding_queued"
        }
        "knowledge_asset.embedded" => "platform_admin.audit.action.embedded",
        "knowledge_asset.published" => "platform_admin.audit.action.published",
        "knowledge_asset.archived" | "knowledge_asset.withdrawn" => {
            "platform_admin.audit.action.archived"
        }
        _ => "platform_admin.audit.action.other",
    };
    platform_admin_translation(key, locale)
        .unwrap_or("Governance event")
        .to_string()
}

pub(crate) fn platform_admin_actor_label(role: &str, locale: Locale) -> String {
    let key = match role {
        "PlatformAdmin" => "platform_admin.audit.actor.platform_admin",
        "SchoolManager" => "platform_admin.audit.actor.school_manager",
        "Teacher" => "platform_admin.audit.actor.teacher",
        _ => "platform_admin.audit.actor.system",
    };
    platform_admin_translation(key, locale)
        .unwrap_or("System actor")
        .to_string()
}

pub(crate) fn platform_admin_target_type_label(target_type: &str, locale: Locale) -> String {
    let key = match target_type {
        "knowledge_asset" | "KnowledgeAsset" => "platform_admin.audit.target.asset",
        "knowledge_source_file" | "KnowledgeSourceFile" => "platform_admin.audit.target.source",
        "knowledge_ocr_text" | "KnowledgeOcrText" => "platform_admin.audit.target.ocr",
        _ => "platform_admin.audit.target.other",
    };
    platform_admin_translation(key, locale)
        .unwrap_or("Governance record")
        .to_string()
}

pub(crate) fn platform_admin_lifecycle_guidance(
    status: &str,
    has_verified_ocr: bool,
    locale: Locale,
) -> (String, String) {
    let (title_key, detail_key) = match status {
        "submitted" | "ocr_pending" => (
            "platform_admin.guidance.source.title",
            "platform_admin.guidance.source.detail",
        ),
        "ocr_ready" => (
            "platform_admin.guidance.embed.title",
            "platform_admin.guidance.embed.detail",
        ),
        "embedding_pending" => (
            "platform_admin.guidance.embedding.title",
            "platform_admin.guidance.embedding.detail",
        ),
        "embedded" => (
            "platform_admin.guidance.publish.title",
            "platform_admin.guidance.publish.detail",
        ),
        "published" => (
            "platform_admin.guidance.published.title",
            "platform_admin.guidance.published.detail",
        ),
        "archived" => (
            "platform_admin.guidance.archived.title",
            "platform_admin.guidance.archived.detail",
        ),
        "failed" if has_verified_ocr => (
            "platform_admin.guidance.recovery_ocr.title",
            "platform_admin.guidance.recovery_ocr.detail",
        ),
        "failed" => (
            "platform_admin.guidance.recovery_source.title",
            "platform_admin.guidance.recovery_source.detail",
        ),
        _ => (
            "platform_admin.guidance.unknown.title",
            "platform_admin.guidance.unknown.detail",
        ),
    };
    (
        platform_admin_translation(title_key, locale)
            .unwrap_or("State unavailable")
            .to_string(),
        platform_admin_translation(detail_key, locale)
            .unwrap_or("Refresh the asset list before taking another lifecycle action.")
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEYS: &[&str] = &[
        "platform_admin.review.title",
        "platform_admin.review.description",
        "platform_admin.review.step1.title",
        "platform_admin.review.step1.detail",
        "platform_admin.review.step2.title",
        "platform_admin.review.step2.detail",
        "platform_admin.review.step3.title",
        "platform_admin.review.step3.detail",
        "platform_admin.review.loading",
        "platform_admin.review.load_error",
        "platform_admin.review.empty",
        "platform_admin.metadata.school",
        "platform_admin.metadata.subject",
        "platform_admin.metadata.grade",
        "platform_admin.metadata.language",
        "platform_admin.source.title",
        "platform_admin.source.review",
        "platform_admin.action.update_ocr",
        "platform_admin.action.attach_ocr",
        "platform_admin.action.queue_embedding",
        "platform_admin.action.publish",
        "platform_admin.action.archive",
        "platform_admin.ocr.helper",
        "platform_admin.ocr.provider",
        "platform_admin.ocr.text",
        "platform_admin.audit.title",
        "platform_admin.audit.description",
        "platform_admin.audit.time",
        "platform_admin.audit.actor",
        "platform_admin.audit.action",
        "platform_admin.audit.target",
        "platform_admin.audit.details",
        "platform_admin.audit.view_details",
        "platform_admin.audit.exact_utc",
        "platform_admin.audit.action_code",
        "platform_admin.audit.target_id",
        "platform_admin.audit.structured_details",
    ];

    #[test]
    fn platform_admin_core_keys_have_en_fa_parity() {
        for key in KEYS {
            assert!(
                platform_admin_translation(key, Locale::En).is_some(),
                "missing EN {key}"
            );
            assert!(
                platform_admin_translation(key, Locale::Fa).is_some(),
                "missing FA {key}"
            );
        }
    }

    #[test]
    fn lifecycle_statuses_and_audit_codes_never_render_raw_primary_codes() {
        assert_eq!(
            platform_admin_status_label("ocr_ready", Locale::En),
            "OCR verified"
        );
        assert_eq!(
            platform_admin_status_label("ocr_ready", Locale::Fa),
            "OCR تأییدشده"
        );
        assert_ne!(
            platform_admin_audit_action_label("knowledge_asset.ocr_verified", Locale::En),
            "knowledge_asset.ocr_verified"
        );
        assert_eq!(
            platform_admin_actor_label("PlatformAdmin", Locale::Fa),
            "مدیر سامانه"
        );
    }

    #[test]
    fn language_codes_are_localized_without_mutating_unknown_source_data() {
        assert_eq!(platform_admin_language_label("fa", Locale::En), "Persian");
        assert_eq!(platform_admin_language_label("fa", Locale::Fa), "فارسی");
        assert_eq!(
            platform_admin_language_label("school-defined", Locale::Fa),
            "school-defined"
        );
    }
}
