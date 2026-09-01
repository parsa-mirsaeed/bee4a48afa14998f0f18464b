use crate::application::AuthHooks;
use crate::ui::{ConfirmDialog, Dialog};
use crate::views::role_based::components::{DashboardSection, ResponsiveDashboardLayout};
use api::server_functions::admin_knowledge_ocr_functions::{
    get_admin_knowledge_source_revision, save_admin_verified_ocr, SaveAdminVerifiedOcrRequest,
};
use api::server_functions::admin_knowledge_review_functions::{
    get_admin_verified_ocr, list_admin_knowledge_assets_for_review, AdminKnowledgeReviewAssetDto,
};
use api::server_functions::knowledge_audit_functions::list_admin_knowledge_audit;
use api::server_functions::knowledge_functions::{
    archive_admin_knowledge_asset, embed_admin_knowledge_asset, publish_admin_knowledge_asset,
};
use dioxus::prelude::*;

#[derive(Clone)]
struct OcrEditorState {
    asset_id: String,
    title: String,
    revision: Option<String>,
    original_text: String,
    original_provider: String,
    verified_at: Option<String>,
    verified_by: Option<String>,
    text_sha256: Option<String>,
    source_file_id: Option<String>,
    source_sha256: Option<String>,
    loading: bool,
    error: Option<String>,
}

impl OcrEditorState {
    fn loading(asset_id: String, title: String) -> Self {
        Self {
            asset_id,
            title,
            revision: None,
            original_text: String::new(),
            original_provider: String::new(),
            verified_at: None,
            verified_by: None,
            text_sha256: None,
            source_file_id: None,
            source_sha256: None,
            loading: true,
            error: None,
        }
    }

    fn is_dirty(&self, text: &str, provider: &str) -> bool {
        self.original_text != text || self.original_provider != provider
    }
}

#[component]
pub fn PlatformAdminDashboard(section: String) -> Element {
    let current_user = AuthHooks::use_current_user().ok().flatten();

    if let Some(user) = current_user {
        let content = match section.as_str() {
            "knowledge-audit" => rsx! { PlatformKnowledgeAuditSection {} },
            _ => rsx! { PlatformKnowledgeReviewSection {} },
        };
        rsx! {
            ResponsiveDashboardLayout {
                user,
                active_section: section,
                children: rsx! { {content} }
            }
        }
    } else {
        rsx! { div { class: "flex min-h-screen items-center justify-center", "Loading..." } }
    }
}

