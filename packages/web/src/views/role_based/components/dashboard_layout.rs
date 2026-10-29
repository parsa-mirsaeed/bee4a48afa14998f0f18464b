use super::{Header, Sidebar};
use crate::application::routing_service::RoutingService;
use crate::domain::User;
use crate::i18n::use_locale;
use crate::ui::AppShell;
use dioxus::prelude::*;

const DASHBOARD_TABLET_CSS: Asset = asset!("/assets/dashboard-tablet.css");
const EXPERIENCE_2026_CSS: Asset = asset!("/assets/experience-2026.css");

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

    let page_title = navigation_items
        .iter()
        .find(|item| item.id == active_section)
        .map(|item| item.label.clone())
        .unwrap_or_else(|| locale.t("nav.dashboard"));

    rsx! {
        document::Stylesheet { href: DASHBOARD_TABLET_CSS }
        document::Stylesheet { href: EXPERIENCE_2026_CSS }
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
                        on_open_navigation: move |_| {
                            mobile_nav_open.set(true);
                            focus_open_mobile_drawer_after_render(12);
                        },
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

/// Opening the compact drawer is a state transition, not a DOM mount event.
/// On slow hydration or reduced-motion pages its close button can mount while
/// the aside is still hidden; focusing that invisible node silently fails.
/// Re-try in bounded animation frames, and stop as soon as focus succeeds.
fn focus_open_mobile_drawer_after_render(frames_left: u8) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::{closure::Closure, JsCast};

        let callback = Closure::once_into_js(move |_: f64| {
            let Some(document) = web_sys::window().and_then(|window| window.document()) else {
                return;
            };

            let Ok(sidebar) = document.query_selector(".et-sidebar--mobile-open") else {
                return;
            };
            // The user may have already dismissed navigation. Never refocus
            // a hidden drawer after Escape or navigation.
            if let Some(sidebar) = sidebar {
                if let Ok(Some(button)) = sidebar.query_selector(".et-sidebar-mobile-close") {
                    if let Ok(button) = button.dyn_into::<web_sys::HtmlElement>() {
                        let _ = button.focus();
                        let focused = document
                            .active_element()
                            .and_then(|element| element.get_attribute("class"))
                            .is_some_and(|value| {
                                value
                                    .split_whitespace()
                                    .any(|class| class == "et-sidebar-mobile-close")
                            });
                        if focused {
                            return;
                        }
                    }
                }
            }

            if frames_left > 0 {
                focus_open_mobile_drawer_after_render(frames_left - 1);
            }
        });

        if let Some(window) = web_sys::window() {
            let _ = window.request_animation_frame(callback.unchecked_ref());
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    let _ = frames_left;
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
