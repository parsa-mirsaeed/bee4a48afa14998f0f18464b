use super::knowledge_upload;
use crate::i18n::{use_locale, Locale};
use crate::views::role_based::components::DashboardSection;
use api::server_functions::knowledge_asset_edit_functions::{
    list_manager_knowledge_assets_for_editing, update_manager_knowledge_asset_metadata,
    ManagerKnowledgeAssetEditState, UpdateKnowledgeAssetMetadataRequest,
};
use dioxus::prelude::*;
use serde_json::Value;

#[cfg(target_arch = "wasm32")]
use gloo_net::http::Request;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

const SOURCE_REPLACEMENT_FORM_ID: &str = "manager-knowledge-source-replacement-form";

#[derive(Clone, Debug, Default, PartialEq)]
struct KnowledgeEditDraft {
    title: String,
    description: String,
    subject: String,
    grade: String,
    language: String,
    template_type: String,
    tags_json: String,
}

impl From<&ManagerKnowledgeAssetEditState> for KnowledgeEditDraft {
    fn from(asset: &ManagerKnowledgeAssetEditState) -> Self {
        Self {
            title: asset.title.clone(),
            description: asset.description.clone().unwrap_or_default(),
            subject: asset.subject.clone().unwrap_or_default(),
            grade: asset.grade.clone().unwrap_or_default(),
            language: asset.language.clone(),
            template_type: asset.template_type.clone().unwrap_or_default(),
            tags_json: serde_json::to_string_pretty(&asset.tags)
                .unwrap_or_else(|_| "{}".to_string()),
        }
    }
}

#[component]
pub fn ManagerKnowledgeUploadSection() -> Element {
    rsx! {
        div { class: "space-y-8",
            knowledge_upload::ManagerKnowledgeUploadSection {}
            ManagerKnowledgeAssetEditor {}
        }
    }
}

