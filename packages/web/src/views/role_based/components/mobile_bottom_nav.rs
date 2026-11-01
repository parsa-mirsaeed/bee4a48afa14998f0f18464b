use super::sidebar::focus_main_content_after_navigation;
use crate::application::routing_service::{NavigationItem, RoutingService};
use crate::domain::{SystemRole, User};
use crate::i18n::use_locale;
use dioxus::prelude::*;

/// Only select destinations already exposed by the canonical role navigation.
/// This is a compact view of existing routes, never a second authorization map.
fn primary_mobile_items(user: &User, available: &[NavigationItem]) -> Vec<NavigationItem> {
    let preferred: &[&str] = match user.role {
        SystemRole::Student => &["overview", "classes", "assignments", "grades"],
        SystemRole::Teacher => &["overview", "classes", "assignments", "submissions"],
        SystemRole::Parent => &["overview", "children", "ai-chat", "reports"],
        SystemRole::SchoolManager => &["overview", "users", "classes", "knowledge-submissions"],
        SystemRole::PlatformAdmin => &["knowledge-assets", "knowledge-audit", "personalization"],
    };

    preferred
        .iter()
        .filter_map(|id| {
            available
                .iter()
                .find(|item| {
                    item.id == *id && RoutingService::can_access_dashboard_section(user, &item.id)
                })
                .cloned()
        })
        .collect()
}

/// Fixed mobile shortcuts. All remaining authorized sections still live in
/// the existing keyboard-accessible drawer; desktop/tablet are unchanged.
#[component]
pub fn MobileBottomNavigation(
    user: User,
    navigation_items: Vec<NavigationItem>,
    active_section: String,
) -> Element {
    let locale = use_locale();
    let nav = use_navigator();
    let items = primary_mobile_items(&user, &navigation_items);

    rsx! {
        nav {
            class: "et-mobile-bottom-nav",
            "aria-label": locale.t("navigation.mobile_primary"),
            div { class: "et-mobile-bottom-nav__surface",
                for item in items {
                    {
                        let selected = item.id == active_section;
                        let destination = item.id.clone();
                        let nav = nav.clone();
                        rsx! {
                            button {
                                key: "{item.id}",
                                r#type: "button",
                                class: if selected {
                                    "et-mobile-bottom-nav__item et-mobile-bottom-nav__item--active"
                                } else {
                                    "et-mobile-bottom-nav__item"
                                },
                                "aria-label": item.label.clone(),
                                "aria-current": if selected { Some("page") } else { None },
                                onclick: move |_| {
                                    if destination == "overview" {
                                        nav.push(crate::Route::DashboardRoute {});
                                    } else {
                                        nav.push(crate::Route::DashboardSectionRoute {
                                            section: destination.clone(),
                                        });
                                    }
                                    focus_main_content_after_navigation();
                                },
                                span { class: "et-mobile-bottom-nav__symbol",
                                    span {
                                        class: "material-icons-outlined",
                                        "aria-hidden": "true",
                                        "{item.icon}"
                                    }
                                }
                                span { class: "et-mobile-bottom-nav__label", "{item.label}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(role: SystemRole) -> User {
        User::new(
            "mobile-nav-test".to_string(),
            "mobile@example.test".to_string(),
            role,
            None,
        )
    }

    fn items(ids: &[&str]) -> Vec<NavigationItem> {
        ids.iter()
            .map(|id| NavigationItem::section(*id, *id, "home"))
            .collect()
    }

    #[test]
    fn student_primary_destinations_are_authorized_and_ordered() {
        let all = items(&[
            "users",
            "grades",
            "overview",
            "schedule",
            "assignments",
            "classes",
        ]);
        let selected = primary_mobile_items(&user(SystemRole::Student), &all);
        let ids: Vec<_> = selected.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(ids, ["overview", "classes", "assignments", "grades"]);
        assert!(selected
            .iter()
            .all(|item| item.route.starts_with("/dashboard")));
    }

    #[test]
    fn teacher_keeps_secondary_sections_in_the_canonical_drawer() {
        let all = items(&[
            "students",
            "assignments",
            "knowledge-assets",
            "classes",
            "overview",
            "submissions",
        ]);
        let selected = primary_mobile_items(&user(SystemRole::Teacher), &all);
        let ids: Vec<_> = selected.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(ids, ["overview", "classes", "assignments", "submissions"]);
    }

    #[test]
    fn parent_does_not_advertise_unavailable_reports() {
        let all = items(&["overview", "children", "ai-chat", "users"]);
        let selected = primary_mobile_items(&user(SystemRole::Parent), &all);
        let ids: Vec<_> = selected.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(ids, ["overview", "children", "ai-chat"]);
    }

    #[test]
    fn manager_and_platform_admin_use_their_own_role_routes() {
        let all = items(&[
            "overview",
            "users",
            "classes",
            "knowledge-submissions",
            "knowledge-assets",
            "knowledge-audit",
            "personalization",
        ]);
        let manager = primary_mobile_items(&user(SystemRole::SchoolManager), &all);
        assert_eq!(
            manager
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            ["overview", "users", "classes", "knowledge-submissions"]
        );
        let admin = primary_mobile_items(&user(SystemRole::PlatformAdmin), &all);
        assert_eq!(
            admin
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            ["knowledge-assets", "knowledge-audit", "personalization"]
        );
    }
}
