use crate::i18n::{format_product_datetime_text, use_locale};
use api::server_functions::assignment_personalization_functions::get_assignment_personalization_status;
use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use gloo_timers::future::TimeoutFuture;

#[component]
pub fn PersonalizationQueueStatusPanel() -> Element {
    let locale = use_locale();
    let mut status =
        use_resource(move || async move { get_assignment_personalization_status().await });
    #[cfg(target_arch = "wasm32")]
    {
        let mut polling = use_signal(|| false);
        let mut observed_active = use_signal(|| false);
        use_effect(move || {
            let active = status.read().as_ref().is_some_and(|result| {
                result.as_ref().is_ok_and(|summary| summary.queued + summary.running > 0)
            });
            if !active {
                observed_active.set(false);
                return;
            }
            if polling() || observed_active() {
                return;
            }
            observed_active.set(true);
            polling.set(true);
            spawn(async move {
                for delay in [1_000_u32, 2_000, 4_000, 8_000, 10_000, 10_000] {
                    TimeoutFuture::new(delay).await;
                    let active = status.read().as_ref().is_some_and(|result| {
                        result.as_ref().is_ok_and(|summary| summary.queued + summary.running > 0)
                    });
                    if !active {
                        break;
                    }
                    status.restart();
                }
                polling.set(false);
            });
        });
    }
    let failed_read = status.read().as_ref().is_some_and(|result| result.is_err());

    rsx! {
        section {
            "data-testid": "teacher-personalization-status",
            class: "et-ui-card mb-4 space-y-3 p-4",
            div { class: "flex flex-wrap items-center justify-between gap-3",
                h3 { class: "font-semibold",
                    "{locale.t(\"teachers.assignments.create.ai_title\")} · {locale.t(\"common.status\")}"
                }
                button {
                    r#type: "button",
                    class: "et-ui-button et-ui-button--secondary et-ui-button--lg",
                    onclick: move |_| status.restart(),
                    if failed_read {
                        "{locale.t(\"common.retry\")}"
                    } else {
                        "{locale.t(\"teacher.assignments.refresh_status\")}"
                    }
                }
            }
            match &*status.read() {
                None => rsx! {
                    p { role: "status", class: "text-sm", "{locale.t(\"common.loading\")}" }
                },
                Some(Err(_)) => rsx! {
                    p { role: "alert", class: "text-sm", "{locale.t(\"teacher.assignments.status_read_failed\")}" }
                },
                Some(Ok(summary)) if summary.total == 0 => rsx! {
                    p { role: "status", class: "text-sm", "{locale.t(\"common.no_data\")}" }
                },
                Some(Ok(summary)) => rsx! {
                    div { role: "status", class: "flex flex-wrap gap-x-4 gap-y-2 text-sm",
                        span { "{summary.succeeded}/{summary.total} {locale.t(\"assignments.completed\")}" }
                        span { "{summary.queued} {locale.t(\"teacher.assignments.status_queued\")}" }
                        span { "{summary.running} {locale.t(\"teacher.assignments.status_running\")}" }
                        if summary.failed > 0 {
                            span { "{summary.failed} {locale.t(\"teacher.assignments.status_failed\")}" }
                        }
                        if summary.cancelled > 0 {
                            span { "{summary.cancelled} {locale.t(\"teacher.assignments.status_cancelled\")}" }
                        }
                    }
                    if let Some(timestamp) = &summary.last_completed_at {
                        p { class: "text-xs", "{locale.t(\"teacher.assignments.last_completed\")}: "
                            bdi { {format_product_datetime_text(timestamp, locale.current())} }
                        }
                    }
                    if let Some(reference) = &summary.latest_failed_job_id {
                        details { class: "rounded-lg border border-gray-300 p-3 dark:border-gray-700",
                            summary { class: "min-h-[44px] cursor-pointer font-medium", "{locale.t(\"teacher.assignments.failure_details\")}" }
                            p { class: "text-sm", "{locale.t(\"teacher.assignments.failure_help\")}" }
                            SupportReference { key: "{reference}", reference: reference.clone() }
                        }
                    }
                },
            }
        }
    }
}

#[component]
fn SupportReference(reference: String) -> Element {
    let locale = use_locale();
    let mut outcome = use_signal(|| None::<bool>);
    let short: String = reference.chars().take(8).collect();
    rsx! {
        div { "data-testid": "teacher-personalization-support-reference", class: "mt-2 space-y-2 text-sm",
            p {
                "{locale.t(\"teacher.assignments.support_reference\")}: "
                bdi { "{short}…" }
            }
            button {
                r#type: "button",
                class: "et-ui-button et-ui-button--secondary et-ui-button--lg",
                onclick: move |_| {
                    #[cfg(target_arch = "wasm32")]
                    {
                        use wasm_bindgen::JsCast;
                        let reference = reference.clone();
                        spawn(async move {
                            let clipboard = web_sys::window()
                                .and_then(|window| js_sys::Reflect::get(window.navigator().as_ref(), &"clipboard".into()).ok())
                                .and_then(|value| value.dyn_into::<web_sys::Clipboard>().ok());
                            let copied = match clipboard {
                                Some(clipboard) => wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&reference)).await.is_ok(),
                                None => false,
                            };
                            outcome.set(Some(copied));
                        });
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    outcome.set(Some(false));
                },
                "{locale.t(\"teacher.assignments.copy_reference\")}"
            }
            if let Some(copied) = outcome() {
                p { role: "status",
                    if copied {
                        "{locale.t(\"teacher.assignments.reference_copied\")}"
                    } else {
                        "{locale.t(\"teacher.assignments.reference_copy_failed\")}"
                    }
                }
                if !copied {
                    bdi { class: "break-all", "{reference}" }
                }
            }
        }
    }
}
