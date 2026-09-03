use crate::application::AuthHooks;
use crate::i18n::{
    format_product_datetime_text, platform_admin_actor_label, platform_admin_audit_action_label,
    platform_admin_language_label, platform_admin_lifecycle_guidance, platform_admin_status_label,
    platform_admin_target_type_label, platform_admin_translation, use_locale, Locale,
};
use crate::ui::{ConfirmDialog, Dialog};
use crate::views::role_based::components::{DashboardSection, ResponsiveDashboardLayout};
use api::server_functions::admin_knowledge_ocr_functions::{
    get_admin_knowledge_source_revision, save_admin_verified_ocr, SaveAdminVerifiedOcrRequest,
};
use api::server_functions::admin_knowledge_review_functions::{
    get_admin_verified_ocr, list_admin_knowledge_assets_for_review, AdminKnowledgeReviewAssetDto,
};
use api::server_functions::knowledge_audit_functions::{
    list_admin_knowledge_audit, KnowledgeAuditLogDto,
};
use api::server_functions::knowledge_functions::{
    archive_admin_knowledge_asset, embed_admin_knowledge_asset, publish_admin_knowledge_asset,
};
use dioxus::prelude::*;

fn admin_t(key: &'static str, locale: Locale) -> String {
    platform_admin_translation(key, locale)
        .unwrap_or(key)
        .to_string()
}

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
    error: Option<&'static str>,
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
    let locale = use_locale().current();
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
        rsx! {
            div { class: "flex min-h-screen items-center justify-center", role: "status",
                "{admin_t("platform_admin.loading", locale)}"
            }
        }
    }
}

