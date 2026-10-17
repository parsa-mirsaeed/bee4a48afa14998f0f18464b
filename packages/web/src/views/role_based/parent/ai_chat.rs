//! Ephemeral, child-scoped Parent AI guidance. Never persist the transcript or
//! send another child's prior questions after the child selector changes.
use crate::i18n::use_locale;
use api::server_functions::parent_scoped_functions::{
    ask_parent_academic_chat, get_parent_children_scoped, ParentAcademicChatRequest,
};
use dioxus::prelude::*;

#[component]
pub fn ParentAcademicChatSection() -> Element {
    let locale = use_locale();
    let mut children = use_resource(move || async move { get_parent_children_scoped().await });
    let mut selected_child = use_signal(|| None::<String>);
    let mut question = use_signal(String::new);
    let mut messages = use_signal(|| Vec::<(String, String, String)>::new());
    let mut consented = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| false);

    rsx! {
        div { class: "et-page-stack",
            header { class: "et-overview-intro",
                h2 { class: "et-overview-title", "{locale.t(\"parent.ai.title\")}" }
                p { class: "et-overview-copy", "{locale.t(\"parent.ai.intro\")}" }
            }
            p { class: "text-sm text-gray-600 dark:text-gray-300", role: "note",
                "{locale.t(\"parent.ai.privacy\")}"
            }
            match children.read().as_ref() {
                None => rsx! { p { role: "status", "{locale.t(\"parent.dashboard.loading\")}" } },
                Some(Err(_)) => rsx! {
                    div { role: "alert",
                        p { "{locale.t(\"parent.children.load_error\")}" }
                        button { r#type: "button", onclick: move |_| children.restart(),
                            "{locale.t(\"common.retry\")}"
                        }
                    }
                },
                Some(Ok(items)) if items.is_empty() => rsx! {
                    p { role: "status", "{locale.t(\"parent.dashboard.empty_description\")}" }
                },
                Some(Ok(items)) => rsx! {
                    section { class: "et-panel space-y-4",
                        label { class: "block text-sm font-semibold",
                            "{locale.t(\"parent.ai.select_child\")}"
                            select {
                                class: "mt-2 w-full rounded-lg border border-gray-300 bg-white p-2 dark:border-gray-700 dark:bg-gray-900",
                                value: selected_child().unwrap_or_default(),
                                disabled: busy(),
                                onchange: move |event| {
                                    let new_id = event.value();
                                    if selected_child().as_deref() != Some(new_id.as_str()) {
                                        selected_child.set(if new_id.is_empty() { None } else { Some(new_id) });
                                        question.set(String::new());
                                        consented.set(false);
                                        messages.set(Vec::new());
                                        error.set(false);
                                    }
                                },
                                option { value: "", "{locale.t(\"parent.ai.choose_child\")}" }
                                for child in items.iter() {
                                    option { value: "{child.id}", "{child.name}" }
                                }
                            }
                        }
                        if selected_child().is_some() {
                            div { class: "space-y-3", role: "log", "aria-label": locale.t("parent.ai.history"),
                                for (i, (asked, answered, time)) in messages().iter().enumerate() {
                                    article { key: "{i}", class: "rounded-lg border border-gray-200 p-3 dark:border-gray-700",
                                        p { class: "font-medium whitespace-pre-wrap break-words", "{asked}" }
                                        p { class: "mt-2 whitespace-pre-wrap break-words", "{answered}" }
                                        p { class: "mt-2 text-xs text-gray-500", dir: "ltr", "{time}" }
                                    }
                                }
                            }
                            if messages().is_empty() {
                                p { class: "text-sm text-gray-500", "{locale.t(\"parent.ai.empty\")}" }
                            }
                            div { class: "flex flex-wrap gap-2",
                                button {
                                    r#type: "button",
                                    class: "rounded-lg border px-3 py-2 text-sm",
                                    disabled: busy(),
                                    onclick: move |_| question.set(locale.t("parent.ai.example_missing").to_string()),
                                    "{locale.t(\"parent.ai.example_missing\")}"
                                }
                                button {
                                    r#type: "button",
                                    class: "rounded-lg border px-3 py-2 text-sm",
                                    disabled: busy(),
                                    onclick: move |_| question.set(locale.t("parent.ai.example_home").to_string()),
                                    "{locale.t(\"parent.ai.example_home\")}"
                                }
                            }
                            label { class: "block text-sm font-semibold",
                                "{locale.t(\"parent.ai.question\")}"
                                textarea {
                                    class: "mt-2 min-h-24 w-full rounded-lg border border-gray-300 bg-white p-3 dark:border-gray-700 dark:bg-gray-900",
                                    maxlength: "800",
                                    value: "{question}",
                                    disabled: busy(),
                                    oninput: move |event| question.set(event.value()),
                                }
                            }
                            label { class: "flex items-start gap-2 text-sm",
                                input {
                                    r#type: "checkbox",
                                    checked: consented(),
                                    disabled: busy(),
                                    onchange: move |event| consented.set(event.checked()),
                                }
                                span { "{locale.t(\"parent.ai.consent\")}" }
                            }
                            button {
                                r#type: "button",
                                class: "rounded-lg bg-primary px-4 py-2 font-semibold text-white disabled:opacity-50",
                                disabled: busy() || !consented() || question().trim().is_empty(),
                                onclick: move |_| {
                                    if busy() || !consented() { return; }
                                    let Some(child_id) = selected_child() else { return; };
                                    let current_question = question().trim().to_string();
                                    if current_question.is_empty() || current_question.chars().count() > 800 {
                                        error.set(true);
                                        return;
                                    }
                                    let previous = messages().iter().rev().take(3)
                                        .map(|(prior, _, _)| prior.clone())
                                        .collect::<Vec<_>>()
                                        .into_iter().rev().collect();
                                    busy.set(true);
                                    error.set(false);
                                    let req = ParentAcademicChatRequest {
                                        child_id, question: current_question.clone(),
                                        prior_questions: previous,
                                        locale: match locale.current() {
                                            crate::i18n::Locale::Fa => "fa",
                                            _ => "en",
                                        }.to_string(),
                                    };
                                    spawn(async move {
                                        match ask_parent_academic_chat(req).await {
                                            Ok(reply) => {
                                                messages.write().push((
                                                    current_question, reply.answer, reply.as_of,
                                                ));
                                                // Retain a small ephemeral display history.
                                                if messages().len() > 6 {
                                                    messages.write().remove(0);
                                                }
                                                question.set(String::new());
                                            }
                                            Err(_) => error.set(true),
                                        }
                                        busy.set(false);
                                    });
                                },
                                if busy() { "{locale.t(\"parent.ai.generating\")}" }
                                else { "{locale.t(\"parent.ai.ask\")}" }
                            }
                            if error() {
                                p { class: "text-sm text-red-700 dark:text-red-300", role: "alert",
                                    "{locale.t(\"parent.ai.unavailable\")}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
