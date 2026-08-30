use crate::i18n::use_locale;
use crate::ui::{DataState, DataStateKind};
use dioxus::prelude::*;
use serde_json::Value;

#[component]
pub fn ErrorBoundary(children: Element, fallback: Option<Element>) -> Element {
    let mut error_state = use_signal(|| Option::<String>::None);
    let mut error_info = use_signal(|| Option::<Value>::None);
    let error_val = error_state.read().clone();

    rsx! {
        div { class: "error-boundary-wrapper",
            if let Some(error_message) = error_val {
                if let Some(fallback_component) = fallback {
                    {fallback_component}
                } else {
                    DefaultErrorUI {
                        error: error_message,
                        error_info: error_info.read().clone(),
                        on_retry: move |_| {
                            error_state.set(None);
                            error_info.set(None);
                        },
                    }
                }
            } else {
                {children}
            }
        }
    }
}

/// Logs diagnostic detail but exposes only localized, actionable copy.
#[component]
pub fn DefaultErrorUI(error: String, error_info: Option<Value>, on_retry: EventHandler) -> Element {
    let locale = use_locale();
    use_effect(move || {
        web_sys::console::error_1(&error.clone().into());
        if let Some(info) = error_info.as_ref() {
            web_sys::console::error_1(&info.to_string().into());
        }
    });

    rsx! {
        DataState {
            kind: DataStateKind::Error,
            title: locale.t("errors.generic_title"),
            description: locale.t("errors.generic_description"),
            action_label: locale.t("common.retry"),
            on_action: move |_| on_retry.call(()),
        }
    }
}

#[component]
pub fn NetworkError(message: Option<String>, on_retry: EventHandler) -> Element {
    let locale = use_locale();
    if let Some(message) = message {
        web_sys::console::warn_1(&message.into());
    }
    rsx! {
        DataState {
            kind: DataStateKind::Error,
            title: locale.t("errors.network_title"),
            description: locale.t("errors.network_description"),
            action_label: locale.t("common.retry"),
            on_action: move |_| on_retry.call(()),
        }
    }
}

#[component]
pub fn NotFoundError(resource: Option<String>, on_go_home: EventHandler) -> Element {
    let locale = use_locale();
    if let Some(resource) = resource {
        web_sys::console::warn_1(&format!("Requested resource not found: {resource}").into());
    }
    rsx! {
        DataState {
            kind: DataStateKind::Unavailable,
            title: locale.t("errors.not_found_title"),
            description: locale.t("errors.not_found_description"),
            action_label: locale.t("common.go_home"),
            on_action: move |_| on_go_home.call(()),
        }
    }
}

#[component]
pub fn PermissionDeniedError(
    resource: Option<String>,
    required_permission: Option<String>,
    on_go_back: EventHandler,
) -> Element {
    let locale = use_locale();
    if let Some(resource) = resource {
        web_sys::console::warn_1(&format!("Denied resource: {resource}").into());
    }
    if let Some(permission) = required_permission {
        web_sys::console::warn_1(&format!("Required permission: {permission}").into());
    }
    rsx! {
        DataState {
            kind: DataStateKind::Permission,
            title: locale.t("errors.access_denied"),
            description: locale.t("errors.destination_unavailable"),
            action_label: locale.t("common.go_back"),
            on_action: move |_| on_go_back.call(()),
        }
    }
}

pub struct ErrorBoundaryHooks;

impl ErrorBoundaryHooks {
    pub fn use_async_error_handler() -> EventHandler<String> {
        let mut error_state = use_signal(|| Option::<String>::None);
        Callback::new(move |error: String| {
            error_state.set(Some(error.clone()));
            web_sys::console::error_1(&error.into());
        })
    }

    pub fn use_clear_error() -> impl FnMut() {
        let mut error_state = use_signal(|| Option::<String>::None);
        move || {
            *error_state.write() = None;
        }
    }

    pub fn use_has_error() -> bool {
        let error_state = use_signal(|| Option::<String>::None);
        let has_error = error_state.read().is_some();
        has_error
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppError {
    NetworkError(String),
    ValidationError(String),
    AuthenticationError(String),
    AuthorizationError(String),
    NotFoundError(String),
    ServerError(String),
    UnknownError(String),
}

impl AppError {
    /// Stable category copy for UI decisions. The underlying detail remains
    /// available for diagnostics but is not returned to end-user surfaces.
    pub fn user_message(&self) -> String {
        match self {
            AppError::NetworkError(_) => "Network error".to_string(),
            AppError::ValidationError(_) => "Validation error".to_string(),
            AppError::AuthenticationError(_) => "Authentication error".to_string(),
            AppError::AuthorizationError(_) => "Access denied".to_string(),
            AppError::NotFoundError(_) => "Not found".to_string(),
            AppError::ServerError(_) => "Service unavailable".to_string(),
            AppError::UnknownError(_) => "Unexpected error".to_string(),
        }
    }

    pub fn error_type(&self) -> &str {
        match self {
            AppError::NetworkError(_) => "network",
            AppError::ValidationError(_) => "validation",
            AppError::AuthenticationError(_) => "authentication",
            AppError::AuthorizationError(_) => "authorization",
            AppError::NotFoundError(_) => "not_found",
            AppError::ServerError(_) => "server",
            AppError::UnknownError(_) => "unknown",
        }
    }
}

impl From<String> for AppError {
    fn from(value: String) -> Self {
        AppError::UnknownError(value)
    }
}

impl From<&str> for AppError {
    fn from(value: &str) -> Self {
        AppError::UnknownError(value.to_string())
    }
}
