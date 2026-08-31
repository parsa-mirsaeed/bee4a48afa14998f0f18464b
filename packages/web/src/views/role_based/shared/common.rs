use crate::i18n::use_locale;
use crate::ui::{
    Button as UiButton, ButtonSize as UiButtonSize, ButtonVariant as UiButtonVariant,
    Card as UiCard, DataState, DataStateKind, Dialog, FeedbackTone, InlineAlert, Progress,
    StatusBadge,
};
use dioxus::prelude::*;

/// Keep academic grade tokens in their semantic LTR order inside RTL UI.
#[component]
pub fn GradeToken(value: String, class: Option<String>) -> Element {
    rsx! { bdi { dir: "ltr", class: format!("block {}", class.unwrap_or_default()), "{value}" } }
}

/// Compatibility loading indicator backed by the canonical progress primitive.
#[component]
pub fn LoadingSpinner() -> Element {
    let locale = use_locale();
    rsx! {
        div { class: "et-ui-compat-loading",
            Progress {
                label: locale.t("common.loading"),
                value: None,
            }
        }
    }
}

/// Compatibility error surface backed by the canonical feedback primitives.
#[component]
pub fn ErrorMessage(message: String, on_retry: Option<Callback<()>>) -> Element {
    let locale = use_locale();
    let action = on_retry.map(|retry| {
        rsx! {
            UiButton {
                label: locale.t("common.retry"),
                variant: UiButtonVariant::Secondary,
                onclick: move |_| retry.call(()),
            }
        }
    });

    rsx! {
        InlineAlert {
            title: locale.t("common.error"),
            message,
            tone: FeedbackTone::Danger,
            action,
        }
    }
}

/// Compatibility empty state backed by the canonical data-state primitive.
#[component]
pub fn EmptyState(
    icon: String,
    title: String,
    description: String,
    action_text: Option<String>,
    on_action: Option<Callback<()>>,
) -> Element {
    let _ = icon;
    let action = on_action.map(|callback| EventHandler::new(move |_| callback.call(())));
    rsx! {
        DataState {
            kind: DataStateKind::Empty,
            title,
            description,
            action_label: action_text,
            on_action: action,
        }
    }
}

#[component]
pub fn Badge(text: String, variant: BadgeVariant) -> Element {
    let tone = match variant {
        BadgeVariant::Success => FeedbackTone::Success,
        BadgeVariant::Warning => FeedbackTone::Warning,
        BadgeVariant::Error => FeedbackTone::Danger,
        BadgeVariant::Info => FeedbackTone::Info,
        BadgeVariant::Gray => FeedbackTone::Neutral,
    };
    rsx! { StatusBadge { label: text, tone } }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BadgeVariant {
    Success,
    Warning,
    Error,
    Info,
    Gray,
}

/// Compatibility card. Arbitrary legacy padding/class hooks remain accepted so
/// callers do not need a PR-3 visual migration yet, but the glass container is
/// replaced by the canonical surface.
#[component]
pub fn Card(
    title: Option<String>,
    children: Element,
    padding: Option<String>,
    className: Option<String>,
) -> Element {
    let extra_classes = className.unwrap_or_default();
    let padding_class = padding.unwrap_or_default();
    rsx! {
        UiCard {
            class: extra_classes,
            if let Some(card_title) = title {
                h3 { class: "et-ui-compat-card-title", "{card_title}" }
            }
            if padding_class.is_empty() {
                {children}
            } else {
                div { class: "{padding_class}", {children} }
            }
        }
    }
}

#[component]
pub fn Button(
    text: String,
    variant: ButtonVariant,
    size: ButtonSize,
    onclick: Callback<()>,
    disabled: Option<bool>,
    loading: Option<bool>,
    icon: Option<String>,
) -> Element {
    let variant = match variant {
        ButtonVariant::Primary => UiButtonVariant::Primary,
        ButtonVariant::Secondary => UiButtonVariant::Secondary,
        ButtonVariant::Success => UiButtonVariant::Success,
        ButtonVariant::Warning => UiButtonVariant::Warning,
        ButtonVariant::Danger => UiButtonVariant::Danger,
        ButtonVariant::Ghost => UiButtonVariant::Ghost,
    };
    let size = match size {
        ButtonSize::Small => UiButtonSize::Sm,
        ButtonSize::Medium => UiButtonSize::Md,
        ButtonSize::Large => UiButtonSize::Lg,
    };

    rsx! {
        UiButton {
            label: text,
            variant,
            size,
            disabled,
            pending: loading,
            icon,
            onclick: move |_| onclick.call(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Success,
    Warning,
    Danger,
    Ghost,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ButtonSize {
    Small,
    Medium,
    Large,
}

/// Compatibility modal backed by the canonical generated-ID dialog contract.
#[component]
pub fn Modal(title: String, open: bool, on_close: Callback<()>, children: Element) -> Element {
    let locale = use_locale();
    rsx! {
        Dialog {
            open,
            title,
            close_label: locale.t("common.close"),
            on_close: move |_| on_close.call(()),
            children,
        }
    }
}
