use super::{Header, Sidebar};
use crate::application::routing_service::RoutingService;
use crate::domain::User;
use crate::i18n::use_locale;
use crate::ui::AppShell;
use dioxus::prelude::*;

/// Shared responsive product shell for every authenticated role.
///
/// Desktop uses a persistent logical-inline sidebar. Tablet and mobile use the
/// same navigation model as a drawer, so destinations and authorization do not
/// diverge by viewport.
#[component]
pub fn DashboardLayout(user: User, active_section: String, children: Element) -> Element {
    let locale = use_locale();
    let mut mobile_nav_open = use_signal(|| false);
    let navigation_items = RoutingService::get_navigation_items(&user, &locale);

    use_effect(move || {
        if mobile_nav_open() {
            focus_selector(".et-sidebar-mobile-close");
        }
    });

    let page_title = navigation_items
        .iter()
        .find(|item| item.id == active_section)
        .map(|item| item.label.clone())
        .unwrap_or_else(|| locale.t("nav.dashboard"));

    rsx! {
        AppShell {
            div { class: "et-dashboard-shell",
                if mobile_nav_open() {
                    button {
                        class: "et-mobile-overlay",
                        r#type: "button",
                        "aria-label": locale.t("navigation.close"),
                        onclick: move |_| {
                            mobile_nav_open.set(false);
                            focus_selector(".et-mobile-menu-button");
                        },
                    }
                }

                Sidebar {
                    user: user.clone(),
                    navigation_items,
                    active_section: active_section.clone(),
                    mobile_open: mobile_nav_open(),
                    on_close_mobile: move |_| {
                        mobile_nav_open.set(false);
                        focus_selector(".et-mobile-menu-button");
                    },
                }

                div { class: "et-dashboard-main",
                    Header {
                        user,
                        page_title,
                        on_open_navigation: move |_| mobile_nav_open.set(true),
                    }

                    main {
                        class: "et-dashboard-content",
                        id: "dashboard-main-content",
                        tabindex: "-1",
                        div { class: "et-dashboard-content-inner", {children} }
                    }
                }
            }
        }
    }
}

fn focus_selector(selector: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Ok(Some(element)) = document.query_selector(selector) {
                    if let Some(element) = element.dyn_ref::<web_sys::HtmlElement>() {
                        let _ = element.focus();
                    }
                }
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    let _ = selector;
}

#[component]
pub fn MobileDashboardLayout(user: User, active_section: String, children: Element) -> Element {
    rsx! { DashboardLayout { user, active_section, children } }
}

#[component]
pub fn ResponsiveDashboardLayout(user: User, active_section: String, children: Element) -> Element {
    rsx! { DashboardLayout { user, active_section, children } }
}

#[component]
pub fn DashboardSection(title: String, description: Option<String>, children: Element) -> Element {
    rsx! {
        section { class: "et-section",
            if !title.is_empty() {
                div { class: "et-section-heading",
                    div {
                        h2 { class: "et-section-title", "{title}" }
                        if let Some(desc) = description {
                            p { class: "et-section-support", "{desc}" }
                        }
                    }
                }
            }
            {children}
        }
    }
}
