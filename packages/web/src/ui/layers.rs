use super::actions::{Button, ButtonVariant, DestructiveAction, IconButton};
use dioxus::prelude::*;
use uuid::Uuid;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

fn layer_id(prefix: &str) -> String {
    format!("et-{prefix}-{}", Uuid::new_v4().simple())
}

fn remember_active_element(mut return_focus_id: Signal<Option<String>>) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Some(active) = document.active_element() else {
        return;
    };

    let mut id = active.id();
    if id.is_empty() {
        id = layer_id("return-focus");
        active.set_id(&id);
    }
    return_focus_id.set(Some(id));
}

fn restore_active_element(return_focus_id: Signal<Option<String>>) {
    let Some(id) = return_focus_id.read().clone() else {
        return;
    };
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Some(element) = document.get_element_by_id(&id) else {
        return;
    };
    if let Ok(element) = element.dyn_into::<HtmlElement>() {
        let _ = element.focus();
    }
}

fn trap_tab(event: &KeyboardEvent, root_id: &str) {
    if event.key() != Key::Tab {
        return;
    }

    event.prevent_default();
    let backwards = event.modifiers().contains(Modifiers::SHIFT);
    let root_id = serde_json::to_string(root_id).unwrap_or_else(|_| "\"\"".to_string());
    let backwards = if backwards { "true" } else { "false" };

    let _ = document::eval(&format!(
        r#"(() => {{
            const root = document.getElementById({root_id});
            if (!root) return;
            const focusable = Array.from(root.querySelectorAll(
                'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [contenteditable="true"], [tabindex]:not([tabindex="-1"])'
            )).filter((element) =>
                !element.hasAttribute('hidden') &&
                element.getAttribute('aria-hidden') !== 'true' &&
                element.getClientRects().length > 0
            );
            if (focusable.length === 0) {{
                root.focus();
                return;
            }}
            const current = focusable.indexOf(document.activeElement);
            let next;
            if (current < 0) {{
                next = {backwards} ? focusable.length - 1 : 0;
            }} else {{
                next = (current + ({backwards} ? -1 : 1) + focusable.length) % focusable.length;
            }}
            focusable[next].focus();
        }})()"#
    ));
}

#[component]
pub fn Dialog(
    open: bool,
    title: String,
    on_close: EventHandler,
    children: Element,
    busy: Option<bool>,
    close_label: Option<String>,
) -> Element {
    let busy = busy.unwrap_or(false);
    let title_id = use_signal(|| layer_id("dialog-title")).read().clone();
    let dialog_id = use_signal(|| layer_id("dialog")).read().clone();
    let dialog_focus_root = dialog_id.clone();
    let return_focus_id = use_signal(|| None::<String>);
    let close_label = close_label.unwrap_or_else(|| "Close".to_string());

    rsx! {
        if open {
            div {
                class: "et-ui-layer",
                role: "presentation",
                onkeydown: move |event| {
                    if event.key() == Key::Escape && !busy {
                        event.stop_propagation();
                        on_close.call(());
                        restore_active_element(return_focus_id);
                    } else if event.key() == Key::Tab {
                        trap_tab(&event, &dialog_focus_root);
                    }
                },
                button {
                    class: "et-ui-layer__backdrop",
                    r#type: "button",
                    tabindex: "-1",
                    "aria-label": "{close_label}",
                    onclick: move |_| {
                        if !busy {
                            on_close.call(());
                            restore_active_element(return_focus_id);
                        }
                    },
                }
                div {
                    id: "{dialog_id}",
                    class: "et-ui-dialog",
                    role: "dialog",
                    "aria-modal": "true",
                    "aria-labelledby": "{title_id}",
                    tabindex: "-1",
                    onmounted: move |element| async move {
                        remember_active_element(return_focus_id);
                        let _ = element.data().set_focus(true).await;
                    },
                    div { class: "et-ui-dialog__header",
                        h2 { id: "{title_id}", class: "et-ui-dialog__title", "{title}" }
                        IconButton {
                            label: close_label.clone(),
                            icon: "close".to_string(),
                            disabled: busy,
                            onclick: move |_| {
                                on_close.call(());
                                restore_active_element(return_focus_id);
                            },
                        }
                    }
                    div { class: "et-ui-dialog__body", {children} }
                }
            }
        }
    }
}

