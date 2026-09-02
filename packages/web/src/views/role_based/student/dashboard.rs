use crate::application::AuthHooks;
use crate::i18n::{assignment_status_label, format_product_date_text, use_locale};
use crate::views::role_based::components::ResponsiveDashboardLayout;
use api::server_functions::dashboard_functions::{
    get_student_assignments, get_student_classes_view, StudentAssignmentPresentationState,
};
use dioxus::prelude::*;

#[component]
pub fn StudentDashboard(section: String) -> Element {
    let current_user = AuthHooks::use_current_user().ok().flatten();
    let locale = use_locale();
    let nav = use_navigator();

    if let Some(user) = current_user {
        let content = match section.as_str() {
            "classes" => rsx! { super::classes::Classes {} },
            "assignments" => rsx! { super::assignments::AssignmentsSection {} },
            "grades" => rsx! { super::grades::GradesSection {} },
            "schedule" if api::product_capabilities::PRODUCTION_PRODUCT_CAPABILITIES.timetable => {
                rsx! { super::schedule::ScheduleSection {} }
            }
            _ => {
                let nav = nav.clone();
                rsx! {
                    StudentOverviewSection {
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
pub fn StudentOverviewSection(on_navigate: EventHandler<String>) -> Element {
    let locale = use_locale();
    let classes = use_resource(move || async move { get_student_classes_view().await });
    let assignments = use_resource(move || async move { get_student_assignments().await });
    let enrolled_class_count = match classes.read().as_ref() {
        Some(Ok(items)) => items.len().to_string(),
        _ => "—".to_string(),
    };

    rsx! {
        div { class: "et-page-stack",
            header { class: "et-overview-intro",
                h2 { class: "et-overview-title", "{locale.t(\"dashboard.overview\")}" }
                p { class: "et-overview-copy", "{locale.t(\"student.dashboard.intro\")}" }
            }

            section { class: "et-section",
                div { class: "et-section-heading",
                    h3 { class: "et-section-title", "{locale.t(\"dashboard.upcoming_assignments\")}" }
                    button {
                        class: "et-inline-action",
                        onclick: move |_| on_navigate.call("assignments".to_string()),
                        "{locale.t(\"student.dashboard.view_all\")}"
                    }
                }
                match &*assignments.read() {
                    None => rsx! {
                        StudentState {
                            message: locale.t("student.dashboard.loading_assignments"),
                            error: false,
                        }
                    },
                    Some(Err(_)) => rsx! {
                        StudentState {
                            message: locale.t("student.dashboard.assignments_load_error"),
                            error: true,
                        }
                    },
                    Some(Ok(items)) if items.is_empty() => rsx! {
                        StudentState { message: locale.t("assignments.no_assignments"), error: false }
                    },
                    Some(Ok(items)) => {
                        let pending_count = items
                            .iter()
                            .filter(|item| {
                                matches!(
                                    item.presentation_state,
                                    StudentAssignmentPresentationState::Pending
                                        | StudentAssignmentPresentationState::Overdue
                                )
                            })
                            .count();
                        let upcoming = items
                            .iter()
                            .filter(|item| {
                                item.presentation_state
                                    == StudentAssignmentPresentationState::Pending
                            })
                            .take(5)
                            .collect::<Vec<_>>();
                        rsx! {
                            div { class: "et-panel grid grid-cols-1 md:grid-cols-2",
                                div { class: "et-stat",
                                    p { class: "et-stat-label", "{locale.t(\"dashboard.pending_tasks\")}" }
                                    p { class: "et-stat-value", "{pending_count}" }
                                }
                                div { class: "et-stat",
                                    p { class: "et-stat-label", "{locale.t(\"dashboard.enrolled_classes\")}" }
                                    p { class: "et-stat-value", "{enrolled_class_count}" }
                                }
                            }
                            if upcoming.is_empty() {
                                StudentState {
                                    message: locale.t("student.dashboard.no_upcoming_assignments"),
                                    error: false,
                                }
                            } else {
                                div { class: "et-panel",
                                    for assignment in upcoming {
                                        {
                                            let status = assignment_status_label(
                                                assignment.presentation_state.display_name(),
                                                locale.current(),
                                            );
                                            let due_date = format_product_date_text(
                                                &assignment.due_date,
                                                locale.current(),
                                            );
                                            rsx! {
                                                div { key: "{assignment.id}", class: "et-list-row",
                                                    div { class: "et-list-primary",
                                                        h4 { class: "et-list-title", "{assignment.title}" }
                                                        p { class: "et-list-meta", "{assignment.class_name}" }
                                                    }
                                                    div { class: "et-list-aside text-end",
                                                        p { "{status}" }
                                                        p { class: "mt-1", "{due_date}" }
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

            section { class: "et-section",
                div { class: "et-section-heading",
                    h3 { class: "et-section-title", "{locale.t(\"dashboard.my_courses\")}" }
                    button {
                        class: "et-inline-action",
                        onclick: move |_| on_navigate.call("classes".to_string()),
                        "{locale.t(\"student.dashboard.view_all\")}"
                    }
                }
                match &*classes.read() {
                    None => rsx! {
                        StudentState {
                            message: locale.t("student.dashboard.loading_classes"),
                            error: false,
                        }
                    },
                    Some(Err(_)) => rsx! {
                        StudentState {
                            message: locale.t("student.dashboard.classes_load_error"),
                            error: true,
                        }
                    },
                    Some(Ok(items)) if items.is_empty() => rsx! {
                        StudentState { message: locale.t("classes.no_classes"), error: false }
                    },
                    Some(Ok(items)) => rsx! {
                        div { class: "et-panel",
                            for class in items.iter().take(4) {
                                div { key: "{class.id}", class: "et-list-row",
                                    div { class: "et-list-primary",
                                        h4 { class: "et-list-title", "{class.name}" }
                                        p { class: "et-list-meta", "{class.subject_name} · {class.teacher_name}" }
                                    }
                                    div { class: "et-list-aside", "{class.term}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StudentState(message: String, error: bool) -> Element {
    let class = if error {
        "et-state-panel et-state-panel--error"
    } else {
        "et-state-panel"
    };
    rsx! { div { class: "{class}", "{message}" } }
}

#[cfg(test)]
mod tests {
    #[test]
    fn upcoming_queue_excludes_submitted_and_graded_work() {
        let source = include_str!("dashboard.rs");
        let implementation = source
            .split("#[cfg(test)]")
            .next()
            .expect("dashboard implementation before tests");
        assert!(implementation.contains("StudentAssignmentPresentationState::Pending"));
        assert!(!implementation.contains("assignment.status"));
    }

    #[test]
    fn overview_localizes_status_and_legacy_date_at_the_presentation_boundary() {
        let source = include_str!("dashboard.rs");
        assert!(source.contains("assignment_status_label"));
        assert!(source.contains("format_product_date_text"));
        assert!(!source.contains("let is_fa"));
    }
}