#[component]
fn ManagerKnowledgeAssetEditor() -> Element {
    let locale = use_locale();
    let is_fa = locale.current() == Locale::Fa;
    let mut assets = use_resource(move || async move {
        list_manager_knowledge_assets_for_editing().await
    });
    let mut baseline = use_signal(|| None::<ManagerKnowledgeAssetEditState>);
    let mut draft = use_signal(KnowledgeEditDraft::default);
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(|| None::<(bool, String)>);
    let mut source_epoch = use_signal(|| 0_u64);

    let selected = baseline();
    let dirty = selected
        .as_ref()
        .map(|asset| draft() != KnowledgeEditDraft::from(asset))
        .unwrap_or(false);

    let title = if is_fa { "ویرایش منابع دانشی" } else { "Manage existing assets" };
    let description = if is_fa {
        "فرادادهٔ منبع را با همان شناسه و تاریخچه ویرایش کنید یا یک نسخهٔ جدید PDF جایگزین کنید. وضعیت چرخهٔ عمر فقط در سمت سرور تغییر می‌کند."
    } else {
        "Edit metadata under the same logical asset ID, or append a new PDF source revision. Lifecycle status remains server-owned."
    };

    let save = move |event: FormEvent| {
        event.prevent_default();
        if busy() {
            return;
        }
        let Some(asset) = baseline() else {
            return;
        };
        let current = draft();
        let parsed_tags = serde_json::from_str::<Value>(current.tags_json.trim());
        let Ok(tags) = parsed_tags else {
            notice.set(Some((false, if is_fa {
                "برچسب‌ها/طبقه‌بندی باید یک شیء JSON معتبر باشد.".to_string()
            } else {
                "Tags/classification must be a valid JSON object.".to_string()
            })));
            return;
        };
        if !tags.is_object() {
            notice.set(Some((false, if is_fa {
                "برچسب‌ها/طبقه‌بندی باید یک شیء JSON باشد.".to_string()
            } else {
                "Tags/classification must be a JSON object.".to_string()
            })));
            return;
        }
        if current.title.trim().is_empty() || current.language.trim().is_empty() {
            notice.set(Some((false, if is_fa {
                "عنوان و زبان الزامی هستند.".to_string()
            } else {
                "Title and language are required.".to_string()
            })));
            return;
        }

        let sensitive = retrieval_sensitive_changed(&asset, &current, &tags);
        if sensitive {
            let warning = if is_fa {
                "این تغییر روی بازیابی/بردارها اثر می‌گذارد. اگر منبع پردازش یا منتشر شده باشد، انتشار لغو می‌شود و برای پردازش مجدد بازمی‌گردد. ادامه می‌دهید؟"
            } else {
                "This change affects retrieval metadata. If the asset is embedded or published, it will be withdrawn and returned for reprocessing. Continue?"
            };
            if !browser_confirm(warning) {
                return;
            }
        }

        busy.set(true);
        notice.set(None);
        spawn(async move {
            let result = update_manager_knowledge_asset_metadata(UpdateKnowledgeAssetMetadataRequest {
                asset_id: asset.id.clone(),
                expected_revision: asset.asset_revision,
                title: current.title.trim().to_string(),
                description: normalized_optional(&current.description),
                subject: normalized_optional(&current.subject),
                grade: normalized_optional(&current.grade),
                language: current.language.trim().to_string(),
                template_type: normalized_optional(&current.template_type),
                tags,
            })
            .await;

            match result {
                Ok(result) => {
                    let message = if result.vectors_invalidated {
                        if is_fa {
                            "فراداده ذخیره شد. منبع برای پردازش مجدد از انتشار خارج شد.".to_string()
                        } else {
                            "Metadata saved. The asset was withdrawn for reprocessing.".to_string()
                        }
                    } else if is_fa {
                        "فراداده با همان منبع و چرخهٔ عمر ذخیره شد.".to_string()
                    } else {
                        "Metadata saved with the existing source and lifecycle state.".to_string()
                    };
                    notice.set(Some((true, message)));
                    baseline.set(None);
                    draft.set(KnowledgeEditDraft::default());
                    assets.restart();
                }
                Err(error) => {
                    let error_text = error.to_string();
                    let message = if error_text.contains("changed while you were editing") {
                        if is_fa {
                            "این منبع هم‌زمان تغییر کرده است. فهرست را تازه کنید و دوباره تلاش کنید.".to_string()
                        } else {
                            "This asset changed while you were editing. Refresh the list and try again.".to_string()
                        }
                    } else if is_fa {
                        "ذخیرهٔ تغییرات ممکن نشد. داده‌ها را بررسی و فهرست را تازه کنید.".to_string()
                    } else {
                        "The changes could not be saved. Check the values and refresh the asset list.".to_string()
                    };
                    notice.set(Some((false, message)));
                    assets.restart();
                }
            }
            busy.set(false);
        });
    };

    let cancel = move |_| {
        if dirty {
            let message = if is_fa {
                "تغییرات ذخیره‌نشده کنار گذاشته شوند؟"
            } else {
                "Discard your unsaved changes?"
            };
            if !browser_confirm(message) {
                return;
            }
        }
        baseline.set(None);
        draft.set(KnowledgeEditDraft::default());
        notice.set(None);
    };

    let replace_source = move |event: FormEvent| {
        event.prevent_default();
        if busy() {
            return;
        }
        let Some(asset) = baseline() else {
            return;
        };
        if asset.status == "archived" {
            notice.set(Some((false, if is_fa {
                "منبع بایگانی‌شده نسخهٔ منبع جدید نمی‌پذیرد.".to_string()
            } else {
                "Archived assets cannot receive a new source revision.".to_string()
            })));
            return;
        }
        let warning = if is_fa {
            "جایگزینی PDF یک نسخهٔ منبع تغییرناپذیر جدید می‌سازد، OCR/بردارهای قبلی را از حالت جاری خارج می‌کند و انتشار را لغو می‌کند. ادامه می‌دهید؟"
        } else {
            "Replacing the PDF creates a new immutable source revision, retires current OCR/vectors, and withdraws publication. Continue?"
        };
        if !browser_confirm(warning) {
            return;
        }

        busy.set(true);
        notice.set(None);

        #[cfg(target_arch = "wasm32")]
        {
            let form = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.get_element_by_id(SOURCE_REPLACEMENT_FORM_ID))
                .and_then(|element| element.dyn_into::<web_sys::HtmlFormElement>().ok())
                .and_then(|form| web_sys::FormData::new_with_form(&form).ok());

            let Some(form) = form else {
                busy.set(false);
                notice.set(Some((false, if is_fa {
                    "فرم جایگزینی PDF خوانده نشد. صفحه را تازه کنید.".to_string()
                } else {
                    "The replacement form could not be read. Refresh the page.".to_string()
                })));
                return;
            };

            spawn(async move {
                let response = Request::post("/api/manager/knowledge-submissions/upload").body(form);
                match response {
                    Ok(request) => match request.send().await {
                        Ok(response) if (200..300).contains(&response.status()) => {
                            notice.set(Some((true, if is_fa {
                                "نسخهٔ جدید PDF ثبت شد. منبع برای بررسی و پردازش مجدد آماده است.".to_string()
                            } else {
                                "New PDF source revision registered. The asset is ready for governed review and reprocessing.".to_string()
                            })));
                            source_epoch.set(source_epoch() + 1);
                            baseline.set(None);
                            draft.set(KnowledgeEditDraft::default());
                            assets.restart();
                        }
                        Ok(response) => {
                            notice.set(Some((false, replacement_error_message(response.status(), is_fa))));
                            assets.restart();
                        }
                        Err(_) => notice.set(Some((false, if is_fa {
                            "سرویس ذخیره‌سازی در دسترس نیست؛ نسخهٔ فعلی تغییر نکرده است.".to_string()
                        } else {
                            "Storage is unavailable; the current source revision was not changed.".to_string()
                        }))),
                    },
                    Err(_) => notice.set(Some((false, if is_fa {
                        "آماده‌سازی فایل جایگزین ممکن نشد.".to_string()
                    } else {
                        "The replacement upload could not be prepared.".to_string()
                    }))),
                }
                busy.set(false);
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = asset;
            busy.set(false);
            notice.set(Some((false, "PDF replacement is available in the browser application.".to_string())));
        }
    };

    rsx! {
        DashboardSection {
            title: title.to_string(),
            description: Some(description.to_string()),
            children: rsx! {
                div { class: "space-y-5",
                    if let Some((success, message)) = notice() {
                        div {
                            class: if success { "et-ui-alert et-ui-tone--success" } else { "et-ui-alert et-ui-tone--danger" },
                            role: if success { "status" } else { "alert" },
                            "aria-live": if success { "polite" } else { "assertive" },
                            "{message}"
                        }
                    }

                    match assets.read().as_ref() {
                        None => rsx! { p { class: "text-sm text-gray-500", if is_fa { "در حال بارگذاری منابع…" } else { "Loading assets…" } } },
                        Some(Err(_)) => rsx! {
                            div { class: "et-state-panel et-state-panel--error",
                                p { if is_fa { "فهرست منابع بارگذاری نشد." } else { "Assets could not be loaded." } }
                                button { class: "et-inline-action mt-2", onclick: move |_| assets.restart(), if is_fa { "تلاش دوباره" } else { "Try again" } }
                            }
                        },
                        Some(Ok(items)) if items.is_empty() => rsx! {
                            div { class: "et-ui-data-state",
                                p { if is_fa { "هنوز منبعی برای ویرایش وجود ندارد." } else { "There are no assets to edit yet." } }
                            }
                        },
                        Some(Ok(items)) => rsx! {
                            div { class: "grid grid-cols-1 gap-3 lg:grid-cols-2",
                                for item in items.iter() {
                                    {
                                        let item_for_click = item.clone();
                                        let selected_id = selected.as_ref().map(|value| value.id.as_str());
                                        let active = selected_id == Some(item.id.as_str());
                                        rsx! {
                                            article {
                                                key: "edit-card-{item.id}",
                                                class: if active { "et-ui-card ring-2 ring-primary" } else { "et-ui-card" },
                                                div { class: "flex flex-wrap items-start justify-between gap-3",
                                                    div {
                                                        h3 { class: "font-semibold text-gray-900 dark:text-white", "{item.title}" }
                                                        p { class: "mt-1 text-xs text-gray-500 dark:text-gray-400",
                                                            if is_fa { "وضعیت: {item.status} · نسخه: {item.asset_revision}" } else { "Status: {item.status} · Revision: {item.asset_revision}" }
                                                        }
                                                    }
                                                    button {
                                                        class: "et-ui-button et-ui-button--secondary et-ui-button--sm",
                                                        r#type: "button",
                                                        disabled: busy(),
                                                        onclick: move |_| {
                                                            if dirty {
                                                                let message = if is_fa { "تغییرات ذخیره‌نشده کنار گذاشته و منبع دیگری باز شود؟" } else { "Discard unsaved changes and edit another asset?" };
                                                                if !browser_confirm(message) { return; }
                                                            }
                                                            draft.set(KnowledgeEditDraft::from(&item_for_click));
                                                            baseline.set(Some(item_for_click.clone()));
                                                            notice.set(None);
                                                        },
                                                        if is_fa { "ویرایش" } else { "Edit" }
                                                    }
                                                }
                                                if let Some(filename) = item.current_source_filename.as_ref() {
                                                    p { class: "mt-2 text-xs text-gray-500 dark:text-gray-400", if is_fa { "منبع جاری: {filename}" } else { "Current source: {filename}" } }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                    }

                    if let Some(asset) = selected.as_ref() {
                        form {
                            class: "et-ui-card space-y-5",
                            onsubmit: save,
                            div { class: "flex flex-wrap items-start justify-between gap-3",
                                div {
                                    h3 { class: "text-lg font-semibold text-gray-900 dark:text-white", if is_fa { "جزئیات منبع" } else { "Asset details" } }
                                    p { class: "mt-1 text-sm text-gray-500 dark:text-gray-400",
                                        if is_fa { "شناسهٔ منطقی ثابت می‌ماند. وضعیت چرخهٔ عمر توسط سرور کنترل می‌شود." } else { "The logical asset ID stays fixed. Lifecycle status is controlled by the server." }
                                    }
                                }
                                span { class: "rounded-full bg-gray-100 dark:bg-gray-800 px-3 py-1 text-xs", "{asset.status}" }
                            }

                            div { class: "grid grid-cols-1 gap-4 md:grid-cols-2",
                                EditInput { label: if is_fa { "عنوان" } else { "Title" }, value: draft().title, required: true, oninput: move |value| { let mut next = draft(); next.title = value; draft.set(next); } }
                                EditInput { label: if is_fa { "زبان" } else { "Language" }, value: draft().language, required: true, oninput: move |value| { let mut next = draft(); next.language = value; draft.set(next); } }
                                EditInput { label: if is_fa { "موضوع" } else { "Subject" }, value: draft().subject, required: false, oninput: move |value| { let mut next = draft(); next.subject = value; draft.set(next); } }
                                EditInput { label: if is_fa { "پایه" } else { "Grade" }, value: draft().grade, required: false, oninput: move |value| { let mut next = draft(); next.grade = value; draft.set(next); } }
                                EditInput { label: if is_fa { "نوع/قالب" } else { "Template/type" }, value: draft().template_type, required: false, oninput: move |value| { let mut next = draft(); next.template_type = value; draft.set(next); } }
                            }
                            div {
                                label { class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1", if is_fa { "توضیحات" } else { "Description" } }
                                textarea {
                                    class: "et-ui-textarea",
                                    rows: "4",
                                    maxlength: "8000",
                                    value: "{draft().description}",
                                    oninput: move |event| { let mut next = draft(); next.description = event.value(); draft.set(next); },
                                }
                                p { class: "mt-1 text-xs text-gray-500 dark:text-gray-400", if is_fa { "ویرایش توضیحات به‌تنهایی انتشار را باطل نمی‌کند." } else { "A description-only edit does not invalidate publication." } }
                            }
                            div {
                                label { class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1", if is_fa { "برچسب‌ها / طبقه‌بندی (JSON)" } else { "Tags / classification (JSON)" } }
                                textarea {
                                    class: "et-ui-textarea font-mono text-sm",
                                    rows: "4",
                                    value: "{draft().tags_json}",
                                    oninput: move |event| { let mut next = draft(); next.tags_json = event.value(); draft.set(next); },
                                }
                            }
                            div { class: "et-ui-alert et-ui-tone--neutral",
                                if is_fa {
                                    "تغییر عنوان، موضوع، پایه، زبان، نوع/قالب یا طبقه‌بندی ممکن است بردارها را قدیمی کند؛ سامانه در این حالت منبع را از انتشار خارج می‌کند."
                                } else {
                                    "Changing title, subject, grade, language, template/type, or classification may stale vector metadata; the server withdraws the asset when reprocessing is required."
                                }
                            }
                            div { class: "flex flex-wrap gap-3",
                                button { class: "et-ui-button et-ui-button--primary et-ui-button--md", r#type: "submit", disabled: busy() || !dirty, if busy() { if is_fa { "در حال ذخیره…" } else { "Saving…" } } else { if is_fa { "ذخیره" } else { "Save" } } }
                                button { class: "et-ui-button et-ui-button--secondary et-ui-button--md", r#type: "button", disabled: busy(), onclick: cancel, if is_fa { "لغو" } else { "Cancel" } }
                                if dirty {
                                    span { class: "self-center text-xs font-medium text-amber-700 dark:text-amber-300", if is_fa { "تغییرات ذخیره‌نشده" } else { "Unsaved changes" } }
                                }
                            }
                        }

                        form {
                            key: "source-replacement-{source_epoch}-{asset.id}",
                            id: SOURCE_REPLACEMENT_FORM_ID,
                            class: "et-ui-card space-y-4",
                            enctype: "multipart/form-data",
                            onsubmit: replace_source,
                            h3 { class: "text-lg font-semibold text-gray-900 dark:text-white", if is_fa { "سند منبع" } else { "Source document" } }
                            p { class: "text-sm text-gray-500 dark:text-gray-400",
                                if is_fa { "جایگزینی، فایل قبلی را بازنویسی نمی‌کند؛ یک نسخهٔ تغییرناپذیر جدید برای همان شناسهٔ منبع می‌سازد." } else { "Replacement never overwrites the previous object; it appends a new immutable source revision under the same asset ID." }
                            }
                            dl { class: "grid grid-cols-1 gap-2 text-sm md:grid-cols-2",
                                div { dt { class: "font-medium", if is_fa { "فایل جاری" } else { "Current file" } } dd { class: "break-all text-gray-500", "{asset.current_source_filename.clone().unwrap_or_else(|| \"—\".to_string())}" } }
                                div { dt { class: "font-medium", "SHA-256" } dd { class: "break-all text-gray-500", "{asset.current_source_sha256.clone().unwrap_or_else(|| \"—\".to_string())}" } }
                            }
                            input { r#type: "hidden", name: "asset_id", value: "{asset.id}" }
                            input { r#type: "hidden", name: "expected_revision", value: "{asset.asset_revision}" }
                            div {
                                label { r#for: "knowledge-source-replacement-file", class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1", if is_fa { "PDF جایگزین" } else { "Replacement PDF" } }
                                input {
                                    id: "knowledge-source-replacement-file",
                                    class: "et-ui-input",
                                    r#type: "file",
                                    name: "file",
                                    accept: "application/pdf,.pdf",
                                    disabled: busy() || asset.status == "archived",
                                    "aria-describedby": "knowledge-source-replacement-help",
                                }
                                p { id: "knowledge-source-replacement-help", class: "mt-1 text-xs text-gray-500 dark:text-gray-400", if is_fa { "حداکثر ۲۰ MiB؛ فقط PDF. این اقدام OCR و بردارهای جاری را قدیمی می‌کند." } else { "Maximum 20 MiB; PDF only. This action retires current OCR and vector state." } }
                            }
                            button {
                                class: "et-ui-button et-ui-button--danger et-ui-button--md",
                                r#type: "submit",
                                disabled: busy() || asset.status == "archived",
                                if is_fa { "جایگزینی سند منبع" } else { "Replace source document" }
                            }
                            if asset.status == "archived" {
                                p { class: "text-sm text-gray-500", if is_fa { "منبع بایگانی‌شده پایانی است و نسخهٔ منبع جدید نمی‌پذیرد." } else { "Archived is terminal; no new source revision can be appended." } }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn EditInput(
    label: &'static str,
    value: String,
    required: bool,
    oninput: EventHandler<String>,
) -> Element {
    rsx! {
        div {
            label { class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1", "{label}" if required { " *" } }
            input {
                class: "et-ui-input",
                r#type: "text",
                value: "{value}",
                "aria-required": required,
                oninput: move |event| oninput.call(event.value()),
            }
        }
    }
}

fn normalized_optional(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() { None } else { Some(value.to_string()) }
}

fn retrieval_sensitive_changed(
    baseline: &ManagerKnowledgeAssetEditState,
    draft: &KnowledgeEditDraft,
    tags: &Value,
) -> bool {
    baseline.title.trim() != draft.title.trim()
        || baseline.subject.as_deref().unwrap_or("").trim() != draft.subject.trim()
        || baseline.grade.as_deref().unwrap_or("").trim() != draft.grade.trim()
        || baseline.language.trim() != draft.language.trim()
        || baseline.template_type.as_deref().unwrap_or("").trim() != draft.template_type.trim()
        || &baseline.tags != tags
}

fn browser_confirm(message: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|window| window.confirm_with_message(message).ok())
            .unwrap_or(false)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = message;
        true
    }
}

fn replacement_error_message(status: u16, is_fa: bool) -> String {
    if is_fa {
        match status {
            400 => "فایل یا نسخهٔ منبع نامعتبر است.".to_string(),
            401 | 403 => "اجازهٔ جایگزینی این منبع را ندارید.".to_string(),
            409 => "این منبع هم‌زمان تغییر کرده است؛ فهرست را تازه کنید.".to_string(),
            413 => "PDF از حد ۲۰ MiB بزرگ‌تر است.".to_string(),
            415 => "فایل انتخاب‌شده PDF کامل نیست.".to_string(),
            _ => "نسخهٔ جدید منبع ثبت نشد؛ نسخهٔ جاری را تازه کنید و دوباره تلاش کنید.".to_string(),
        }
    } else {
        match status {
            400 => "The file or asset revision is invalid.".to_string(),
            401 | 403 => "You are not allowed to replace this asset source.".to_string(),
            409 => "This asset changed concurrently; refresh the asset list.".to_string(),
            413 => "The PDF exceeds the 20 MiB limit.".to_string(),
            415 => "The selected file is not a complete PDF.".to_string(),
            _ => "The new source revision was not registered; refresh the asset and try again.".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset() -> ManagerKnowledgeAssetEditState {
        ManagerKnowledgeAssetEditState {
            id: "asset".to_string(),
            title: "Title".to_string(),
            description: Some("Description".to_string()),
            subject: Some("Math".to_string()),
            grade: Some("8".to_string()),
            language: "en".to_string(),
            template_type: None,
            tags: serde_json::json!({"kind":"guide"}),
            status: "published".to_string(),
            asset_revision: 7,
            current_source_file_id: Some("source".to_string()),
            current_source_filename: Some("guide.pdf".to_string()),
            current_source_sha256: Some("a".repeat(64)),
        }
    }

    #[test]
    fn description_only_is_not_retrieval_sensitive() {
        let baseline = asset();
        let mut draft = KnowledgeEditDraft::from(&baseline);
        draft.description = "New presentation copy".to_string();
        assert!(!retrieval_sensitive_changed(&baseline, &draft, &baseline.tags));
    }

    #[test]
    fn vector_payload_metadata_is_retrieval_sensitive() {
        let baseline = asset();
        let mut draft = KnowledgeEditDraft::from(&baseline);
        draft.subject = "Physics".to_string();
        assert!(retrieval_sensitive_changed(&baseline, &draft, &baseline.tags));
    }
}
