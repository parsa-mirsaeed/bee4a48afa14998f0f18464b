use dioxus::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Success,
    Warning,
    Ghost,
    Danger,
}

impl ButtonVariant {
    fn class(self) -> &'static str {
        match self {
            Self::Primary => "et-ui-button--primary",
            Self::Secondary => "et-ui-button--secondary",
            Self::Success => "et-ui-button--success",
            Self::Warning => "et-ui-button--warning",
            Self::Ghost => "et-ui-button--ghost",
            Self::Danger => "et-ui-button--danger",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonSize {
    Sm,
    Md,
    Lg,
}

impl ButtonSize {
    fn class(self) -> &'static str {
        match self {
            Self::Sm => "et-ui-button--sm",
            Self::Md => "et-ui-button--md",
            Self::Lg => "et-ui-button--lg",
        }
    }
}

#[component]
pub fn Button(
    label: String,
    onclick: EventHandler,
    variant: Option<ButtonVariant>,
    size: Option<ButtonSize>,
    disabled: Option<bool>,
    pending: Option<bool>,
    icon: Option<String>,
    button_type: Option<String>,
) -> Element {
    let variant = variant.unwrap_or(ButtonVariant::Primary);
    let size = size.unwrap_or(ButtonSize::Md);
    let pending = pending.unwrap_or(false);
    let disabled = disabled.unwrap_or(false) || pending;
    let button_type = button_type.unwrap_or_else(|| "button".to_string());
    let variant_class = variant.class();
    let size_class = size.class();

    rsx! {
        button {
            class: "et-ui-button {variant_class} {size_class}",
            r#type: "{button_type}",
            disabled,
            "aria-busy": if pending { "true" } else { "false" },
            onclick: move |_| {
                if !disabled {
                    onclick.call(());
                }
            },
            if pending {
                span { class: "et-ui-spinner", "aria-hidden": "true" }
            } else if let Some(icon) = icon {
                span { class: "material-icons-outlined et-ui-button__icon", "aria-hidden": "true", "{icon}" }
            }
            span { "{label}" }
        }
    }
}

#[component]
pub fn IconButton(
    label: String,
    icon: String,
    onclick: EventHandler,
    expanded: Option<bool>,
    disabled: Option<bool>,
) -> Element {
    let disabled = disabled.unwrap_or(false);
    rsx! {
        button {
            class: "et-ui-icon-button",
            r#type: "button",
            "aria-label": "{label}",
            "aria-expanded": expanded.map(|value| if value { "true" } else { "false" }),
            disabled,
            onclick: move |_| {
                if !disabled {
                    onclick.call(());
                }
            },
            span { class: "material-icons-outlined", "aria-hidden": "true", "{icon}" }
        }
    }
}

#[component]
pub fn DropdownMenu(label: String, open: bool, children: Element) -> Element {
    rsx! {
        div {
            class: if open { "et-ui-menu et-ui-menu--open" } else { "et-ui-menu" },
            role: "menu",
            "aria-label": "{label}",
            hidden: !open,
            {children}
        }
    }
}

#[component]
pub fn SplitButton(
    label: String,
    on_primary: EventHandler,
    menu_label: String,
    on_menu: EventHandler,
    pending: Option<bool>,
) -> Element {
    let pending = pending.unwrap_or(false);
    rsx! {
        div { class: "et-ui-split-button",
            Button {
                label,
                onclick: move |_| on_primary.call(()),
                variant: ButtonVariant::Primary,
                pending,
            }
            IconButton {
                label: menu_label,
                icon: "arrow_drop_down".to_string(),
                onclick: move |_| on_menu.call(()),
                disabled: pending,
            }
        }
    }
}

#[component]
pub fn DestructiveAction(
    label: String,
    onclick: EventHandler,
    pending: Option<bool>,
    disabled: Option<bool>,
) -> Element {
    rsx! {
        Button {
            label,
            onclick: move |_| onclick.call(()),
            variant: ButtonVariant::Danger,
            pending,
            disabled,
            icon: "delete_outline".to_string(),
        }
    }
}
