use crate::application::AuthHooks;
use crate::i18n::{format_parent_class_count, use_locale};
use crate::views::role_based::components::ResponsiveDashboardLayout;
use api::server_functions::parent_scoped_functions::get_parent_children_scoped;
use dioxus::prelude::*;

#[component]
pub fn ParentDashboard(section: String) -> Element {
    let current_user = AuthHooks::use_current_user().ok().flatten();
    let locale = use_locale();
    let nav = use_navigator();

    if let Some(user) = current_user {
        let content = match section.as_str() {
            "children" => rsx! { super::children::ChildrenSection {} },
            "reports"
                if api::product_capabilities::PRODUCTION_PRODUCT_CAPABILITIES.parent_reports =>
            {
                rsx! { super::reports::ReportsSection {} }
            }
            "communication"
                if api::product_capabilities::PRODUCTION_PRODUCT_CAPABILITIES
                    .parent_teacher_communication =>
            {
                rsx! { super::communication::CommunicationSection {} }
            }
            _ => {
                let nav = nav.clone();
                rsx! {
                    ParentOverviewSection {
                        on_navigate: move |next: String| {
                            if next == "overview" {
                                nav.push(crate::Route::DashboardRoute {});
                            } else {
                                nav.push(crate::Route::DashboardSectionRoute { section: next });
                            }
                        },
                    }
                }
            }
        };
        rsx! {
            ResponsiveDashboardLayout {
                user,
                active_section: section,
                children: rsx! { {content} }
            }
        }
    } else {
        rsx! { div { class: "flex min-h-screen items-center justify-center", "{locale.t(\"common.loading\")}" } }
    }
}

#[component]
pub fn ParentOverviewSection(on_navigate: EventHandler<String>) -> Element {
    let locale = use_locale();
    let mut children = use_resource(move || async move { get_parent_children_scoped().await });

    rsx! {
        div { class: "et-page-stack",
            header { class: "et-overview-intro",
                h2 { class: "et-overview-title", "{locale.t(\"parent.dashboard.sections.overview\")}" }
                p { class: "et-overview-copy", "{locale.t(\"parent.dashboard.intro\")}" }
            }

            match children.read().as_ref() {
                None => rsx! { div { class: "et-state-panel", "{locale.t(\"parent.dashboard.loading\")}" } },
                Some(Err(_)) => rsx! {
                    div { class: "et-state-panel et-state-panel--error",
                        p { "{locale.t(\"parent.dashboard.load_error\")}" }
                        button { class: "et-inline-action mt-3", onclick: move |_| children.restart(), "{locale.t(\"common.retry\")}" }
                    }
                },
                Some(Ok(items)) if items.is_empty() => rsx! {
                    div { class: "et-state-panel",
                        h3 { class: "font-semibold text-gray-900 dark:text-white", "{locale.t(\"parent.dashboard.empty_title\")}" }
                        p { class: "mt-2", "{locale.t(\"parent.dashboard.empty_description\")}" }
                    }
                },
                Some(Ok(items)) => {
                    let total_classes: i64 = items.iter().map(|child| child.enrolled_classes).sum();
                    rsx! {
                        div { class: "et-panel grid grid-cols-1 md:grid-cols-2",
                            div { class: "et-stat",
                                p { class: "et-stat-label", "{locale.t(\"parent.dashboard.stats.children\")}" }
                                p { class: "et-stat-value", "{items.len()}" }
                            }
                            div { class: "et-stat",
                                p { class: "et-stat-label", "{locale.t(\"parent.dashboard.stats.enrolled_classes\")}" }
                                p { class: "et-stat-value", "{total_classes}" }
                            }
                        }
                        section { class: "et-section",
                            div { class: "et-section-heading",
                                h3 { class: "et-section-title", "{locale.t(\"nav.children\")}" }
                                button { class: "et-inline-action", onclick: move |_| on_navigate.call("children".to_string()), "{locale.t(\"parent.dashboard.view_details\")}" }
                            }
                            div { class: "et-panel",
                                for child in items {
                                    {
                                        let class_count = format_parent_class_count(
                                            child.enrolled_classes,
                                            locale.current(),
                                        );
                                        rsx! {
                                            div { key: "{child.id}", class: "et-list-row",
                                                div { class: "et-list-primary",
                                                    h4 { class: "et-list-title", "{child.name}" }
                                                    p { class: "et-list-meta",
                                                        if let Some(grade) = child.grade_level.as_ref() {
                                                            "{grade}"
                                                        } else {
                                                            "{locale.t(\"parent.child.grade_not_recorded\")}"
                                                        }
                                                    }
                                                }
                                                div { class: "et-list-aside", "{class_count}" }
                                            }
                                        }
                                    }
                                }
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
    #[test]
    fn parent_overview_uses_product_localization_without_release_commentary() {
        let source = include_str!("dashboard.rs");
        let implementation = source
            .split("#[cfg(test)]")
            .next()
            .expect("parent dashboard implementation before tests");
        assert!(implementation.contains("parent.dashboard.intro"));
        assert!(implementation.contains("format_parent_class_count"));
        assert!(!implementation.contains("Incomplete capabilities"));
        assert!(!implementation.contains("let is_fa"));
        assert!(!implementation.contains("Grade not recorded"));
    }
}
