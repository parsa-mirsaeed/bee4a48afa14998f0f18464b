use crate::ui::actions::{Button, ButtonVariant};
use dioxus::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackTone {
    Neutral,
    Info,
    Success,
    Warning,
    Danger,
}

impl FeedbackTone {
    fn class(self) -> &'static str {
        match self {
            Self::Neutral => "et-ui-tone--neutral",
            Self::Info => "et-ui-tone--info",
            Self::Success => "et-ui-tone--success",
            Self::Warning => "et-ui-tone--warning",
            Self::Danger => "et-ui-tone--danger",
        }
    }
}

#[component]
pub fn Toast(message: String, tone: Option<FeedbackTone>) -> Element {
    let tone = tone.unwrap_or(FeedbackTone::Neutral).class();
    rsx! { div { class: "et-ui-toast {tone}", role: "status", "{message}" } }
}

#[component]
pub fn InlineAlert(
    title: Option<String>,
    message: String,
    tone: Option<FeedbackTone>,
    action: Option<Element>,
) -> Element {
    let tone_value = tone.unwrap_or(FeedbackTone::Info);
    let tone = tone_value.class();
    let role = if tone_value == FeedbackTone::Danger {
        "alert"
    } else {
        "status"
    };
    rsx! {
        div { class: "et-ui-alert {tone}", role,
            div { class: "et-ui-alert__copy",
                if let Some(title) = title { p { class: "et-ui-alert__title", "{title}" } }
                p { class: "et-ui-alert__message", "{message}" }
            }
            if let Some(action) = action { div { class: "et-ui-alert__action", {action} } }
        }
    }
}

#[component]
pub fn StatusBanner(message: String, tone: Option<FeedbackTone>) -> Element {
    let tone = tone.unwrap_or(FeedbackTone::Neutral).class();
    rsx! { div { class: "et-ui-status-banner {tone}", role: "status", "{message}" } }
}

#[component]
pub fn Progress(label: String, value: Option<u8>) -> Element {
    if let Some(value) = value {
        let value = value.min(100);
        rsx! {
            div { class: "et-ui-progress-wrap",
                div { class: "et-ui-progress-copy", span { "{label}" } span { "{value}%" } }
                progress { class: "et-ui-progress", max: "100", value: "{value}" }
            }
        }
    } else {
        rsx! {
            div { class: "et-ui-progress-wrap", role: "status", "aria-label": "{label}",
                div { class: "et-ui-progress et-ui-progress--indeterminate", span {} }
            }
        }
    }
}

#[component]
pub fn Skeleton(label: Option<String>, lines: Option<u8>) -> Element {
    let label = label.unwrap_or_else(|| "Loading".to_string());
    let lines = lines.unwrap_or(3).clamp(1, 8);
    rsx! {
        div { class: "et-ui-skeleton", role: "status", "aria-label": "{label}",
            for index in 0..lines {
                div { key: "{index}", class: "et-ui-skeleton__line" }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataStateKind {
    Loading,
    Empty,
    Error,
    Permission,
    Unavailable,
}

impl DataStateKind {
    fn icon(self) -> &'static str {
        match self {
            Self::Loading => "hourglass_empty",
            Self::Empty => "inbox",
            Self::Error => "error_outline",
            Self::Permission => "lock_outline",
            Self::Unavailable => "cloud_off",
        }
    }
}

#[component]
pub fn DataState(
    kind: DataStateKind,
    title: String,
    description: String,
    action_label: Option<String>,
    on_action: Option<EventHandler>,
) -> Element {
    let icon = kind.icon();
    rsx! {
        div { class: "et-ui-data-state",
            span { class: "material-icons-outlined et-ui-data-state__icon", "aria-hidden": "true", "{icon}" }
            h2 { class: "et-ui-data-state__title", "{title}" }
            p { class: "et-ui-data-state__description", "{description}" }
            if let (Some(label), Some(action)) = (action_label, on_action) {
                Button {
                    label,
                    variant: ButtonVariant::Secondary,
                    onclick: move |_| action.call(()),
                }
            }
        }
    }
}

#[component]
pub fn GuideCard(
    title: String,
    description: String,
    step: Option<String>,
    actions: Option<Element>,
) -> Element {
    rsx! {
        article { class: "et-ui-guide-card",
            if let Some(step) = step { span { class: "et-ui-guide-card__step", "{step}" } }
            h3 { class: "et-ui-guide-card__title", "{title}" }
            p { class: "et-ui-guide-card__description", "{description}" }
            if let Some(actions) = actions { div { class: "et-ui-guide-card__actions", {actions} } }
        }
    }
}
