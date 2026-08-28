use crate::application::routing_service::{NavigationItem, RoutingService};
use crate::domain::User;
use crate::i18n::{use_locale, LanguageSwitcher, Locale};
use crate::ui::{NavItem, SidebarNav};
use dioxus::prelude::*;

/// Shared role-aware navigation. Desktop keeps it persistent; compact viewports
/// expose the exact same destinations in a keyboard-operable drawer.
#[component]
pub fn Sidebar(
    user: User,
    navigation_items: Vec<NavigationItem>,
    active_section: String,
    mobile_open: bool,
    on_close_mobile: EventHandler,
) -> Element {
    let locale = use_locale();
    let nav = use_navigator();
    let t_sign_out = locale.t("auth.sign_out");
    let t_portal = if locale.current() == Locale::Fa {
        "پرتال"
    } else {
        "Portal"
    };
    let sidebar_class = if mobile_open {
        "et-sidebar et-sidebar--mobile-open"
    } else {
        "et-sidebar"
    };
    let home_section = RoutingService::default_dashboard_section(&user).to_string();
    let ui_items = navigation_items
        .iter()
        .map(|item| NavItem {
            id: item.id.clone(),
            label: item.label.clone(),
            icon: item.icon.clone(),
            active: item.id == active_section,
        })
        .collect::<Vec<_>>();

    rsx! {
        aside {
            class: "{sidebar_class}",
            "aria-label": locale.t("navigation.primary"),
            onkeydown: move |event| {
                if mobile_open && event.key() == Key::Escape {
                    event.stop_propagation();
                    on_close_mobile.call(());
                }
            },

            div { class: "et-sidebar-brand",
                button {
                    class: "et-brand-button",
                    r#type: "button",
                    "aria-label": "EduTalent dashboard",
                    onclick: {
                        let nav = nav.clone();
                        let home_section = home_section.clone();
                        move |_| {
                            on_close_mobile.call(());
                            if home_section == "overview" {
                                nav.push(crate::Route::DashboardRoute {});
                            } else {
                                nav.push(crate::Route::DashboardSectionRoute { section: home_section.clone() });
                            }
                        }
                    },
                    span { class: "et-brand-mark", span { class: "material-icons-outlined", "aria-hidden": "true", "school" } }
                    span { class: "et-brand-copy",
                        span { class: "et-brand-name", "EduTalent" }
                        span { class: "et-brand-context", "{t_portal}" }
                    }
                }

                button {
                    class: "et-sidebar-mobile-close",
                    r#type: "button",
                    "aria-label": locale.t("navigation.close"),
                    onclick: move |_| on_close_mobile.call(()),
                    span { class: "material-icons-outlined", "aria-hidden": "true", "close" }
                }
            }

            div { class: "et-sidebar-nav",
                SidebarNav {
                    label: locale.t("navigation.primary"),
                    items: ui_items,
                    on_select: {
                        let nav = nav.clone();
                        move |section: String| {
                            on_close_mobile.call(());
                            if section == "overview" {
                                nav.push(crate::Route::DashboardRoute {});
                            } else {
                                nav.push(crate::Route::DashboardSectionRoute { section });
                            }
                            focus_main_content();
                        }
                    },
                }
            }

            div { class: "et-sidebar-footer",
                div { class: "et-user-summary",
                    div { class: "et-avatar", "{user.initials()}" }
                    div { class: "et-user-copy",
                        p { class: "et-user-name", "{user.display_name()}" }
                        p { class: "et-user-role", "{user.role.display_name()}" }
                    }
                }

                div { class: "et-sidebar-utilities",
                    div { class: "et-language-row", LanguageSwitcher { class: "text-sm".to_string() } }
                    button {
                        class: "et-logout-button",
                        r#type: "button",
                        onclick: move |_| {
                            let nav = use_navigator();
                            spawn(async move {
                                if let Err(error) = crate::application::SessionManager::clear_session().await {
                                    eprintln!("Logout error: {error:?}");
                                }
                                nav.replace(crate::Route::LoginPage {});
                            });
                        },
                        span { class: "material-icons-outlined text-lg", "aria-hidden": "true", "logout" }
                        span { "{t_sign_out}" }
                    }
                }
            }
        }
    }
}

fn focus_main_content() {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(element) = document.get_element_by_id("dashboard-main-content") {
                    if let Some(element) = element.dyn_ref::<web_sys::HtmlElement>() {
                        let _ = element.focus();
                    }
                }
            }
        }
    }
}
