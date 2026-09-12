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

const SOURCE_FORM_ID: &str = "manager-knowledge-source-replacement-form";

#[derive(Clone, Debug, Default, PartialEq)]
struct Draft {
    title: String,
    description: String,
    subject: String,
    grade: String,
    language: String,
    template_type: String,
    tags_json: String,
}

impl From<&ManagerKnowledgeAssetEditState> for Draft {
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
    let fa = locale.current() == Locale::Fa;
    let mut assets =
        use_resource(move || async move { list_manager_knowledge_assets_for_editing().await });
    let mut selected = use_signal(|| None::<ManagerKnowledgeAssetEditState>);
    let mut draft = use_signal(Draft::default);
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(|| None::<(bool, String)>);
    let mut source_epoch = use_signal(|| 0_u64);

    let current = selected();
    let dirty = current
        .as_ref()
        .map(|asset| draft() != Draft::from(asset))
        .unwrap_or(false);

    let section_title = if fa {
        "ویرایش منابع دانشی"
    } else {
        "Manage existing assets"
    };
    let section_description = if fa {
        "فراداده را با همان شناسه و تاریخچه ویرایش کنید یا یک نسخهٔ جدید PDF بسازید. وضعیت چرخهٔ عمر فقط در سمت سرور تغییر می‌کند."
    } else {
        "Edit metadata under the same logical asset ID, or append a new PDF source revision. Lifecycle status remains server-owned."
    };

    let save = move |event: FormEvent| {
        event.prevent_default();
        if busy() {
            return;
        }
        let Some(asset) = selected() else {
            return;
        };
        let values = draft();
        if values.title.trim().is_empty() || values.language.trim().is_empty() {
            notice.set(Some((false, required_message(fa).to_string())));
            return;
        }
        let Ok(tags) = serde_json::from_str::<Value>(values.tags_json.trim()) else {
            notice.set(Some((false, json_message(fa).to_string())));
            return;
        };
        if !tags.is_object() {
            notice.set(Some((false, json_message(fa).to_string())));
            return;
        }
        if retrieval_sensitive_changed(&asset, &values, &tags) && !confirm(retrieval_warning(fa)) {
            return;
        }

        busy.set(true);
        notice.set(None);
        spawn(async move {
            let result =
                update_manager_knowledge_asset_metadata(UpdateKnowledgeAssetMetadataRequest {
                    asset_id: asset.id,
                    expected_revision: asset.asset_revision,
                    title: values.title.trim().to_string(),
                    description: optional(&values.description),
                    subject: optional(&values.subject),
                    grade: optional(&values.grade),
                    language: values.language.trim().to_string(),
                    template_type: optional(&values.template_type),
                    tags,
                })
                .await;

            match result {
                Ok(result) => {
                    let message = if result.vectors_invalidated {
                        if fa {
                            "فراداده ذخیره شد. منبع برای پردازش مجدد از انتشار خارج شد."
                        } else {
                            "Metadata saved. The asset was withdrawn for reprocessing."
                        }
                    } else if fa {
                        "فراداده با همان منبع و وضعیت چرخهٔ عمر ذخیره شد."
                    } else {
                        "Metadata saved with the existing source and lifecycle state."
                    };
                    notice.set(Some((true, message.to_string())));
                    selected.set(None);
                    draft.set(Draft::default());
                    assets.restart();
                }
                Err(error) => {
                    let text = error.to_string();
                    let message = if text.contains("changed while") {
                        conflict_message(fa)
                    } else {
                        save_error_message(fa)
                    };
                    notice.set(Some((false, message.to_string())));
                    assets.restart();
                }
            }
            busy.set(false);
        });
    };

    let cancel = move |_| {
        if dirty && !confirm(discard_warning(fa)) {
            return;
        }
        selected.set(None);
        draft.set(Draft::default());
        notice.set(None);
    };

