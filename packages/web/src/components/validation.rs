use crate::i18n::use_locale;
use crate::ui::{FeedbackTone, Field, StatusBanner};
use crate::utils::validation::FormValidationState;
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub fn FieldValidationErrors(
    field_name: String,
    validation_state: Signal<FormValidationState>,
    #[props(default)] show_label: bool,
) -> Element {
    let validation = validation_state.read();
    if let Some(error_message) = validation.get_field_error(&field_name) {
        rsx! {
            p { class: "et-ui-field__error", role: "alert", "{error_message}" }
        }
    } else if show_label && validation.is_dirty && validation.is_valid {
        let locale = use_locale();
        rsx! {
            StatusBanner {
                message: locale.t("validation.valid"),
                tone: FeedbackTone::Success,
            }
        }
    } else {
        rsx! {}
    }
}

#[component]
pub fn ValidatedTextInput(
    field_name: String,
    label: String,
    value: Signal<String>,
    validation_state: Signal<FormValidationState>,
    #[props(default)] placeholder: String,
    #[props(default)] required: bool,
    #[props(default)] disabled: bool,
    #[props(default)] input_type: String,
    #[props(default)] max_length: Option<usize>,
) -> Element {
    let control_id = use_signal(|| format!("et-validated-{}", Uuid::new_v4().simple()))
        .read()
        .clone();
    let error = validation_state.read().get_field_error(&field_name);
    let invalid = error.is_some();
    let input_type = if input_type.is_empty() {
        "text".to_string()
    } else {
        input_type
    };

    rsx! {
        Field {
            control_id: control_id.clone(),
            label,
            required,
            error: error.clone(),
            children: rsx! {
                input {
                    id: "{control_id}",
                    class: "et-ui-input",
                    r#type: "{input_type}",
                    placeholder,
                    value: "{value}",
                    disabled,
                    maxlength: max_length.map(|value| value.to_string()),
                    "aria-required": if required { "true" } else { "false" },
                    "aria-invalid": if invalid { "true" } else { "false" },
                    "aria-describedby": if invalid { Some(format!("{control_id}-error")) } else { None },
                    oninput: move |event| value.set(event.value()),
                    onblur: move |_| {
                        validation_state.write().is_dirty = true;
                    },
                }
            },
        }
    }
}

#[component]
pub fn ValidatedSelectInput(
    field_name: String,
    label: String,
    value: Signal<String>,
    validation_state: Signal<FormValidationState>,
    options: Vec<(String, String)>,
    #[props(default)] placeholder: String,
    #[props(default)] required: bool,
    #[props(default)] disabled: bool,
) -> Element {
    let control_id = use_signal(|| format!("et-validated-select-{}", Uuid::new_v4().simple()))
        .read()
        .clone();
    let error = validation_state.read().get_field_error(&field_name);
    let invalid = error.is_some();

    rsx! {
        Field {
            control_id: control_id.clone(),
            label,
            required,
            error: error.clone(),
            children: rsx! {
                select {
                    id: "{control_id}",
                    class: "et-ui-select",
                    disabled,
                    value: "{value}",
                    "aria-required": if required { "true" } else { "false" },
                    "aria-invalid": if invalid { "true" } else { "false" },
                    "aria-describedby": if invalid { Some(format!("{control_id}-error")) } else { None },
                    onchange: move |event| value.set(event.value()),
                    onblur: move |_| {
                        validation_state.write().is_dirty = true;
                    },
                    if !placeholder.is_empty() {
                        option {
                            value: "",
                            selected: value.read().is_empty(),
                            disabled: required,
                            "{placeholder}"
                        }
                    }
                    for (option_value, option_label) in options {
                        option {
                            value: "{option_value}",
                            selected: *value.read() == option_value,
                            "{option_label}"
                        }
                    }
                }
            },
        }
    }
}

#[component]
pub fn ValidationSummary(validation_state: Signal<FormValidationState>) -> Element {
    let validation = validation_state.read();
    let locale = use_locale();

    if validation.errors.is_empty() {
        if validation.is_dirty && validation.is_valid {
            rsx! {
                StatusBanner {
                    message: locale.t("validation.all_valid"),
                    tone: FeedbackTone::Success,
                }
            }
        } else {
            rsx! {}
        }
    } else {
        let details = validation
            .errors
            .iter()
            .map(|error| error.message.as_str())
            .collect::<Vec<_>>()
            .join(" · ");
        rsx! {
            div { class: "et-ui-alert et-ui-tone--danger", role: "alert",
                div { class: "et-ui-alert__copy",
                    p { class: "et-ui-alert__title", "{locale.t(\"validation.fix_errors\")}" }
                    p { class: "et-ui-alert__message", "{details}" }
                }
            }
        }
    }
}