#[component]
pub fn Drawer(
    open: bool,
    title: String,
    on_close: EventHandler,
    children: Element,
    close_label: Option<String>,
) -> Element {
    let title_id = use_signal(|| layer_id("drawer-title")).read().clone();
    let drawer_id = use_signal(|| layer_id("drawer")).read().clone();
    let drawer_focus_root = drawer_id.clone();
    let return_focus_id = use_signal(|| None::<String>);
    let close_label = close_label.unwrap_or_else(|| "Close navigation".to_string());
    rsx! {
        if open {
            div {
                class: "et-ui-layer et-ui-layer--drawer",
                onkeydown: move |event| {
                    if event.key() == Key::Escape {
                        event.stop_propagation();
                        on_close.call(());
                        restore_active_element(return_focus_id);
                    } else if event.key() == Key::Tab {
                        trap_tab(&event, &drawer_focus_root);
                    }
                },
                button {
                    class: "et-ui-layer__backdrop",
                    r#type: "button",
                    tabindex: "-1",
                    "aria-label": "{close_label}",
                    onclick: move |_| {
                        on_close.call(());
                        restore_active_element(return_focus_id);
                    },
                }
                aside {
                    id: "{drawer_id}",
                    class: "et-ui-drawer",
                    role: "dialog",
                    "aria-modal": "true",
                    "aria-labelledby": "{title_id}",
                    tabindex: "-1",
                    onmounted: move |element| async move {
                        remember_active_element(return_focus_id);
                        let _ = element.data().set_focus(true).await;
                    },
                    div { class: "et-ui-dialog__header",
                        h2 { id: "{title_id}", class: "et-ui-dialog__title", "{title}" }
                        IconButton {
                            label: close_label.clone(),
                            icon: "close".to_string(),
                            onclick: move |_| {
                                on_close.call(());
                                restore_active_element(return_focus_id);
                            },
                        }
                    }
                    div { class: "et-ui-drawer__body", {children} }
                }
            }
        }
    }
}

#[component]
pub fn Popover(
    open: bool,
    label: String,
    on_close: EventHandler,
    children: Element,
    class: Option<String>,
) -> Element {
    let class = class.unwrap_or_default();
    let return_focus_id = use_signal(|| None::<String>);

    rsx! {
        if open {
            div {
                class: "et-ui-popover {class}",
                role: "dialog",
                "aria-label": "{label}",
                tabindex: "-1",
                onmounted: move |element| async move {
                    remember_active_element(return_focus_id);
                    let _ = element.data().set_focus(true).await;
                },
                onkeydown: move |event| {
                    if event.key() == Key::Escape {
                        event.stop_propagation();
                        on_close.call(());
                        restore_active_element(return_focus_id);
                    }
                },
                {children}
            }
        }
    }
}

#[component]
pub fn ConfirmDialog(
    open: bool,
    title: String,
    description: String,
    confirm_label: String,
    cancel_label: String,
    on_confirm: EventHandler,
    on_cancel: EventHandler,
    pending: Option<bool>,
    destructive: Option<bool>,
) -> Element {
    let pending = pending.unwrap_or(false);
    let destructive = destructive.unwrap_or(true);
    rsx! {
        Dialog {
            open,
            title,
            busy: pending,
            on_close: move |_| on_cancel.call(()),
            children: rsx! {
                div { class: "et-ui-confirm",
                    p { class: "et-ui-confirm__description", "{description}" }
                    div { class: "et-ui-confirm__actions",
                        Button {
                            label: cancel_label,
                            variant: ButtonVariant::Secondary,
                            disabled: pending,
                            onclick: move |_| on_cancel.call(()),
                        }
                        if destructive {
                            DestructiveAction {
                                label: confirm_label,
                                pending,
                                onclick: move |_| on_confirm.call(()),
                            }
                        } else {
                            Button {
                                label: confirm_label,
                                pending,
                                onclick: move |_| on_confirm.call(()),
                            }
                        }
                    }
                }
            },
        }
    }
}