#[component]
fn PlatformKnowledgeReviewSection() -> Element {
    let locale = use_locale().current();
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
    let mut notice = use_signal(|| None::<&'static str>);
    let mut assets =
        use_resource(move || async move { list_admin_knowledge_assets_for_review().await });

    rsx! {
        DashboardSection {
            title: admin_t("platform_admin.review.title", locale),
            description: Some(admin_t("platform_admin.review.description", locale)),
            children: rsx! {
                div { class: "space-y-6",
                    div { class: "et-ui-card p-5",
                        ol { class: "grid grid-cols-1 gap-3 text-sm md:grid-cols-3",
                            LifecycleStep {
                                number: "1",
                                title: admin_t("platform_admin.review.step1.title", locale),
                                detail: admin_t("platform_admin.review.step1.detail", locale),
                            }
                            LifecycleStep {
                                number: "2",
                                title: admin_t("platform_admin.review.step2.title", locale),
                                detail: admin_t("platform_admin.review.step2.detail", locale),
                            }
                            LifecycleStep {
                                number: "3",
                                title: admin_t("platform_admin.review.step3.title", locale),
                                detail: admin_t("platform_admin.review.step3.detail", locale),
                            }
                        }
                    }
                    if let Some(message_key) = notice() {
                        p {
                            class: "rounded-lg bg-blue-50 px-4 py-3 text-sm text-blue-800 dark:bg-blue-900/20 dark:text-blue-200",
                            role: "status",
                            "{admin_t(message_key, locale)}"
                        }
                    }
                    match &*assets.read() {
                        None => rsx! {
                            p { class: "text-gray-500", role: "status", "{admin_t("platform_admin.review.loading", locale)}" }
                        },
                        Some(Err(_)) => rsx! {
                            p { class: "text-red-600", role: "alert", "{admin_t("platform_admin.review.load_error", locale)}" }
                        },
                        Some(Ok(items)) if items.is_empty() => rsx! {
                            div { class: "et-ui-card p-8 text-center text-gray-500", "{admin_t("platform_admin.review.empty", locale)}" }
                        },
                        Some(Ok(items)) => rsx! {
                            div { class: "grid grid-cols-1 gap-4 xl:grid-cols-2",
                                for item in items.iter() {
                                    {render_review_card(
                                        item,
                                        locale,
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
                        locale,
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
                    error: Some("platform_admin.ocr.source_load_error"),
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
                error: Some("platform_admin.ocr.load_error"),
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
    mut notice: Signal<Option<&'static str>>,
    mut assets: Resource<Result<Vec<AdminKnowledgeReviewAssetDto>, dioxus::prelude::ServerFnError>>,
) -> Element {
    let locale = use_locale().current();
    let editor = selected_ocr_asset();
    let open = editor.is_some() && !discard_ocr_confirmation();
    let editor = editor.unwrap_or_else(|| OcrEditorState::loading(String::new(), String::new()));
    let is_update = editor.revision.is_some();
    let title = format!(
        "{} — {}",
        if is_update {
            admin_t("platform_admin.ocr.update_title", locale)
        } else {
            admin_t("platform_admin.ocr.attach_title", locale)
        },
        editor.title
    );
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
                error: Some("platform_admin.ocr.source_revision_unavailable"),
                ..editor_for_submit.clone()
            }));
            return;
        };
        let Some(expected_source_sha256) = editor_for_submit.source_sha256.clone() else {
            selected_ocr_asset.set(Some(OcrEditorState {
                error: Some("platform_admin.ocr.source_revision_unavailable"),
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
                    notice.set(Some("platform_admin.notice.ocr_saved"));
                    assets.restart();
                }
                Err(_) => selected_ocr_asset.set(Some(OcrEditorState {
                    error: Some("platform_admin.ocr.save_conflict"),
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
            close_label: Some(admin_t("platform_admin.ocr.close", locale)),
            on_close: move |_| request_close_ocr_editor(
                selected_ocr_asset,
                pending_ocr_asset,
                discard_ocr_confirmation,
                ocr_text,
                provider,
            ),
            form { class: "space-y-4", onsubmit: submit,
                p { class: "text-sm text-gray-500", "{admin_t("platform_admin.ocr.helper", locale)}" }
                if editor.loading {
                    p {
                        class: "rounded-lg bg-blue-50 px-3 py-2 text-sm text-blue-800 dark:bg-blue-900/20 dark:text-blue-200",
                        role: "status",
                        "{admin_t("platform_admin.ocr.loading", locale)}"
                    }
                } else {
                    if let Some(error_key) = editor.error {
                        p {
                            class: "rounded-lg bg-red-50 px-3 py-2 text-sm text-red-800 dark:bg-red-900/20 dark:text-red-200",
                            role: "alert",
                            "{admin_t(error_key, locale)}"
                        }
                    }
                    if let Some(source_file_id) = editor.source_file_id.as_ref() {
                        div { class: "rounded-lg bg-gray-50 p-3 text-xs text-gray-600 dark:bg-gray-900/40 dark:text-gray-300",
                            p {
                                span { class: "font-medium", "{admin_t("platform_admin.ocr.source_revision", locale)}: " }
                                code { class: "break-all", dir: "ltr", "{source_file_id}" }
                            }
                            if let Some(source_sha256) = editor.source_sha256.as_ref() {
                                p {
                                    span { class: "font-medium", "{admin_t("platform_admin.ocr.source_hash", locale)}: " }
                                    code { class: "break-all", dir: "ltr", "{source_sha256}" }
                                }
                            }
                        }
                    }
                    if let Some(revision) = editor.revision.as_ref() {
                        div { class: "rounded-lg bg-gray-50 p-3 text-xs text-gray-600 dark:bg-gray-900/40 dark:text-gray-300",
                            p {
                                span { class: "font-medium", "{admin_t("platform_admin.ocr.current_revision", locale)}: " }
                                code { class: "break-all", dir: "ltr", "{revision}" }
                            }
                            if let Some(verified_at) = editor.verified_at.as_ref() {
                                p {
                                    span { class: "font-medium", "{admin_t("platform_admin.ocr.verified_at", locale)}: " }
                                    "{format_product_datetime_text(verified_at, locale)}"
                                }
                            }
                            if let Some(verified_by) = editor.verified_by.as_ref() {
                                p {
                                    span { class: "font-medium", "{admin_t("platform_admin.ocr.verified_by", locale)}: " }
                                    code { class: "break-all", dir: "ltr", "{verified_by}" }
                                }
                            }
                            if let Some(text_sha256) = editor.text_sha256.as_ref() {
                                p {
                                    span { class: "font-medium", "{admin_t("platform_admin.ocr.text_hash", locale)}: " }
                                    code { class: "break-all", dir: "ltr", "{text_sha256}" }
                                }
                            }
                        }
                    }
                    div {
                        label {
                            r#for: "ocr-provider",
                            class: "mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300",
                            "{admin_t("platform_admin.ocr.provider", locale)}"
                        }
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
                        label {
                            r#for: "verified-ocr-text",
                            class: "mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300",
                            "{admin_t("platform_admin.ocr.text", locale)}"
                        }
                        textarea {
                            id: "verified-ocr-text",
                            class: "w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-gray-900 dark:border-gray-700 dark:bg-gray-900 dark:text-white",
                            rows: "12",
                            value: "{ocr_text}",
                            oninput: move |event| ocr_text.set(event.value()),
                            placeholder: admin_t("platform_admin.ocr.placeholder", locale),
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
                                "{admin_t("platform_admin.ocr.reload", locale)}"
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
                            "{admin_t("platform_admin.ocr.cancel", locale)}"
                        }
                        button {
                            class: "rounded-lg bg-primary px-4 py-2 font-medium text-white disabled:opacity-50",
                            r#type: "submit",
                            disabled: busy(),
                            if busy() {
                                "{admin_t("platform_admin.ocr.saving", locale)}"
                            } else if is_update {
                                "{admin_t("platform_admin.ocr.save_changes", locale)}"
                            } else {
                                "{admin_t("platform_admin.ocr.save", locale)}"
                            }
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
    let locale = use_locale().current();
    let open = discard_ocr_confirmation();
    let pending = pending_ocr_asset();
    let title = if let Some((_, next_title)) = pending.as_ref() {
        format!(
            "{} \"{}\"?",
            admin_t("platform_admin.ocr.discard_open", locale),
            next_title
        )
    } else {
        admin_t("platform_admin.ocr.discard_title", locale)
    };
    rsx! {
        ConfirmDialog {
            open,
            title,
            description: admin_t("platform_admin.ocr.discard_description", locale),
            confirm_label: admin_t("platform_admin.ocr.discard_confirm", locale),
            cancel_label: admin_t("platform_admin.ocr.keep_editing", locale),
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
    locale: Locale,
    mut archive_confirmation: Signal<Option<(String, String, String)>>,
    mut busy: Signal<bool>,
    mut notice: Signal<Option<&'static str>>,
    mut assets: Resource<Result<Vec<AdminKnowledgeReviewAssetDto>, dioxus::prelude::ServerFnError>>,
    mut selected_ocr_asset: Signal<Option<OcrEditorState>>,
    mut ocr_text: Signal<String>,
) -> Element {
    let confirmation = archive_confirmation();
    let (asset_id, title, status) = confirmation.clone().unwrap_or_default();

    rsx! {
        ConfirmDialog {
            open: confirmation.is_some(),
            title: format!("{} — {}", admin_t("platform_admin.archive.title", locale), title),
            description: if status == "published" {
                admin_t("platform_admin.archive.published_description", locale)
            } else {
                admin_t("platform_admin.archive.description", locale)
            },
            confirm_label: admin_t("platform_admin.archive.confirm", locale),
            cancel_label: admin_t("platform_admin.archive.cancel", locale),
            pending: Some(busy()),
            destructive: Some(true),
            on_cancel: move |_| archive_confirmation.set(None),
            on_confirm: move |_| {
                let archived_asset_id = asset_id.clone();
                if archived_asset_id.is_empty() {
                    return;
                }
                busy.set(true);
                notice.set(Some("platform_admin.notice.archiving"));
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
                            notice.set(Some("platform_admin.notice.archived"));
                            assets.restart();
                        }
                        Err(_) => notice.set(Some("platform_admin.notice.archive_failed")),
                    }
                    busy.set(false);
                });
            },
        }
    }
}

fn render_review_card(
    item: &AdminKnowledgeReviewAssetDto,
    locale: Locale,
    mut busy: Signal<bool>,
    mut notice: Signal<Option<&'static str>>,
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
    let (stage_title, next_step) =
        platform_admin_lifecycle_guidance(status, item.has_verified_ocr, locale);
    let source_href = format!("/api/admin/knowledge-assets/source?asset_id={}", asset.id);
    let source_description = source_description(item, locale);
    let language = platform_admin_language_label(&asset.language, locale);
    let school_reference = short_reference(&asset.school_id);

    rsx! {
        article { key: "{asset.id}", class: "et-ui-card space-y-4 p-5",
            div { class: "flex items-start justify-between gap-3",
                div { class: "min-w-0",
                    h3 { class: "font-semibold text-gray-900 dark:text-white", "{asset.title}" }
                    p { class: "mt-1 text-sm text-gray-600 dark:text-gray-300",
                        span { class: "font-medium", "{admin_t("platform_admin.metadata.school", locale)}: " }
                        span { dir: "auto", "{item.school_name}" }
                    }
                    p { class: "mt-0.5 text-xs text-gray-500",
                        "{admin_t("platform_admin.metadata.school_reference", locale)}: {school_reference}"
                    }
                }
                StatusBadge { status: asset.status.clone(), locale }
            }
            div { class: "grid grid-cols-1 gap-2 text-sm sm:grid-cols-3",
                if let Some(subject) = asset.subject.as_ref() {
                    MetadataField {
                        label: admin_t("platform_admin.metadata.subject", locale),
                        value: subject.clone(),
                    }
                }
                if let Some(grade) = asset.grade.as_ref() {
                    MetadataField {
                        label: admin_t("platform_admin.metadata.grade", locale),
                        value: grade.clone(),
                    }
                }
                MetadataField {
                    label: admin_t("platform_admin.metadata.language", locale),
                    value: language,
                }
            }
            if let Some(description) = asset.description.as_ref() {
                p { class: "text-sm text-gray-600 dark:text-gray-300", dir: "auto", "{description}" }
            }
            div { class: "rounded-lg border border-gray-200 p-3 dark:border-gray-700",
                p { class: "text-xs font-semibold uppercase tracking-wide text-gray-500", "{stage_title}" }
                p { class: "mt-1 text-sm text-gray-800 dark:text-gray-200", "{next_step}" }
            }
            if let Some(reason) = asset.failure_reason.as_ref() {
                details { class: "rounded-lg bg-red-50 p-3 text-sm text-red-700 dark:bg-red-900/20 dark:text-red-300",
                    summary { class: "cursor-pointer font-medium", "{platform_admin_status_label("failed", locale)}" }
                    p { class: "mt-2", dir: "auto", "{reason}" }
                }
            }
            div { class: "rounded-lg bg-gray-50 p-3 text-sm dark:bg-gray-900/40",
                p { class: "font-medium text-gray-800 dark:text-gray-200", "{admin_t("platform_admin.source.title", locale)}" }
                p { class: "mt-1 text-xs text-gray-500", dir: "auto", "{source_description}" }
                if item.source_review_available {
                    a {
                        class: "mt-2 inline-flex items-center gap-1 font-medium text-primary hover:underline",
                        href: source_href,
                        target: "_blank",
                        rel: "noopener noreferrer",
                        "{admin_t("platform_admin.source.review", locale)}",
                        span { class: "material-icons-outlined text-base", aria_hidden: "true", "open_in_new" }
                    }
                } else {
                    p { class: "mt-2 text-xs text-amber-700 dark:text-amber-300", "{admin_t("platform_admin.source.unavailable", locale)}" }
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
                        if item.has_verified_ocr {
                            "{admin_t("platform_admin.action.update_ocr", locale)}"
                        } else {
                            "{admin_t("platform_admin.action.attach_ocr", locale)}"
                        }
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
                                                notice.set(Some("platform_admin.notice.embedding_queued"));
                                                assets.restart();
                                            }
                                            Err(_) => notice.set(Some("platform_admin.notice.embedding_failed")),
                                        }
                                        busy.set(false);
                                    });
                                },
                                if status == "failed" {
                                    "{admin_t("platform_admin.action.retry_embedding", locale)}"
                                } else {
                                    "{admin_t("platform_admin.action.queue_embedding", locale)}"
                                }
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
                                                notice.set(Some("platform_admin.notice.published"));
                                                assets.restart();
                                            }
                                            Err(_) => notice.set(Some("platform_admin.notice.publish_failed")),
                                        }
                                        busy.set(false);
                                    });
                                },
                                "{admin_t("platform_admin.action.publish", locale)}"
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
                                if status == "published" {
                                    "{admin_t("platform_admin.action.withdraw_archive", locale)}"
                                } else {
                                    "{admin_t("platform_admin.action.archive", locale)}"
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
fn MetadataField(label: String, value: String) -> Element {
    rsx! {
        div { class: "rounded-lg border border-gray-200 px-3 py-2 dark:border-gray-700",
            p { class: "text-xs font-medium text-gray-500", "{label}" }
            p { class: "mt-0.5 font-medium text-gray-800 dark:text-gray-200", dir: "auto", "{value}" }
        }
    }
}

#[component]
fn LifecycleStep(number: &'static str, title: String, detail: String) -> Element {
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
    let locale = use_locale().current();
    let mut selected_audit = use_signal(|| None::<KnowledgeAuditLogDto>);
    let logs = use_resource(move || async move { list_admin_knowledge_audit(200).await });
    let assets =
        use_resource(move || async move { list_admin_knowledge_assets_for_review().await });

    rsx! {
        DashboardSection {
            title: admin_t("platform_admin.audit.title", locale),
            description: Some(admin_t("platform_admin.audit.description", locale)),
            children: rsx! {
                match &*logs.read() {
                    None => rsx! {
                        p { class: "text-gray-500", role: "status", "{admin_t("platform_admin.audit.loading", locale)}" }
                    },
                    Some(Err(_)) => rsx! {
                        p { class: "text-red-600", role: "alert", "{admin_t("platform_admin.audit.load_error", locale)}" }
                    },
                    Some(Ok(items)) if items.is_empty() => rsx! {
                        div { class: "et-ui-card p-8 text-center text-gray-500", "{admin_t("platform_admin.audit.empty", locale)}" }
                    },
                    Some(Ok(items)) => {
                        let asset_guard = assets.read();
                        let known_assets = match &*asset_guard {
                            Some(Ok(items)) => Some(items.as_slice()),
                            _ => None,
                        };
                        rsx! {
                            div { class: "et-ui-card overflow-x-auto",
                                table { class: "min-w-full text-sm",
                                    thead { class: "border-b border-gray-200 dark:border-gray-700",
                                        tr {
                                            th { class: "px-4 py-3 text-start", scope: "col", "{admin_t("platform_admin.audit.time", locale)}" }
                                            th { class: "px-4 py-3 text-start", scope: "col", "{admin_t("platform_admin.audit.actor", locale)}" }
                                            th { class: "px-4 py-3 text-start", scope: "col", "{admin_t("platform_admin.audit.action", locale)}" }
                                            th { class: "px-4 py-3 text-start", scope: "col", "{admin_t("platform_admin.audit.target", locale)}" }
                                            th { class: "px-4 py-3 text-start", scope: "col", "{admin_t("platform_admin.audit.details", locale)}" }
                                        }
                                    }
                                    tbody {
                                        for log in items.iter() {
                                            {
                                                let friendly_time = format_product_datetime_text(&log.created_at, locale);
                                                let actor = platform_admin_actor_label(&log.actor_role, locale);
                                                let school = audit_school_label(log, known_assets, locale);
                                                let action = platform_admin_audit_action_label(&log.action, locale);
                                                let target = audit_target_label(log, known_assets, locale);
                                                let detail_log = log.clone();
                                                rsx! {
                                                    tr { key: "{log.id}", class: "border-b border-gray-100 align-top dark:border-gray-800",
                                                        td { class: "whitespace-nowrap px-4 py-3 text-start", "{friendly_time}" }
                                                        td { class: "px-4 py-3 text-start",
                                                            p { class: "font-medium", "{actor}" }
                                                            p { class: "mt-0.5 text-xs text-gray-500", dir: "auto", "{school}" }
                                                        }
                                                        td { class: "px-4 py-3 text-start font-medium", "{action}" }
                                                        td { class: "px-4 py-3 text-start", dir: "auto", "{target}" }
                                                        td { class: "px-4 py-3 text-start",
                                                            button {
                                                                r#type: "button",
                                                                class: "font-medium text-primary hover:underline",
                                                                onclick: move |_| selected_audit.set(Some(detail_log.clone())),
                                                                "{admin_t("platform_admin.audit.view_details", locale)}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            AuditDetailDialog { selected_audit, assets }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn AuditDetailDialog(
    mut selected_audit: Signal<Option<KnowledgeAuditLogDto>>,
    assets: Resource<Result<Vec<AdminKnowledgeReviewAssetDto>, dioxus::prelude::ServerFnError>>,
) -> Element {
    let locale = use_locale().current();
    let Some(log) = selected_audit() else {
        return rsx! {};
    };
    let asset_guard = assets.read();
    let known_assets = match &*asset_guard {
        Some(Ok(items)) => Some(items.as_slice()),
        _ => None,
    };
    let action_label = platform_admin_audit_action_label(&log.action, locale);
    let target_label = audit_target_label(&log, known_assets, locale);
    let school_label = audit_school_label(&log, known_assets, locale);
    let actor_label = platform_admin_actor_label(&log.actor_role, locale);
    let friendly_time = format_product_datetime_text(&log.created_at, locale);
    let none = admin_t("platform_admin.audit.none", locale);

    rsx! {
        Dialog {
            open: true,
            title: format!("{} — {}", admin_t("platform_admin.audit.detail_title", locale), action_label),
            busy: Some(false),
            close_label: Some(admin_t("platform_admin.ocr.cancel", locale)),
            on_close: move |_| selected_audit.set(None),
            div { class: "space-y-4 text-sm",
                dl { class: "grid grid-cols-1 gap-3 sm:grid-cols-2",
                    AuditDetailField {
                        label: admin_t("platform_admin.audit.action", locale),
                        value: platform_admin_audit_action_label(&log.action, locale),
                        technical: false,
                    }
                    AuditDetailField {
                        label: admin_t("platform_admin.audit.target", locale),
                        value: target_label,
                        technical: false,
                    }
                    AuditDetailField {
                        label: admin_t("platform_admin.audit.actor", locale),
                        value: actor_label,
                        technical: false,
                    }
                    AuditDetailField {
                        label: admin_t("platform_admin.audit.school", locale),
                        value: school_label,
                        technical: false,
                    }
                    AuditDetailField {
                        label: admin_t("platform_admin.audit.friendly_time", locale),
                        value: friendly_time,
                        technical: false,
                    }
                    AuditDetailField {
                        label: admin_t("platform_admin.audit.exact_utc", locale),
                        value: log.created_at.clone(),
                        technical: true,
                    }
                    AuditDetailField {
                        label: admin_t("platform_admin.audit.action_code", locale),
                        value: log.action.clone(),
                        technical: true,
                    }
                    AuditDetailField {
                        label: admin_t("platform_admin.audit.target_type", locale),
                        value: log.target_type.clone(),
                        technical: true,
                    }
                    AuditDetailField {
                        label: admin_t("platform_admin.audit.target_id", locale),
                        value: log.target_id.clone(),
                        technical: true,
                    }
                    AuditDetailField {
                        label: admin_t("platform_admin.audit.school_id", locale),
                        value: log.school_id.clone().unwrap_or_else(|| none.clone()),
                        technical: true,
                    }
                    AuditDetailField {
                        label: admin_t("platform_admin.audit.actor_role", locale),
                        value: log.actor_role.clone(),
                        technical: true,
                    }
                    AuditDetailField {
                        label: admin_t("platform_admin.audit.actor_id", locale),
                        value: log.actor_id.clone().unwrap_or_else(|| none.clone()),
                        technical: true,
                    }
                    AuditDetailField {
                        label: admin_t("platform_admin.audit.request_id", locale),
                        value: log.request_id.clone().unwrap_or_else(|| none.clone()),
                        technical: true,
                    }
                }
                div { class: "rounded-lg border border-gray-200 p-3 dark:border-gray-700",
                    h3 { class: "font-semibold text-gray-900 dark:text-white", "{admin_t("platform_admin.audit.structured_details", locale)}" }
                    if let Some(object) = log.details.as_object() {
                        if object.is_empty() {
                            p { class: "mt-2 text-gray-500", "{none}" }
                        } else {
                            dl { class: "mt-3 space-y-2",
                                for (key, value) in object.iter() {
                                    div { class: "grid grid-cols-1 gap-1 sm:grid-cols-[minmax(10rem,0.4fr)_1fr]",
                                        dt { class: "font-mono text-xs text-gray-500", dir: "ltr", "{key}" }
                                        dd { class: "break-all font-mono text-xs text-gray-800 dark:text-gray-200", dir: "auto", "{value}" }
                                    }
                                }
                            }
                        }
                    } else {
                        pre { class: "mt-3 whitespace-pre-wrap break-all text-xs", dir: "auto", "{log.details}" }
                    }
                }
            }
        }
    }
}

#[component]
fn AuditDetailField(label: String, value: String, technical: bool) -> Element {
    rsx! {
        div { class: "rounded-lg bg-gray-50 p-3 dark:bg-gray-900/40",
            dt { class: "text-xs font-medium text-gray-500", "{label}" }
            dd {
                class: if technical { "mt-1 break-all font-mono text-xs text-gray-900 dark:text-white" } else { "mt-1 text-gray-900 dark:text-white" },
                dir: if technical { "ltr" } else { "auto" },
                "{value}"
            }
        }
    }
}

#[component]
fn StatusBadge(status: String, locale: Locale) -> Element {
    let classes = match status.as_str() {
        "published" => "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300",
        "embedded" | "embedding_pending" => {
            "bg-indigo-100 text-indigo-800 dark:bg-indigo-900/30 dark:text-indigo-300"
        }
        "failed" => "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300",
        "archived" => "bg-gray-200 text-gray-700 dark:bg-gray-800 dark:text-gray-300",
        _ => "bg-amber-100 text-amber-800 dark:bg-amber-900/30 dark:text-amber-300",
    };
    let label = platform_admin_status_label(&status, locale);
    rsx! {
        span { class: "rounded-full px-2.5 py-1 text-xs font-semibold {classes}", "{label}" }
    }
}

fn source_description(item: &AdminKnowledgeReviewAssetDto, locale: Locale) -> String {
    let filename = item
        .original_filename
        .clone()
        .unwrap_or_else(|| admin_t("platform_admin.source.metadata_unavailable", locale));
    match item
        .file_size_bytes
        .and_then(|value| u64::try_from(value).ok())
    {
        Some(bytes) => format!("{filename} · {}", format_file_size(bytes)),
        None => filename,
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

fn short_reference(value: &str) -> String {
    let suffix = value.chars().rev().take(8).collect::<Vec<_>>();
    let suffix = suffix.into_iter().rev().collect::<String>();
    if value.chars().count() > 8 {
        format!("…{suffix}")
    } else {
        suffix
    }
}

fn audit_target_label(
    log: &KnowledgeAuditLogDto,
    assets: Option<&[AdminKnowledgeReviewAssetDto]>,
    locale: Locale,
) -> String {
    if let Some(asset) = assets.and_then(|items| {
        items
            .iter()
            .find(|item| item.asset.id == log.target_id)
    }) {
        return asset.asset.title.clone();
    }
    format!(
        "{} · {}",
        platform_admin_target_type_label(&log.target_type, locale),
        short_reference(&log.target_id)
    )
}

fn audit_school_label(
    log: &KnowledgeAuditLogDto,
    assets: Option<&[AdminKnowledgeReviewAssetDto]>,
    locale: Locale,
) -> String {
    let Some(school_id) = log.school_id.as_deref() else {
        return admin_t("platform_admin.audit.none", locale);
    };
    if let Some(asset) = assets.and_then(|items| {
        items
            .iter()
            .find(|item| item.asset.school_id == school_id)
    }) {
        return asset.school_name.clone();
    }
    format!(
        "{} · {}",
        admin_t("platform_admin.metadata.school", locale),
        short_reference(school_id)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_size_labels_are_human_readable() {
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(2048), "2.0 KiB");
        assert_eq!(format_file_size(2 * 1024 * 1024), "2.0 MiB");
    }

    #[test]
    fn technical_references_are_secondary_and_short_in_primary_chrome() {
        assert_eq!(
            short_reference("a0000000-0000-0000-0000-0000000000a1"),
            "…000000a1"
        );
    }

    #[test]
    fn platform_admin_production_uses_shared_localized_readable_primitives() {
        let source = include_str!("platform_admin.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert!(production.contains("platform_admin_lifecycle_guidance"));
        assert!(production.contains("platform_admin_status_label"));
        assert!(production.contains("platform_admin_audit_action_label"));
        assert!(production.contains("format_product_datetime_text"));
        assert!(production.contains("item.school_name"));
        assert!(production.contains("text-start"));
        assert!(!production.contains("School {asset.school_id}"));
        assert!(!production.contains("metadata_line("));
        assert!(!production.contains("\"{log.action}\""));
        assert!(!production.contains("\"{log.created_at}\""));
        assert!(!production.contains("text-left"));
    }
}