    let replace_source = move |event: FormEvent| {
        event.prevent_default();
        if busy() {
            return;
        }
        let Some(asset) = selected() else {
            return;
        };
        if asset.status == "archived" {
            notice.set(Some((false, archived_message(fa).to_string())));
            return;
        }
        if !confirm(replacement_warning(fa)) {
            return;
        }

        busy.set(true);
        notice.set(None);
        #[cfg(target_arch = "wasm32")]
        {
            let form = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.get_element_by_id(SOURCE_FORM_ID))
                .and_then(|element| element.dyn_into::<web_sys::HtmlFormElement>().ok())
                .and_then(|form| web_sys::FormData::new_with_form(&form).ok());
            let Some(form) = form else {
                notice.set(Some((false, source_form_error(fa).to_string())));
                busy.set(false);
                return;
            };
            spawn(async move {
                let response =
                    Request::post("/api/manager/knowledge-submissions/upload").body(form);
                match response {
                    Ok(request) => match request.send().await {
                        Ok(response) if (200..300).contains(&response.status()) => {
                            notice.set(Some((true, replacement_success(fa).to_string())));
                            source_epoch.set(source_epoch() + 1);
                            selected.set(None);
                            draft.set(Draft::default());
                            assets.restart();
                        }
                        Ok(response) => {
                            notice.set(Some((
                                false,
                                replacement_error_message(response.status(), fa),
                            )));
                            assets.restart();
                        }
                        Err(_) => {
                            notice.set(Some((false, storage_error(fa).to_string())));
                        }
                    },
                    Err(_) => {
                        notice.set(Some((false, source_form_error(fa).to_string())));
                    }
                }
                busy.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = asset;
            busy.set(false);
            notice.set(Some((
                false,
                "PDF replacement is available in the browser application.".to_string(),
            )));
        }
    };

    rsx! {
        DashboardSection {
            title: section_title.to_string(),
            description: Some(section_description.to_string()),
            children: rsx! {
                div { class: "space-y-5",
                    if let Some((success, message)) = notice() {
                        p {
                            class: if success { "et-ui-alert et-ui-tone--success" } else { "et-ui-alert et-ui-tone--danger" },
                            role: if success { "status" } else { "alert" },
                            "{message}"
                        }
                    }

                    match assets.read().as_ref() {
                        None => rsx! { p { class: "text-sm text-gray-500", "{loading_text(fa)}" } },
                        Some(Err(_)) => rsx! {
                            div { class: "et-state-panel et-state-panel--error",
                                p { "{load_error_text(fa)}" }
                                button { class: "et-inline-action mt-2", onclick: move |_| assets.restart(), "{retry_text(fa)}" }
                            }
                        },
                        Some(Ok(items)) if items.is_empty() => rsx! {
                            div { class: "et-ui-data-state", p { "{empty_text(fa)}" } }
                        },
                        Some(Ok(items)) => rsx! {
                            div { class: "grid grid-cols-1 gap-3 lg:grid-cols-2",
                                for item in items.iter() {
                                    {
                                        let item_for_click = item.clone();
                                        let active = current.as_ref().map(|value| value.id.as_str()) == Some(item.id.as_str());
                                        rsx! {
                                            article { key: "asset-{item.id}", class: if active { "et-ui-card ring-2 ring-primary" } else { "et-ui-card" },
                                                div { class: "flex flex-wrap items-start justify-between gap-3",
                                                    div {
                                                        h3 { class: "font-semibold text-gray-900 dark:text-white", "{item.title}" }
                                                        p { class: "mt-1 text-xs text-gray-500 dark:text-gray-400", "{status_prefix(fa)} {item.status} · {revision_prefix(fa)} {item.asset_revision}" }
                                                    }
                                                    button {
                                                        class: "et-ui-button et-ui-button--secondary et-ui-button--sm",
                                                        r#type: "button",
                                                        disabled: busy(),
                                                        onclick: move |_| {
                                                            if dirty && !confirm(switch_warning(fa)) { return; }
                                                            draft.set(Draft::from(&item_for_click));
                                                            selected.set(Some(item_for_click.clone()));
                                                            notice.set(None);
                                                        },
                                                        "{edit_text(fa)}"
                                                    }
                                                }
                                                if let Some(filename) = item.current_source_filename.as_ref() {
                                                    p { class: "mt-2 text-xs text-gray-500 dark:text-gray-400", "{current_source_prefix(fa)} {filename}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                    }

                    if let Some(asset) = current.as_ref() {
                        form { class: "et-ui-card space-y-5", onsubmit: save,
                            div { class: "flex flex-wrap items-start justify-between gap-3",
                                div {
                                    h3 { class: "text-lg font-semibold text-gray-900 dark:text-white", "{details_text(fa)}" }
                                    p { class: "mt-1 text-sm text-gray-500 dark:text-gray-400", "{server_status_text(fa)}" }
                                }
                                span { class: "rounded-full bg-gray-100 dark:bg-gray-800 px-3 py-1 text-xs", "{asset.status}" }
                            }
                            div { class: "grid grid-cols-1 gap-4 md:grid-cols-2",
                                Field { label: title_text(fa), value: draft().title, required: true, oninput: move |value| { let mut next=draft(); next.title=value; draft.set(next); } }
                                Field { label: language_text(fa), value: draft().language, required: true, oninput: move |value| { let mut next=draft(); next.language=value; draft.set(next); } }
                                Field { label: subject_text(fa), value: draft().subject, required: false, oninput: move |value| { let mut next=draft(); next.subject=value; draft.set(next); } }
                                Field { label: grade_text(fa), value: draft().grade, required: false, oninput: move |value| { let mut next=draft(); next.grade=value; draft.set(next); } }
                                Field { label: template_text(fa), value: draft().template_type, required: false, oninput: move |value| { let mut next=draft(); next.template_type=value; draft.set(next); } }
                            }
                            div {
                                label { class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1", "{description_text(fa)}" }
                                textarea { class: "et-ui-textarea", rows: "4", maxlength: "8000", value: "{draft().description}", oninput: move |event| { let mut next=draft(); next.description=event.value(); draft.set(next); } }
                                p { class: "mt-1 text-xs text-gray-500 dark:text-gray-400", "{description_help(fa)}" }
                            }
                            div {
                                label { class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1", "{tags_text(fa)}" }
                                textarea { class: "et-ui-textarea font-mono text-sm", rows: "4", value: "{draft().tags_json}", oninput: move |event| { let mut next=draft(); next.tags_json=event.value(); draft.set(next); } }
                            }
                            div { class: "et-ui-alert et-ui-tone--neutral", "{retrieval_help(fa)}" }
                            div { class: "flex flex-wrap gap-3",
                                button { class: "et-ui-button et-ui-button--primary et-ui-button--md", r#type: "submit", disabled: busy() || !dirty, "{save_text(fa)}" }
                                button { class: "et-ui-button et-ui-button--secondary et-ui-button--md", r#type: "button", disabled: busy(), onclick: cancel, "{cancel_text(fa)}" }
                                if dirty { span { class: "self-center text-xs font-medium text-amber-700 dark:text-amber-300", "{unsaved_text(fa)}" } }
                            }
                        }

                        form {
                            id: SOURCE_FORM_ID,
                            class: "et-ui-card space-y-4",
                            enctype: "multipart/form-data",
                            onsubmit: replace_source,
                            h3 { class: "text-lg font-semibold text-gray-900 dark:text-white", "{source_document_text(fa)}" }
                            p { class: "text-sm text-gray-500 dark:text-gray-400", "{source_help(fa)}" }
                            dl { class: "grid grid-cols-1 gap-2 text-sm md:grid-cols-2",
                                div {
                                    dt { class: "font-medium", "{current_file_text(fa)}" }
                                    if let Some(filename) = asset.current_source_filename.as_ref() {
                                        dd { class: "break-all text-gray-500", "{filename}" }
                                    } else {
                                        dd { class: "break-all text-gray-500", "—" }
                                    }
                                }
                                div {
                                    dt { class: "font-medium", "SHA-256" }
                                    if let Some(hash) = asset.current_source_sha256.as_ref() {
                                        dd { class: "break-all text-gray-500", "{hash}" }
                                    } else {
                                        dd { class: "break-all text-gray-500", "—" }
                                    }
                                }
                            }
                            input { r#type: "hidden", name: "asset_id", value: "{asset.id}" }
                            input { r#type: "hidden", name: "expected_revision", value: "{asset.asset_revision}" }
                            div {
                                label { r#for: "knowledge-source-replacement-file", class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1", "{replacement_pdf_text(fa)}" }
                                input { id: "knowledge-source-replacement-file", class: "et-ui-input", r#type: "file", name: "file", accept: "application/pdf,.pdf", disabled: busy() || asset.status == "archived" }
                                p { class: "mt-1 text-xs text-gray-500 dark:text-gray-400", "{replacement_help(fa)}" }
                            }
                            button { class: "et-ui-button et-ui-button--danger et-ui-button--md", r#type: "submit", disabled: busy() || asset.status == "archived", "{replace_text(fa)}" }
                            if asset.status == "archived" { p { class: "text-sm text-gray-500", "{archived_message(fa)}" } }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Field(
    label: &'static str,
    value: String,
    required: bool,
    oninput: EventHandler<String>,
) -> Element {
    rsx! {
        div {
            label { class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1", "{label}" if required { " *" } }
            input { class: "et-ui-input", r#type: "text", value: "{value}", "aria-required": required, oninput: move |event| oninput.call(event.value()) }
        }
    }
}

fn optional(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}
fn retrieval_sensitive_changed(
    asset: &ManagerKnowledgeAssetEditState,
    draft: &Draft,
    tags: &Value,
) -> bool {
    asset.title.trim() != draft.title.trim()
        || asset.subject.as_deref().unwrap_or("").trim() != draft.subject.trim()
        || asset.grade.as_deref().unwrap_or("").trim() != draft.grade.trim()
        || asset.language.trim() != draft.language.trim()
        || asset.template_type.as_deref().unwrap_or("").trim() != draft.template_type.trim()
        || &asset.tags != tags
}
fn confirm(message: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.confirm_with_message(message).ok())
            .unwrap_or(false)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = message;
        true
    }
}
fn replacement_error_message(status: u16, fa: bool) -> String {
    match (fa, status) {
        (true, 409) => "این منبع هم‌زمان تغییر کرده است؛ فهرست را تازه کنید.".into(),
        (false, 409) => "This asset changed concurrently; refresh the asset list.".into(),
        (true, 413) => "PDF از حد ۲۰ MiB بزرگ‌تر است.".into(),
        (false, 413) => "The PDF exceeds the 20 MiB limit.".into(),
        (true, 415) => "فایل انتخاب‌شده PDF کامل نیست.".into(),
        (false, 415) => "The selected file is not a complete PDF.".into(),
        (true, _) => "نسخهٔ جدید منبع ثبت نشد؛ دوباره تلاش کنید.".into(),
        (false, _) => "The new source revision was not registered; refresh and try again.".into(),
    }
}

fn required_message(f: bool) -> &'static str {
    if f {
        "عنوان و زبان الزامی هستند."
    } else {
        "Title and language are required."
    }
}
fn json_message(f: bool) -> &'static str {
    if f {
        "برچسب‌ها/طبقه‌بندی باید یک شیء JSON معتبر باشد."
    } else {
        "Tags/classification must be a valid JSON object."
    }
}
fn retrieval_warning(f: bool) -> &'static str {
    if f {
        "این تغییر روی بازیابی/بردارها اثر می‌گذارد. اگر منبع منتشر شده باشد، انتشار لغو و پردازش مجدد لازم می‌شود. ادامه می‌دهید؟"
    } else {
        "This change affects retrieval metadata. If the asset is embedded or published, it will be withdrawn and returned for reprocessing. Continue?"
    }
}
fn discard_warning(f: bool) -> &'static str {
    if f {
        "تغییرات ذخیره‌نشده کنار گذاشته شوند؟"
    } else {
        "Discard your unsaved changes?"
    }
}
fn switch_warning(f: bool) -> &'static str {
    if f {
        "تغییرات ذخیره‌نشده کنار گذاشته و منبع دیگری باز شود؟"
    } else {
        "Discard unsaved changes and edit another asset?"
    }
}
fn replacement_warning(f: bool) -> &'static str {
    if f {
        "جایگزینی PDF یک نسخهٔ منبع تغییرناپذیر جدید می‌سازد، OCR/بردارهای جاری را قدیمی و انتشار را لغو می‌کند. ادامه می‌دهید؟"
    } else {
        "Replacing the PDF creates a new immutable source revision, retires current OCR/vectors, and withdraws publication. Continue?"
    }
}
fn archived_message(f: bool) -> &'static str {
    if f {
        "منبع بایگانی‌شده پایانی است و نسخهٔ منبع جدید نمی‌پذیرد."
    } else {
        "Archived is terminal; no new source revision can be appended."
    }
}
fn conflict_message(f: bool) -> &'static str {
    if f {
        "این منبع هم‌زمان تغییر کرده است. فهرست را تازه کنید."
    } else {
        "This asset changed while you were editing. Refresh the list and try again."
    }
}
fn save_error_message(f: bool) -> &'static str {
    if f {
        "ذخیرهٔ تغییرات ممکن نشد. داده‌ها را بررسی کنید."
    } else {
        "The changes could not be saved. Check the values and refresh the asset list."
    }
}
fn source_form_error(f: bool) -> &'static str {
    if f {
        "فرم جایگزینی PDF خوانده نشد."
    } else {
        "The replacement upload could not be prepared."
    }
}
fn replacement_success(f: bool) -> &'static str {
    if f {
        "نسخهٔ جدید PDF ثبت شد. منبع برای بررسی و پردازش مجدد آماده است."
    } else {
        "New PDF source revision registered. The asset is ready for governed review and reprocessing."
    }
}
fn storage_error(f: bool) -> &'static str {
    if f {
        "سرویس ذخیره‌سازی در دسترس نیست؛ نسخهٔ فعلی تغییر نکرده است."
    } else {
        "Storage is unavailable; the current source revision was not changed."
    }
}
fn loading_text(f: bool) -> &'static str {
    if f {
        "در حال بارگذاری منابع…"
    } else {
        "Loading assets…"
    }
}
fn load_error_text(f: bool) -> &'static str {
    if f {
        "فهرست منابع بارگذاری نشد."
    } else {
        "Assets could not be loaded."
    }
}
fn retry_text(f: bool) -> &'static str {
    if f {
        "تلاش دوباره"
    } else {
        "Try again"
    }
}
fn empty_text(f: bool) -> &'static str {
    if f {
        "هنوز منبعی برای ویرایش وجود ندارد."
    } else {
        "There are no assets to edit yet."
    }
}
fn status_prefix(f: bool) -> &'static str {
    if f {
        "وضعیت:"
    } else {
        "Status:"
    }
}
fn revision_prefix(f: bool) -> &'static str {
    if f {
        "نسخه:"
    } else {
        "Revision:"
    }
}
fn current_source_prefix(f: bool) -> &'static str {
    if f {
        "منبع جاری:"
    } else {
        "Current source:"
    }
}
fn edit_text(f: bool) -> &'static str {
    if f {
        "ویرایش"
    } else {
        "Edit"
    }
}
fn details_text(f: bool) -> &'static str {
    if f {
        "جزئیات منبع"
    } else {
        "Asset details"
    }
}
fn server_status_text(f: bool) -> &'static str {
    if f {
        "شناسهٔ منطقی ثابت می‌ماند. وضعیت چرخهٔ عمر توسط سرور کنترل می‌شود."
    } else {
        "The logical asset ID stays fixed. Lifecycle status is controlled by the server."
    }
}
fn title_text(f: bool) -> &'static str {
    if f {
        "عنوان"
    } else {
        "Title"
    }
}
fn language_text(f: bool) -> &'static str {
    if f {
        "زبان"
    } else {
        "Language"
    }
}
fn subject_text(f: bool) -> &'static str {
    if f {
        "موضوع"
    } else {
        "Subject"
    }
}
fn grade_text(f: bool) -> &'static str {
    if f {
        "پایه"
    } else {
        "Grade"
    }
}
fn template_text(f: bool) -> &'static str {
    if f {
        "نوع/قالب"
    } else {
        "Template/type"
    }
}
fn description_text(f: bool) -> &'static str {
    if f {
        "توضیحات"
    } else {
        "Description"
    }
}
fn description_help(f: bool) -> &'static str {
    if f {
        "ویرایش توضیحات به‌تنهایی انتشار را باطل نمی‌کند."
    } else {
        "A description-only edit does not invalidate publication."
    }
}
fn tags_text(f: bool) -> &'static str {
    if f {
        "برچسب‌ها / طبقه‌بندی (JSON)"
    } else {
        "Tags / classification (JSON)"
    }
}
fn retrieval_help(f: bool) -> &'static str {
    if f {
        "تغییر عنوان، موضوع، پایه، زبان، نوع/قالب یا طبقه‌بندی ممکن است بردارها را قدیمی کند؛ سامانه در این حالت منبع را از انتشار خارج می‌کند."
    } else {
        "Changing title, subject, grade, language, template/type, or classification may stale vector metadata; the server withdraws the asset when reprocessing is required."
    }
}
fn save_text(f: bool) -> &'static str {
    if f {
        "ذخیره"
    } else {
        "Save"
    }
}
fn cancel_text(f: bool) -> &'static str {
    if f {
        "لغو"
    } else {
        "Cancel"
    }
}
fn unsaved_text(f: bool) -> &'static str {
    if f {
        "تغییرات ذخیره‌نشده"
    } else {
        "Unsaved changes"
    }
}
fn source_document_text(f: bool) -> &'static str {
    if f {
        "سند منبع"
    } else {
        "Source document"
    }
}
fn source_help(f: bool) -> &'static str {
    if f {
        "جایگزینی فایل قبلی را بازنویسی نمی‌کند؛ یک نسخهٔ تغییرناپذیر جدید برای همان شناسه می‌سازد."
    } else {
        "Replacement never overwrites the previous object; it appends a new immutable source revision under the same asset ID."
    }
}
fn current_file_text(f: bool) -> &'static str {
    if f {
        "فایل جاری"
    } else {
        "Current file"
    }
}
fn replacement_pdf_text(f: bool) -> &'static str {
    if f {
        "PDF جایگزین"
    } else {
        "Replacement PDF"
    }
}
fn replacement_help(f: bool) -> &'static str {
    if f {
        "حداکثر ۲۰ MiB؛ فقط PDF. این اقدام OCR و بردارهای جاری را قدیمی می‌کند."
    } else {
        "Maximum 20 MiB; PDF only. This action retires current OCR and vector state."
    }
}
fn replace_text(f: bool) -> &'static str {
    if f {
        "جایگزینی سند منبع"
    } else {
        "Replace source document"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn asset() -> ManagerKnowledgeAssetEditState {
        ManagerKnowledgeAssetEditState {
            id: "asset".into(),
            title: "Title".into(),
            description: Some("Description".into()),
            subject: Some("Math".into()),
            grade: Some("8".into()),
            language: "en".into(),
            template_type: None,
            tags: serde_json::json!({"kind":"guide"}),
            status: "published".into(),
            asset_revision: 7,
            current_source_file_id: Some("source".into()),
            current_source_filename: Some("guide.pdf".into()),
            current_source_sha256: Some("a".repeat(64)),
        }
    }
    #[test]
    fn description_only_is_not_retrieval_sensitive() {
        let baseline = asset();
        let mut draft = Draft::from(&baseline);
        draft.description = "New presentation copy".into();
        assert!(!retrieval_sensitive_changed(
            &baseline,
            &draft,
            &baseline.tags
        ));
    }
    #[test]
    fn vector_payload_metadata_is_retrieval_sensitive() {
        let baseline = asset();
        let mut draft = Draft::from(&baseline);
        draft.subject = "Physics".into();
        assert!(retrieval_sensitive_changed(
            &baseline,
            &draft,
            &baseline.tags
        ));
    }
}