#[component]
fn PlatformKnowledgeReviewSection() -> Element {
    let mut selected_ocr_asset = use_signal(|| None::<OcrEditorState>);
    let mut pending_ocr_asset = use_signal(|| None::<(String, String)>);
    let mut pending_archive = use_signal(|| None::<(String, String, String)>);
    let mut discard_ocr_confirmation = use_signal(|| false);
    // Transient confirmation is separate from an asset's persisted lifecycle.
    // Keeping the target context here avoids a cross-card state illusion.
    let mut archive_confirmation = use_signal(|| None::<(String, String, String)>);
    let mut ocr_text = use_signal(String::new);
    let mut provider = use_signal(|| "manual-verified".to_string());
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(|| None::<String>);
    let mut assets =
        use_resource(move || async move { list_admin_knowledge_assets_for_review().await });

    rsx! {
        DashboardSection {
            title: "Governed knowledge review".to_string(),
            description: Some("Review the private source, verify OCR, embed, then publish explicitly. The next legal lifecycle action is shown for each asset.".to_string()),
            children: rsx! {
                div { class: "space-y-6",
                    div { class: "et-ui-card p-5",
                        ol { class: "grid grid-cols-1 gap-3 text-sm md:grid-cols-3",
                            LifecycleStep { number: "1", title: "Review & verify OCR", detail: "Inspect the private PDF and save only verified text." }
                            LifecycleStep { number: "2", title: "Embed", detail: "Queue embedding only after verified OCR exists." }
                            LifecycleStep { number: "3", title: "Publish", detail: "Publish explicitly only after embedding completes." }
                        }
                    }
                    if let Some(message) = notice() {
                        p { class: "rounded-lg bg-blue-50 px-4 py-3 text-sm text-blue-800 dark:bg-blue-900/20 dark:text-blue-200", "{message}" }
                    }
                    match &*assets.read() {
                        None => rsx! { p { class: "text-gray-500", "Loading governed assets..." } },
                        Some(Err(_)) => rsx! { p { class: "text-red-600", "Unable to load governed assets. Refresh and try again." } },
                        Some(Ok(items)) if items.is_empty() => rsx! {
                            div { class: "et-ui-card p-8 text-center text-gray-500", "No manager submissions are waiting." }
                        },
                        Some(Ok(items)) => rsx! {
                            div { class: "grid grid-cols-1 gap-4 xl:grid-cols-2",
                                for item in items.iter() {
                                    {render_review_card(
                                        item,
                                        busy,
                                        notice,
                                        assets,
                                        selected_ocr_asset,
                                        ocr_text,
                                        provider,
                                        archive_confirmation,
                                        pending_ocr_asset,
                                        pending_archive,
                                        discard_ocr_confirmation,
                                    )}
                                }
                            }
                        }
                    }
                    {render_archive_confirmation(
                        archive_confirmation,
                        busy,
                        notice,
                        assets,
                        selected_ocr_asset,
                        ocr_text,
                    )}
                    OcrEditorDialog {
                        selected_ocr_asset,
                        pending_ocr_asset,
                        discard_ocr_confirmation,
                        ocr_text,
                        provider,
                        busy,
                        notice,
                        assets,
                    }
                    DiscardOcrDialog {
                        selected_ocr_asset,
                        pending_ocr_asset,
                        pending_archive,
                        discard_ocr_confirmation,
                        ocr_text,
                        provider,
                        archive_confirmation,
                    }
                }
            }
        }
    }
}

fn open_ocr_editor(
    asset_id: String,
    title: String,
    mut selected_ocr_asset: Signal<Option<OcrEditorState>>,
    mut ocr_text: Signal<String>,
    mut provider: Signal<String>,
) {
    selected_ocr_asset.set(Some(OcrEditorState::loading(asset_id.clone(), title)));
    ocr_text.set(String::new());
    provider.set("manual-verified".to_string());
    spawn(async move {
        let source = get_admin_knowledge_source_revision(asset_id.clone()).await;
        let loaded = get_admin_verified_ocr(asset_id.clone()).await;
        let Some(current) = selected_ocr_asset() else {
            return;
        };
        if current.asset_id != asset_id {
            return;
        }
        let source = match source {
            Ok(source) if source.asset_id == asset_id => source,
            _ => {
                selected_ocr_asset.set(Some(OcrEditorState {
                    loading: false,
                    error: Some(
                        "The current governed source revision could not be loaded. Refresh and review the source again."
                            .to_string(),
                    ),
                    ..current
                }));
                return;
            }
        };
        match loaded {
            Ok(Some(ocr)) => {
                ocr_text.set(ocr.raw_text.clone());
                provider.set(ocr.ocr_provider.clone());
                selected_ocr_asset.set(Some(OcrEditorState {
                    asset_id,
                    title: current.title,
                    revision: Some(ocr.revision),
                    original_text: ocr.raw_text,
                    original_provider: ocr.ocr_provider,
                    verified_at: Some(ocr.verified_at),
                    verified_by: Some(ocr.verified_by),
                    text_sha256: ocr.text_sha256,
                    source_file_id: Some(source.source_file_id),
                    source_sha256: Some(source.source_sha256),
                    loading: false,
                    error: None,
                }));
            }
            Ok(None) => selected_ocr_asset.set(Some(OcrEditorState {
                original_provider: "manual-verified".to_string(),
                source_file_id: Some(source.source_file_id),
                source_sha256: Some(source.source_sha256),
                loading: false,
                ..current
            })),
            Err(_) => selected_ocr_asset.set(Some(OcrEditorState {
                source_file_id: Some(source.source_file_id),
                source_sha256: Some(source.source_sha256),
                loading: false,
                error: Some(
                    "The current verified OCR could not be loaded. Refresh and try again."
                        .to_string(),
                ),
                ..current
            })),
        }
    });
}

