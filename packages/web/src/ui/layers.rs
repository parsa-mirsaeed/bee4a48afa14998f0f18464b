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

    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Some(root) = document.get_element_by_id(root_id) else {
        return;
    };
    let Ok(nodes) = root.query_selector_all(
        "a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [contenteditable=\"true\"], [tabindex]:not([tabindex=\"-1\"])",
    ) else {
        return;
    };

    let mut focusable = Vec::new();
    for index in 0..nodes.length() {
        let Some(node) = nodes.item(index) else {
            continue;
        };
        let Ok(element) = node.dyn_into::<HtmlElement>() else {
            continue;
        };
        if element.has_attribute("hidden")
            || element.get_attribute("aria-hidden").as_deref() == Some("true")
            || (element.offset_width() == 0 && element.offset_height() == 0)
        {
            continue;
        }
        focusable.push(element);
    }

    if focusable.is_empty() {
        event.prevent_default();
        if let Ok(root) = root.dyn_into::<HtmlElement>() {
            let _ = root.focus();
        }
        return;
    }

    let backwards = event.modifiers().contains(Modifiers::SHIFT);
    let active_index = document.active_element().and_then(|active| {
        focusable.iter().position(|candidate| {
            js_sys::Object::is(active.as_ref(), candidate.as_ref())
        })
    });

    let target = match active_index {
        None => {
            if backwards {
                focusable.last()
            } else {
                focusable.first()
            }
        }
        Some(0) if backwards => focusable.last(),
        Some(index) if !backwards && index + 1 == focusable.len() => focusable.first(),
        _ => None,
    };

    if let Some(target) = target {
        event.prevent_default();
        let _ = target.focus();
    }
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
