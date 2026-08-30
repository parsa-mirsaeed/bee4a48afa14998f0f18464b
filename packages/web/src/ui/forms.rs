use dioxus::prelude::*;

fn stable_id(prefix: &str, key: &str) -> String {
    let slug: String = key
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug.trim_matches('-');
    let slug = if slug.is_empty() { "control" } else { slug };
    format!("et-{prefix}-{slug}")
}

fn described_by(
    hint_id: &str,
    error_id: &str,
    hint: &Option<String>,
    error: &Option<String>,
) -> String {
    match (hint.is_some(), error.is_some()) {
        (true, true) => format!("{hint_id} {error_id}"),
        (true, false) => hint_id.to_string(),
        (false, true) => error_id.to_string(),
        (false, false) => String::new(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
    pub disabled: bool,
}

impl SelectOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

#[component]
pub fn Field(
    control_id: String,
    label: String,
    hint: Option<String>,
    error: Option<String>,
    required: Option<bool>,
    children: Element,
) -> Element {
    let hint_id = format!("{control_id}-hint");
    let error_id = format!("{control_id}-error");
    rsx! {
        div { class: "et-ui-field",
            label { class: "et-ui-field__label", r#for: "{control_id}",
                span { "{label}" }
                if required.unwrap_or(false) {
                    span { class: "et-ui-field__required", "aria-hidden": "true", " *" }
                }
            }
            {children}
            if let Some(hint) = hint {
                p { id: "{hint_id}", class: "et-ui-field__hint", "{hint}" }
            }
            if let Some(error) = error {
                p { id: "{error_id}", class: "et-ui-field__error", role: "alert", "{error}" }
            }
        }
    }
}

#[component]
pub fn TextField(
    label: String,
    value: String,
    on_change: EventHandler<String>,
    name: Option<String>,
    input_type: Option<String>,
    autocomplete: Option<String>,
    placeholder: Option<String>,
    hint: Option<String>,
    error: Option<String>,
    required: Option<bool>,
    disabled: Option<bool>,
    leading_icon: Option<String>,
) -> Element {
    let input_id = stable_id("field", name.as_deref().unwrap_or(&label));
    let hint_id = format!("{input_id}-hint");
    let error_id = format!("{input_id}-error");
    let described_by = described_by(&hint_id, &error_id, &hint, &error);
    let invalid = error.is_some();
    let required = required.unwrap_or(false);
    let disabled = disabled.unwrap_or(false);
    let input_type = input_type.unwrap_or_else(|| "text".to_string());
    let name = name.unwrap_or_else(|| input_id.clone());

    rsx! {
        div { class: "et-ui-field",
            label { class: "et-ui-field__label", r#for: "{input_id}",
                span { "{label}" }
                if required { span { class: "et-ui-field__required", "aria-hidden": "true", " *" } }
            }
            div { class: if leading_icon.is_some() { "et-ui-control et-ui-control--icon" } else { "et-ui-control" },
                if let Some(ref icon) = leading_icon {
                    span { class: "material-icons-outlined et-ui-control__leading", "aria-hidden": "true", "{icon}" }
                }
                input {
                    id: "{input_id}",
                    class: "et-ui-input",
                    r#type: "{input_type}",
                    name: "{name}",
                    value: "{value}",
                    placeholder: placeholder.unwrap_or_default(),
                    autocomplete: autocomplete.unwrap_or_default(),
                    disabled,
                    "aria-required": if required { "true" } else { "false" },
                    "aria-invalid": if invalid { "true" } else { "false" },
                    "aria-describedby": if described_by.is_empty() { None } else { Some(described_by.as_str()) },
                    oninput: move |event| on_change.call(event.value()),
                }
            }
            if let Some(hint) = hint {
                p { id: "{hint_id}", class: "et-ui-field__hint", "{hint}" }
            }
            if let Some(error) = error {
                p { id: "{error_id}", class: "et-ui-field__error", role: "alert", "{error}" }
            }
        }
    }
}

#[component]
pub fn EmailField(
    label: String,
    value: String,
    on_change: EventHandler<String>,
    name: Option<String>,
    autocomplete: Option<String>,
    placeholder: Option<String>,
    hint: Option<String>,
    error: Option<String>,
    required: Option<bool>,
    disabled: Option<bool>,
) -> Element {
    rsx! {
        TextField {
            label,
            value,
            on_change: move |value| on_change.call(value),
            name,
            input_type: "email".to_string(),
            autocomplete,
            placeholder,
            hint,
            error,
            required,
            disabled,
            leading_icon: "mail_outline".to_string(),
        }
    }
}

#[component]
pub fn PasswordField(
    label: String,
    value: String,
    on_change: EventHandler<String>,
    reveal_label: String,
    hide_label: String,
    name: Option<String>,
    autocomplete: Option<String>,
    hint: Option<String>,
    error: Option<String>,
    required: Option<bool>,
    disabled: Option<bool>,
) -> Element {
    let input_id = stable_id("password", name.as_deref().unwrap_or(&label));
    let hint_id = format!("{input_id}-hint");
    let error_id = format!("{input_id}-error");
    let described_by = described_by(&hint_id, &error_id, &hint, &error);
    let mut revealed = use_signal(|| false);
    let required = required.unwrap_or(false);
    let disabled = disabled.unwrap_or(false);
    let invalid = error.is_some();
    let name = name.unwrap_or_else(|| input_id.clone());
    let input_type = if revealed() { "text" } else { "password" };
    let toggle_label = if revealed() { hide_label } else { reveal_label };

    rsx! {
        div { class: "et-ui-field",
            label { class: "et-ui-field__label", r#for: "{input_id}",
                span { "{label}" }
                if required { span { class: "et-ui-field__required", "aria-hidden": "true", " *" } }
            }
            div { class: "et-ui-control et-ui-control--icon et-ui-control--trailing",
                span { class: "material-icons-outlined et-ui-control__leading", "aria-hidden": "true", "lock_outline" }
                input {
                    id: "{input_id}",
                    class: "et-ui-input",
                    r#type: "{input_type}",
                    name: "{name}",
                    value: "{value}",
                    autocomplete: autocomplete.unwrap_or_default(),
                    disabled,
                    "aria-required": if required { "true" } else { "false" },
                    "aria-invalid": if invalid { "true" } else { "false" },
                    "aria-describedby": if described_by.is_empty() { None } else { Some(described_by.as_str()) },
                    oninput: move |event| on_change.call(event.value()),
                }
                button {
                    class: "et-ui-input-action",
                    r#type: "button",
                    disabled,
                    "aria-label": "{toggle_label}",
                    onclick: move |_| revealed.set(!revealed()),
                    span { class: "material-icons-outlined", "aria-hidden": "true", if revealed() { "visibility_off" } else { "visibility" } }
                }
            }
            if let Some(hint) = hint {
                p { id: "{hint_id}", class: "et-ui-field__hint", "{hint}" }
            }
            if let Some(error) = error {
                p { id: "{error_id}", class: "et-ui-field__error", role: "alert", "{error}" }
            }
        }
    }
}

#[component]
pub fn TextArea(
    label: String,
    value: String,
    on_change: EventHandler<String>,
    rows: Option<u8>,
    hint: Option<String>,
    error: Option<String>,
    required: Option<bool>,
    disabled: Option<bool>,
) -> Element {
    let input_id = stable_id("textarea", &label);
    let hint_id = format!("{input_id}-hint");
    let error_id = format!("{input_id}-error");
    let described_by = described_by(&hint_id, &error_id, &hint, &error);
    let required = required.unwrap_or(false);
    let disabled = disabled.unwrap_or(false);
    let invalid = error.is_some();
    let rows = rows.unwrap_or(5).to_string();

    rsx! {
        div { class: "et-ui-field",
            label { class: "et-ui-field__label", r#for: "{input_id}", "{label}" }
            textarea {
                id: "{input_id}",
                class: "et-ui-textarea",
                rows: "{rows}",
                value: "{value}",
                disabled,
                "aria-required": if required { "true" } else { "false" },
                "aria-invalid": if invalid { "true" } else { "false" },
                "aria-describedby": if described_by.is_empty() { None } else { Some(described_by.as_str()) },
                oninput: move |event| on_change.call(event.value()),
            }
            if let Some(hint) = hint { p { id: "{hint_id}", class: "et-ui-field__hint", "{hint}" } }
            if let Some(error) = error { p { id: "{error_id}", class: "et-ui-field__error", role: "alert", "{error}" } }
        }
    }
}

#[component]
pub fn Select(
    label: String,
    value: String,
    options: Vec<SelectOption>,
    on_change: EventHandler<String>,
    hint: Option<String>,
    error: Option<String>,
    required: Option<bool>,
    disabled: Option<bool>,
) -> Element {
    let input_id = stable_id("select", &label);
    let hint_id = format!("{input_id}-hint");
    let error_id = format!("{input_id}-error");
    let described_by = described_by(&hint_id, &error_id, &hint, &error);
    let required = required.unwrap_or(false);
    let disabled = disabled.unwrap_or(false);
    let invalid = error.is_some();

    rsx! {
        div { class: "et-ui-field",
            label { class: "et-ui-field__label", r#for: "{input_id}", "{label}" }
            select {
                id: "{input_id}",
                class: "et-ui-select",
                value: "{value}",
                disabled,
                "aria-required": if required { "true" } else { "false" },
                "aria-invalid": if invalid { "true" } else { "false" },
                "aria-describedby": if described_by.is_empty() { None } else { Some(described_by.as_str()) },
                onchange: move |event| on_change.call(event.value()),
                for option in options {
                    option { value: "{option.value}", disabled: option.disabled, selected: option.value == value, "{option.label}" }
                }
            }
            if let Some(hint) = hint { p { id: "{hint_id}", class: "et-ui-field__hint", "{hint}" } }
            if let Some(error) = error { p { id: "{error_id}", class: "et-ui-field__error", role: "alert", "{error}" } }
        }
    }
}

#[component]
pub fn Combobox(
    label: String,
    value: String,
    options: Vec<SelectOption>,
    on_change: EventHandler<String>,
    error: Option<String>,
    disabled: Option<bool>,
) -> Element {
    rsx! {
        Select {
            label,
            value,
            options,
            on_change: move |value| on_change.call(value),
            error,
            disabled,
        }
    }
}

#[component]
pub fn MultiSelect(
    label: String,
    selected: Vec<String>,
    options: Vec<SelectOption>,
    on_change: EventHandler<String>,
    hint: Option<String>,
    disabled: Option<bool>,
) -> Element {
    let input_id = stable_id("multi-select", &label);
    let hint_id = format!("{input_id}-hint");
    let disabled = disabled.unwrap_or(false);
    rsx! {
        div { class: "et-ui-field",
            label { class: "et-ui-field__label", r#for: "{input_id}", "{label}" }
            select {
                id: "{input_id}",
                class: "et-ui-select et-ui-select--multi",
                multiple: true,
                disabled,
                "aria-describedby": if hint.is_some() { Some(hint_id.as_str()) } else { None },
                onchange: move |event| on_change.call(event.value()),
                for option in options {
                    option {
                        value: "{option.value}",
                        disabled: option.disabled,
                        selected: selected.iter().any(|value| value == &option.value),
                        "{option.label}"
                    }
                }
            }
            if let Some(ref hint) = hint { p { id: "{hint_id}", class: "et-ui-field__hint", "{hint}" } }
        }
    }
}

#[component]
pub fn Checkbox(
    label: String,
    checked: bool,
    on_change: EventHandler<bool>,
    hint: Option<String>,
    error: Option<String>,
    disabled: Option<bool>,
) -> Element {
    let input_id = stable_id("checkbox", &label);
    let hint_id = format!("{input_id}-hint");
    let error_id = format!("{input_id}-error");
    let described_by = described_by(&hint_id, &error_id, &hint, &error);
    let disabled = disabled.unwrap_or(false);
    rsx! {
        div { class: "et-ui-field",
            label { class: "et-ui-check-row", r#for: "{input_id}",
                input {
                    id: "{input_id}",
                    class: "et-ui-checkbox",
                    r#type: "checkbox",
                    checked,
                    disabled,
                    "aria-invalid": if error.is_some() { "true" } else { "false" },
                    "aria-describedby": if described_by.is_empty() { None } else { Some(described_by.as_str()) },
                    onchange: move |event| on_change.call(event.checked()),
                }
                span { "{label}" }
            }
            if let Some(hint) = hint { p { id: "{hint_id}", class: "et-ui-field__hint", "{hint}" } }
            if let Some(ref error) = error { p { id: "{error_id}", class: "et-ui-field__error", role: "alert", "{error}" } }
        }
    }
}

#[component]
pub fn Switch(
    label: String,
    checked: bool,
    on_change: EventHandler<bool>,
    disabled: Option<bool>,
) -> Element {
    let input_id = stable_id("switch", &label);
    let disabled = disabled.unwrap_or(false);
    rsx! {
        label { class: "et-ui-switch", r#for: "{input_id}",
            input {
                id: "{input_id}",
                class: "et-ui-switch__input",
                r#type: "checkbox",
                role: "switch",
                checked,
                disabled,
                onchange: move |event| on_change.call(event.checked()),
            }
            span { class: "et-ui-switch__track", "aria-hidden": "true", span { class: "et-ui-switch__thumb" } }
            span { class: "et-ui-switch__label", "{label}" }
        }
    }
}

#[component]
pub fn RadioGroup(
    label: String,
    value: String,
    options: Vec<SelectOption>,
    on_change: EventHandler<String>,
    disabled: Option<bool>,
) -> Element {
    let group_id = stable_id("radio", &label);
    let disabled = disabled.unwrap_or(false);
    rsx! {
        fieldset { class: "et-ui-field", disabled,
            legend { class: "et-ui-field__label", "{label}" }
            div { class: "et-ui-radio-group",
                for (index, option) in options.into_iter().enumerate() {
                    {
                        let option_id = format!("{group_id}-{index}");
                        let option_value = option.value.clone();
                        rsx! {
                            label { class: "et-ui-check-row", r#for: "{option_id}",
                                input {
                                    id: "{option_id}",
                                    r#type: "radio",
                                    name: "{group_id}",
                                    value: "{option.value}",
                                    checked: option.value == value,
                                    disabled: option.disabled,
                                    onchange: move |_| on_change.call(option_value.clone()),
                                }
                                span { "{option.label}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn DateInput(
    label: String,
    value: String,
    on_change: EventHandler<String>,
    error: Option<String>,
    required: Option<bool>,
    disabled: Option<bool>,
) -> Element {
    rsx! {
        TextField {
            label,
            value,
            on_change: move |value| on_change.call(value),
            input_type: "date".to_string(),
            error,
            required,
            disabled,
        }
    }
}

#[component]
pub fn FileDropzone(
    label: String,
    accept: Option<String>,
    hint: Option<String>,
    disabled: Option<bool>,
    on_change: EventHandler<FormEvent>,
) -> Element {
    let input_id = stable_id("file", &label);
    let hint_id = format!("{input_id}-hint");
    let disabled = disabled.unwrap_or(false);
    rsx! {
        div { class: "et-ui-field",
            label { class: "et-ui-file-dropzone", r#for: "{input_id}",
                span { class: "material-icons-outlined", "aria-hidden": "true", "upload_file" }
                span { "{label}" }
                input {
                    id: "{input_id}",
                    class: "et-ui-file-dropzone__input",
                    r#type: "file",
                    accept: accept.unwrap_or_default(),
                    disabled,
                    "aria-describedby": if hint.is_some() { Some(hint_id.as_str()) } else { None },
                    onchange: move |event| on_change.call(event),
                }
            }
            if let Some(ref hint) = hint { p { id: "{hint_id}", class: "et-ui-field__hint", "{hint}" } }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::stable_id;

    #[test]
    fn shared_control_ids_are_stable_and_distinct_by_semantic_key() {
        let email_id = stable_id("field", "Email address");
        assert_eq!(email_id, "et-field-email-address");
        assert_eq!(email_id, stable_id("field", "Email address"));
        assert_ne!(email_id, stable_id("field", "Password"));
    }
}