fn request_close_ocr_editor(
    mut selected_ocr_asset: Signal<Option<OcrEditorState>>,
    mut pending_ocr_asset: Signal<Option<(String, String)>>,
    mut discard_ocr_confirmation: Signal<bool>,
    mut ocr_text: Signal<String>,
    provider: Signal<String>,
) {
    if selected_ocr_asset()
        .as_ref()
        .is_some_and(|editor| editor.is_dirty(&ocr_text(), &provider()))
    {
        pending_ocr_asset.set(None);
        discard_ocr_confirmation.set(true);
    } else {
        selected_ocr_asset.set(None);
        ocr_text.set(String::new());
    }
}

#[component]
fn OcrEditorDialog(
    mut selected_ocr_asset: Signal<Option<OcrEditorState>>,
    mut pending_ocr_asset: Signal<Option<(String, String)>>,
    mut discard_ocr_confirmation: Signal<bool>,
    mut ocr_text: Signal<String>,
    mut provider: Signal<String>,
    mut busy: Signal<bool>,
    mut notice: Signal<Option<String>>,
    mut assets: Resource<Result<Vec<AdminKnowledgeReviewAssetDto>, dioxus::prelude::ServerFnError>>,
) -> Element {
    let editor = selected_ocr_asset();
    let open = editor.is_some() && !discard_ocr_confirmation();
    let editor = editor.unwrap_or_else(|| OcrEditorState::loading(String::new(), String::new()));
    let is_update = editor.revision.is_some();
    let title = if is_update {
        format!("Update verified OCR — {}", editor.title)
    } else {
        format!("Attach verified OCR — {}", editor.title)
    };
    let reload_asset_id = editor.asset_id.clone();
    let reload_title = editor.title.clone();
    let editor_for_submit = editor.clone();
    let submit = move |event: FormEvent| {
        event.prevent_default();
        if busy() || editor_for_submit.loading {
            return;
        }
        if ocr_text().trim().is_empty() || provider().trim().is_empty() {
            return;
        }
        let Some(expected_source_file_id) = editor_for_submit.source_file_id.clone() else {
            selected_ocr_asset.set(Some(OcrEditorState {
                error: Some(
                    "The governed source revision is unavailable. Refresh and review the source again."
                        .to_string(),
                ),
                ..editor_for_submit.clone()
            }));
            return;
        };
        let Some(expected_source_sha256) = editor_for_submit.source_sha256.clone() else {
            selected_ocr_asset.set(Some(OcrEditorState {
                error: Some(
                    "The governed source revision is unavailable. Refresh and review the source again."
                        .to_string(),
                ),
                ..editor_for_submit.clone()
            }));
            return;
        };
        busy.set(true);
        let request = SaveAdminVerifiedOcrRequest {
            asset_id: editor_for_submit.asset_id.clone(),
            raw_text: ocr_text(),
            ocr_provider: provider().trim().to_string(),
            expected_source_file_id,
            expected_source_sha256,
            expected_revision: editor_for_submit.revision.clone(),
        };
        let editor_after_failure = editor_for_submit.clone();
        spawn(async move {
            match save_admin_verified_ocr(request).await {
                Ok(_) => {
                    selected_ocr_asset.set(None);
                    ocr_text.set(String::new());
                    notice.set(Some("Verified OCR saved. The asset is ready for embedding.".to_string()));
                    assets.restart();
                }
                Err(_) => selected_ocr_asset.set(Some(OcrEditorState {
                    error: Some("OCR verification could not be saved because the source or OCR revision changed, the current source has not been reviewed, or the asset is no longer eligible. Refresh and review it again.".to_string()),
                    ..editor_after_failure
                })),
            }
            busy.set(false);
        });
    };

    rsx! {
        Dialog {
            open,
            title,
            busy: Some(busy()),
            close_label: Some("Close OCR editor".to_string()),
            on_close: move |_| request_close_ocr_editor(
                selected_ocr_asset,
                pending_ocr_asset,
                discard_ocr_confirmation,
                ocr_text,
                provider,
            ),
            form { class: "space-y-4", onsubmit: submit,
                p { class: "text-sm text-gray-500", "Confirm the text against the private source PDF before saving. Saving verified OCR does not publish the asset." }
                if editor.loading {
                    p { class: "rounded-lg bg-blue-50 px-3 py-2 text-sm text-blue-800", role: "status", "Loading the current verified OCR…" }
                } else {
                    if let Some(error) = editor.error.as_ref() {
                        p { class: "rounded-lg bg-red-50 px-3 py-2 text-sm text-red-800", role: "alert", "{error}" }
                    }
                    if let Some(source_file_id) = editor.source_file_id.as_ref() {
                        div { class: "rounded-lg bg-gray-50 p-3 text-xs text-gray-600 dark:bg-gray-900/40 dark:text-gray-300",
                            p { "Governed source revision: {source_file_id}" }
                            if let Some(source_sha256) = editor.source_sha256.as_ref() {
                                p { "Source hash: {source_sha256}" }
                            }
                        }
                    }
                    if let Some(revision) = editor.revision.as_ref() {
                        div { class: "rounded-lg bg-gray-50 p-3 text-xs text-gray-600 dark:bg-gray-900/40 dark:text-gray-300",
                            p { "Current verified revision: {revision}" }
                            if let Some(verified_at) = editor.verified_at.as_ref() {
                                p { "Verified at: {verified_at}" }
                            }
                            if let Some(verified_by) = editor.verified_by.as_ref() {
                                p { "Verified by: {verified_by}" }
                            }
                            if let Some(text_sha256) = editor.text_sha256.as_ref() {
                                p { "Text hash: {text_sha256}" }
                            }
                        }
                    }
                    div {
                        label { r#for: "ocr-provider", class: "mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300", "OCR provider / verification process" }
                        input {
                            id: "ocr-provider",
                            class: "w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-gray-900 dark:border-gray-700 dark:bg-gray-900 dark:text-white",
                            r#type: "text",
                            value: "{provider}",
                            oninput: move |event| provider.set(event.value()),
                            disabled: busy(),
                        }
                    }
                    div {
                        label { r#for: "verified-ocr-text", class: "mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300", "Verified source text" }
                        textarea {
                            id: "verified-ocr-text",
                            class: "w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-gray-900 dark:border-gray-700 dark:bg-gray-900 dark:text-white",
                            rows: "12",
                            value: "{ocr_text}",
                            oninput: move |event| ocr_text.set(event.value()),
                            placeholder: "Paste text that has been checked against the source PDF...",
                            disabled: busy(),
                        }
                    }
                    div { class: "flex flex-wrap gap-2",
                        if editor.error.is_some() {
                            button {
                                class: "rounded-lg border border-gray-300 px-4 py-2 disabled:opacity-50 dark:border-gray-700",
                                r#type: "button",
                                disabled: busy(),
                                onclick: move |_| open_ocr_editor(
                                    reload_asset_id.clone(),
                                    reload_title.clone(),
                                    selected_ocr_asset,
                                    ocr_text,
                                    provider,
                                ),
                                "Reload current OCR and replace draft"
                            }
                        }
                        button {
                            class: "rounded-lg border border-gray-300 px-4 py-2 disabled:opacity-50 dark:border-gray-700",
                            r#type: "button",
                            disabled: busy(),
                            onclick: move |_| request_close_ocr_editor(
                                selected_ocr_asset,
                                pending_ocr_asset,
                                discard_ocr_confirmation,
                                ocr_text,
                                provider,
                            ),
                            "Cancel"
                        }
                        button {
                            class: "rounded-lg bg-primary px-4 py-2 font-medium text-white disabled:opacity-50",
                            r#type: "submit",
                            disabled: busy(),
                            if busy() { "Saving..." } else if is_update { "Save verified OCR changes" } else { "Save verified OCR" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn DiscardOcrDialog(
    mut selected_ocr_asset: Signal<Option<OcrEditorState>>,
    mut pending_ocr_asset: Signal<Option<(String, String)>>,
    mut pending_archive: Signal<Option<(String, String, String)>>,
    mut discard_ocr_confirmation: Signal<bool>,
    mut ocr_text: Signal<String>,
    mut provider: Signal<String>,
    mut archive_confirmation: Signal<Option<(String, String, String)>>,
) -> Element {
    let open = discard_ocr_confirmation();
    let pending = pending_ocr_asset();
    let title = if let Some((_, next_title)) = pending.as_ref() {
        format!("Discard OCR changes and open \"{next_title}\"?")
    } else {
        "Discard unsaved OCR changes?".to_string()
    };
    rsx! {
        ConfirmDialog {
            open,
            title,
            description: "Your unsaved OCR text or verification method will be lost.".to_string(),
            confirm_label: "Discard changes".to_string(),
            cancel_label: "Keep editing".to_string(),
            pending: Some(false),
            destructive: Some(true),
            on_cancel: move |_| {
                pending_ocr_asset.set(None);
                pending_archive.set(None);
                discard_ocr_confirmation.set(false);
            },
            on_confirm: move |_| {
                let next = pending_ocr_asset();
                let archive = pending_archive();
                selected_ocr_asset.set(None);
                ocr_text.set(String::new());
                provider.set("manual-verified".to_string());
                pending_ocr_asset.set(None);
                pending_archive.set(None);
                discard_ocr_confirmation.set(false);
                if let Some(archive) = archive {
                    archive_confirmation.set(Some(archive));
                } else if let Some((asset_id, title)) = next {
                    open_ocr_editor(asset_id, title, selected_ocr_asset, ocr_text, provider);
                }
            },
        }
    }
}

fn render_archive_confirmation(
    mut archive_confirmation: Signal<Option<(String, String, String)>>,
    mut busy: Signal<bool>,
    mut notice: Signal<Option<String>>,
    mut assets: Resource<Result<Vec<AdminKnowledgeReviewAssetDto>, dioxus::prelude::ServerFnError>>,
    mut selected_ocr_asset: Signal<Option<OcrEditorState>>,
    mut ocr_text: Signal<String>,
) -> Element {
    let confirmation = archive_confirmation();
    let (asset_id, title, status) = confirmation.clone().unwrap_or_default();

    rsx! {
        ConfirmDialog {
            open: confirmation.is_some(),
            title: format!("Archive \"{title}\"?"),
            description: if status == "published" {
                format!("This withdraws \"{title}\" from teacher retrieval and cancels active ingestion work.")
            } else {
                format!("This archives \"{title}\" and cancels active ingestion work. Archived assets are terminal.")
            },
            confirm_label: "Archive asset".to_string(),
            cancel_label: "Cancel".to_string(),
            pending: Some(busy()),
            destructive: Some(true),
            on_cancel: move |_| archive_confirmation.set(None),
            on_confirm: move |_| {
                let archived_asset_id = asset_id.clone();
                if archived_asset_id.is_empty() {
                    return;
                }
                busy.set(true);
                notice.set(Some("Archiving asset…".to_string()));
                spawn(async move {
                    match archive_admin_knowledge_asset(archived_asset_id.clone()).await {
                        Ok(_) => {
                            archive_confirmation.set(None);
                            if selected_ocr_asset()
                                .as_ref()
                                .is_some_and(|selected| selected.asset_id == archived_asset_id)
                            {
                                selected_ocr_asset.set(None);
                                ocr_text.set(String::new());
                            }
                            notice.set(Some("Asset archived and withdrawn from governed retrieval.".to_string()));
                            assets.restart();
                        }
                        Err(_) => notice.set(Some("Archive failed. The asset state is unchanged; refresh and try again.".to_string())),
                    }
                    busy.set(false);
                });
            },
        }
    }
}

fn render_review_card(
    item: &AdminKnowledgeReviewAssetDto,
    mut busy: Signal<bool>,
    mut notice: Signal<Option<String>>,
    mut assets: Resource<Result<Vec<AdminKnowledgeReviewAssetDto>, dioxus::prelude::ServerFnError>>,
    mut selected_ocr_asset: Signal<Option<OcrEditorState>>,
    mut ocr_text: Signal<String>,
    provider: Signal<String>,
    mut archive_confirmation: Signal<Option<(String, String, String)>>,
    mut pending_ocr_asset: Signal<Option<(String, String)>>,
    mut pending_archive: Signal<Option<(String, String, String)>>,
    mut discard_ocr_confirmation: Signal<bool>,
) -> Element {
    let asset = &item.asset;
    let asset_id = asset.id.clone();
    let title = asset.title.clone();
    let ocr_title = title.clone();
    let archive_status = asset.status.clone();
    let status = asset.status.as_str();
    let can_edit_ocr = matches!(status, "submitted" | "ocr_pending" | "ocr_ready" | "failed");
    let can_embed = status == "ocr_ready" || (status == "failed" && item.has_verified_ocr);
    let can_publish = status == "embedded";
    let can_archive = status != "archived";
    let (stage_title, next_step) = lifecycle_guidance(status, item.has_verified_ocr);
    let source_href = format!("/api/admin/knowledge-assets/source?asset_id={}", asset.id);
    let source_description = source_description(item);

    rsx! {
        article { key: "{asset.id}", class: "et-ui-card space-y-4 p-5",
            div { class: "flex items-start justify-between gap-3",
                div {
                    h3 { class: "font-semibold text-gray-900 dark:text-white", "{asset.title}" }
                    p { class: "text-sm text-gray-500", "School {asset.school_id}" }
                }
                StatusBadge { status: asset.status.clone() }
            }
            p { class: "text-sm text-gray-600 dark:text-gray-300", "{metadata_line(&asset.subject, &asset.grade, &asset.language)}" }
            if let Some(description) = asset.description.as_ref() {
                p { class: "text-sm text-gray-600 dark:text-gray-300", "{description}" }
            }
            div { class: "rounded-lg border border-gray-200 p-3 dark:border-gray-700",
                p { class: "text-xs font-semibold uppercase tracking-wide text-gray-500", "{stage_title}" }
                p { class: "mt-1 text-sm text-gray-800 dark:text-gray-200", "{next_step}" }
            }
            if let Some(reason) = asset.failure_reason.as_ref() {
                p { class: "rounded-lg bg-red-50 p-3 text-sm text-red-700 dark:bg-red-900/20 dark:text-red-300", "{reason}" }
            }
            div { class: "rounded-lg bg-gray-50 p-3 text-sm dark:bg-gray-900/40",
                p { class: "font-medium text-gray-800 dark:text-gray-200", "Source document" }
                p { class: "mt-1 text-xs text-gray-500", "{source_description}" }
                if item.source_review_available {
                    a {
                        class: "mt-2 inline-flex items-center gap-1 font-medium text-primary hover:underline",
                        href: source_href,
                        target: "_blank",
                        rel: "noopener noreferrer",
                        "Review private PDF",
                        span { class: "material-icons-outlined text-base", aria_hidden: "true", "open_in_new" }
                    }
                } else {
                    p { class: "mt-2 text-xs text-amber-700 dark:text-amber-300", "Private source review is unavailable for this legacy submission." }
                }
            }
            div { class: "flex flex-wrap gap-2",
                if can_edit_ocr {
                    button {
                        class: "rounded-lg border border-gray-300 px-3 py-2 text-sm font-medium disabled:opacity-50 dark:border-gray-700",
                        disabled: busy(),
                        onclick: move |_| {
                            if selected_ocr_asset().as_ref().is_some_and(|editor| {
                                editor.is_dirty(&ocr_text(), &provider())
                            }) {
                                pending_ocr_asset.set(Some((asset_id.clone(), ocr_title.clone())));
                                discard_ocr_confirmation.set(true);
                            } else {
                                open_ocr_editor(
                                    asset_id.clone(),
                                    ocr_title.clone(),
                                    selected_ocr_asset,
                                    ocr_text,
                                    provider,
                                );
                            }
                        },
                        if item.has_verified_ocr { "Update verified OCR" } else { "Attach verified OCR" }
                    }
                }
                if can_embed {
                    {
                        let embed_id = asset.id.clone();
                        rsx! {
                            button {
                                class: "rounded-lg bg-indigo-600 px-3 py-2 text-sm font-medium text-white disabled:opacity-50",
                                disabled: busy(),
                                onclick: move |_| {
                                    let asset_id = embed_id.clone();
                                    busy.set(true);
                                    notice.set(None);
                                    spawn(async move {
                                        match embed_admin_knowledge_asset(asset_id).await {
                                            Ok(_) => {
                                                notice.set(Some("Embedding queued. Publication stays blocked until embedding completes.".to_string()));
                                                assets.restart();
                                            }
                                            Err(_) => notice.set(Some("Embedding could not be queued. Refresh the asset state and try again.".to_string())),
                                        }
                                        busy.set(false);
                                    });
                                },
                                if status == "failed" { "Retry embedding" } else { "Queue embedding" }
                            }
                        }
                    }
                }
                if can_publish {
                    {
                        let publish_id = asset.id.clone();
                        rsx! {
                            button {
                                class: "rounded-lg bg-green-600 px-3 py-2 text-sm font-medium text-white disabled:opacity-50",
                                disabled: busy(),
                                onclick: move |_| {
                                    let asset_id = publish_id.clone();
                                    busy.set(true);
                                    notice.set(None);
                                    spawn(async move {
                                        match publish_admin_knowledge_asset(asset_id).await {
                                            Ok(_) => {
                                                notice.set(Some("Asset published. It can now be selected by teachers in the same school.".to_string()));
                                                assets.restart();
                                            }
                                            Err(_) => notice.set(Some("Publication failed. Refresh the asset state and try again.".to_string())),
                                        }
                                        busy.set(false);
                                    });
                                },
                                "Publish"
                            }
                        }
                    }
                }
                if can_archive {
                    {
                        let archive_id = asset.id.clone();
                        let archive_title = title.clone();
                        rsx! {
                            button {
                                class: "rounded-lg border border-red-300 px-3 py-2 text-sm font-medium text-red-700 disabled:opacity-50 dark:text-red-300",
                                disabled: busy(),
                                onclick: move |_| {
                                    if selected_ocr_asset().as_ref().is_some_and(|editor| {
                                        editor.asset_id == archive_id
                                            && editor.is_dirty(&ocr_text(), &provider())
                                    }) {
                                        pending_ocr_asset.set(None);
                                        pending_archive.set(Some((
                                            archive_id.clone(),
                                            archive_title.clone(),
                                            archive_status.clone(),
                                        )));
                                        discard_ocr_confirmation.set(true);
                                    } else {
                                        archive_confirmation.set(Some((
                                            archive_id.clone(),
                                            archive_title.clone(),
                                            archive_status.clone(),
                                        )));
                                    }
                                },
                                if status == "published" { "Withdraw / archive" } else { "Archive" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn LifecycleStep(number: &'static str, title: &'static str, detail: &'static str) -> Element {
    rsx! {
        li { class: "flex gap-3",
            span { class: "flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-primary text-xs font-bold text-white", "{number}" }
            div {
                p { class: "font-semibold text-gray-900 dark:text-white", "{title}" }
                p { class: "mt-1 text-gray-500 dark:text-gray-400", "{detail}" }
            }
        }
    }
}

#[component]
fn PlatformKnowledgeAuditSection() -> Element {
    let logs = use_resource(move || async move { list_admin_knowledge_audit(200).await });
    rsx! {
        DashboardSection {
            title: "Knowledge audit trail".to_string(),
            description: Some("Recent governed-knowledge lifecycle, source-review, and retrieval events.".to_string()),
            children: rsx! {
                match &*logs.read() {
                    None => rsx! { p { class: "text-gray-500", "Loading audit events..." } },
                    Some(Err(_)) => rsx! { p { class: "text-red-600", "Unable to load audit events. Refresh and try again." } },
                    Some(Ok(items)) if items.is_empty() => rsx! {
                        div { class: "et-ui-card p-8 text-center text-gray-500", "No audit events have been recorded." }
                    },
                    Some(Ok(items)) => rsx! {
                        div { class: "et-ui-card overflow-x-auto",
                            table { class: "min-w-full text-sm",
                                thead { class: "border-b border-gray-200 dark:border-gray-700",
                                    tr {
                                        th { class: "px-4 py-3 text-left", "Time" }
                                        th { class: "px-4 py-3 text-left", "Actor" }
                                        th { class: "px-4 py-3 text-left", "Action" }
                                        th { class: "px-4 py-3 text-left", "Target" }
                                        th { class: "px-4 py-3 text-left", "Details" }
                                    }
                                }
                                tbody {
                                    for log in items.iter() {
                                        tr { key: "{log.id}", class: "border-b border-gray-100 dark:border-gray-800",
                                            td { class: "whitespace-nowrap px-4 py-3", "{log.created_at}" }
                                            td { class: "px-4 py-3", "{log.actor_role}" }
                                            td { class: "px-4 py-3 font-medium", "{log.action}" }
                                            td { class: "px-4 py-3 font-mono text-xs", "{log.target_id}" }
                                            td { class: "max-w-md truncate px-4 py-3", "{log.details}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StatusBadge(status: String) -> Element {
    let classes = match status.as_str() {
        "published" => "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300",
        "embedded" | "embedding_pending" => {
            "bg-indigo-100 text-indigo-800 dark:bg-indigo-900/30 dark:text-indigo-300"
        }
        "failed" => "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300",
        "archived" => "bg-gray-200 text-gray-700 dark:bg-gray-800 dark:text-gray-300",
        _ => "bg-amber-100 text-amber-800 dark:bg-amber-900/30 dark:text-amber-300",
    };
    rsx! { span { class: "rounded-full px-2.5 py-1 text-xs font-semibold {classes}", "{status}" } }
}

fn lifecycle_guidance(status: &str, has_verified_ocr: bool) -> (&'static str, &'static str) {
    match status {
        "submitted" | "ocr_pending" => (
            "Step 1 · Source review",
            "Review the private PDF and attach text only after verifying it against the source.",
        ),
        "ocr_ready" => (
            "Step 2 · Embedding",
            "Verified OCR is ready. Queue embedding; publication remains blocked until it completes.",
        ),
        "embedding_pending" => (
            "Step 2 · Embedding in progress",
            "An embedding job is queued or running. No ingestion transition is available until it finishes or fails.",
        ),
        "embedded" => (
            "Step 3 · Publication",
            "Embedding is complete. Publish explicitly to make the asset available for teacher selection.",
        ),
        "published" => (
            "Published",
            "The asset is available for governed teacher selection. Withdraw it by archiving if it should no longer be used.",
        ),
        "archived" => (
            "Archived",
            "This asset is withdrawn and terminal. No further ingestion or publication actions are available.",
        ),
        "failed" if has_verified_ocr => (
            "Recovery",
            "Embedding failed after verified OCR. Retry embedding or replace the verified OCR if the source text needs correction.",
        ),
        "failed" => (
            "Recovery",
            "Processing failed before verified OCR was available. Review the source and attach verified OCR before continuing.",
        ),
        _ => (
            "State unavailable",
            "Refresh the asset list before taking another lifecycle action.",
        ),
    }
}

fn source_description(item: &AdminKnowledgeReviewAssetDto) -> String {
    let filename = item
        .original_filename
        .as_deref()
        .unwrap_or("Source metadata unavailable");
    match item
        .file_size_bytes
        .and_then(|value| u64::try_from(value).ok())
    {
        Some(bytes) => format!("{filename} · {}", format_file_size(bytes)),
        None => filename.to_string(),
    }
}

fn format_file_size(bytes: u64) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    const KIB: f64 = 1024.0;
    if bytes >= 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / MIB)
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / KIB)
    } else {
        format!("{bytes} B")
    }
}

fn metadata_line(subject: &Option<String>, grade: &Option<String>, language: &str) -> String {
    [subject.as_deref(), grade.as_deref(), Some(language)]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_guidance_blocks_invalid_next_steps() {
        assert!(lifecycle_guidance("embedding_pending", true)
            .1
            .contains("No ingestion transition"));
        assert!(lifecycle_guidance("archived", true)
            .1
            .contains("No further ingestion"));
        assert!(lifecycle_guidance("failed", false)
            .1
            .contains("attach verified OCR"));
    }

    #[test]
    fn file_size_labels_are_human_readable() {
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(2048), "2.0 KiB");
        assert_eq!(format_file_size(2 * 1024 * 1024), "2.0 MiB");
    }
}
