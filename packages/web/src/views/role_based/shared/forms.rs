use crate::i18n::use_locale;
use crate::ui::{
    Checkbox as UiCheckbox, IconButton, Select as UiSelect, SelectOption as UiSelectOption,
    TextArea as UiTextArea, TextField as UiTextField,
};
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub fn FormInput(
    label: String,
    name: String,
    value: String,
    input_type: Option<String>,
    placeholder: Option<String>,
    required: Option<bool>,
    disabled: Option<bool>,
    error: Option<String>,
    on_change: Callback<String>,
) -> Element {
    rsx! {
        UiTextField {
            label,
            name,
            value,
            input_type,
            placeholder,
            required,
            disabled,
            error,
            on_change: move |next| on_change.call(next),
        }
    }
}

#[component]
pub fn FormSelect(
    label: String,
    name: String,
    value: String,
    options: Vec<SelectOption>,
    required: Option<bool>,
    disabled: Option<bool>,
    error: Option<String>,
    on_change: Callback<String>,
) -> Element {
    let _ = name;
    let options = options
        .into_iter()
        .map(|option| UiSelectOption::new(option.value, option.label))
        .collect();
    rsx! {
        UiSelect {
            label,
            value,
            options,
            required,
            disabled,
            error,
            on_change: move |next| on_change.call(next),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

impl SelectOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}

#[component]
pub fn FormTextarea(
    label: String,
    name: String,
    value: String,
    placeholder: Option<String>,
    required: Option<bool>,
    disabled: Option<bool>,
    rows: Option<i32>,
    error: Option<String>,
    on_change: Callback<String>,
) -> Element {
    let _ = (name, placeholder);
    let rows = rows.map(|value| value.clamp(1, u8::MAX as i32) as u8);
    rsx! {
        UiTextArea {
            label,
            value,
            rows,
            required,
            disabled,
            error,
            on_change: move |next| on_change.call(next),
        }
    }
}

#[component]
pub fn FormCheckbox(
    label: String,
    name: String,
    checked: bool,
    required: Option<bool>,
    disabled: Option<bool>,
    error: Option<String>,
    on_change: Callback<bool>,
) -> Element {
    let _ = (name, required);
    rsx! {
        UiCheckbox {
            label,
            checked,
            disabled,
            error,
            on_change: move |next| on_change.call(next),
        }
    }
}

/// Accessible compatibility search field. The label is visually hidden but is
/// still bound to the generated input ID; layout uses logical CSS properties.
#[component]
pub fn SearchBox(
    placeholder: String,
    value: String,
    on_change: Callback<String>,
    on_clear: Option<Callback<()>>,
) -> Element {
    let locale = use_locale();
    let input_id = use_signal(|| format!("et-search-{}", Uuid::new_v4().simple()))
        .read()
        .clone();

    rsx! {
        div { class: "et-ui-searchbox",
            label { class: "sr-only", r#for: "{input_id}", "{locale.t(\"common.search\")}" }
            span { class: "material-icons-outlined et-ui-searchbox__icon", "aria-hidden": "true", "search" }
            input {
                id: "{input_id}",
                class: "et-ui-input",
                r#type: "search",
                value: "{value}",
                placeholder,
                oninput: move |event| on_change.call(event.value()),
            }
            if !value.is_empty() {
                div { class: "et-ui-searchbox__clear",
                    IconButton {
                        label: locale.t("common.clear"),
                        icon: "close".to_string(),
                        onclick: move |_| {
                            if let Some(clear) = on_clear {
                                clear.call(());
                            }
                        },
                    }
                }
            }
        }
    }
}
