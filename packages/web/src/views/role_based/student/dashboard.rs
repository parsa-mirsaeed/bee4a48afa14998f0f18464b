use crate::application::AuthHooks;
use crate::i18n::{assignment_status_label, format_product_date_text, use_locale};
use crate::views::role_based::components::ResponsiveDashboardLayout;
use crate::views::role_based::student::study_sky::{action_queue, SkyData, StudySkyHero};
use api::server_functions::dashboard_functions::get_student_assignments;
use api::server_functions::dashboard_functions::get_student_classes_view;
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
    // The Study Sky hero and the upcoming list below both read the same
    // get_student_assignments result, so their counts cannot drift apart.
    let sky_data = match &*assignments.read() {
        None => SkyData::Loading,
        Some(Err(_)) => SkyData::Failed,
        Some(Ok(items)) => SkyData::Ready(items.clone()),
    };

    rsx! {
        div { class: "et-page-stack",
            StudySkyHero { data: sky_data, on_navigate }

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
                        // Overdue first, then pending: the same queue that drives
                        // "Up next" and the to-do count in the hero.
                        let queue = action_queue(items);
                        let upcoming = queue.into_iter().take(5).collect::<Vec<_>>();
                        rsx! {
                            if upcoming.is_empty() {
                                StudentState {
                                    message: locale.t("student.dashboard.no_upcoming_assignments"),
                                    error: false,
                                }
                            } else {
                                div { class: "et-panel ss-queue",
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
    fn overview_uses_the_study_sky_hero_and_one_shared_action_queue() {
        let source = include_str!("dashboard.rs");
        let implementation = source
            .split("#[cfg(test)]")
            .next()
            .expect("dashboard implementation before tests");
        assert!(implementation.contains("StudySkyHero"));
        assert!(implementation.contains("action_queue"));
        assert!(!implementation.contains("StudentQuestTracker"));
        assert!(!implementation.contains("et-student-progress"));
        assert!(!implementation.contains("et-overview-intro"));
        assert!(!implementation.contains("assignment.status"));
    }

    #[test]
    fn overview_localizes_status_and_legacy_date_at_the_presentation_boundary() {
        let source = include_str!("dashboard.rs");
        let implementation = source
            .split("#[cfg(test)]")
            .next()
            .expect("dashboard implementation before tests");
        assert!(implementation.contains("assignment_status_label"));
        assert!(implementation.contains("format_product_date_text"));
        assert!(!implementation.contains("let is_fa"));
        assert!(!implementation.contains("\"{assignment.status}\""));
        assert!(!implementation.contains("\"{assignment.due_date}\""));
    }
}
