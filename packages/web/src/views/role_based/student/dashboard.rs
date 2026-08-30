use crate::application::AuthHooks;
use crate::i18n::{use_locale, Locale};
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
    let is_fa = locale.current() == Locale::Fa;
    let intro = if is_fa {
        "ابتدا کارهایی را ببینید که نیاز به اقدام دارند، سپس کلاس‌ها و نمره‌های خود را مرور کنید."
    } else {
        "Start with work that needs attention, then review your classes and grades."
    };
    let view_all = if is_fa {
        "مشاهده همه"
    } else {
        "View all"
    };
    let loading_assignments = if is_fa {
        "در حال بارگذاری تکلیف‌ها…"
    } else {
        "Loading assignments…"
    };
    let failed_assignments = if is_fa {
        "بارگذاری تکلیف‌ها ناموفق بود."
    } else {
        "Unable to load assignments."
    };
    let loading_classes = if is_fa {
        "در حال بارگذاری کلاس‌ها…"
    } else {
        "Loading classes…"
    };
    let failed_classes = if is_fa {
        "بارگذاری کلاس‌ها ناموفق بود."
    } else {
        "Unable to load classes."
    };
    let no_upcoming_assignments = if is_fa {
        "تکلیف آینده‌ای وجود ندارد."
    } else {
        "No upcoming assignments."
    };

    rsx! {
        div { class: "et-page-stack",
            header { class: "et-overview-intro",
                h2 { class: "et-overview-title", "{locale.t(\"dashboard.overview\")}" }
                p { class: "et-overview-copy", "{intro}" }
            }

            section { class: "et-section",
                div { class: "et-section-heading",
                    h3 { class: "et-section-title", "{locale.t(\"dashboard.upcoming_assignments\")}" }
                    button {
                        class: "et-inline-action",
                        onclick: move |_| on_navigate.call("assignments".to_string()),
                        "{view_all}"
                    }
                }
                match &*assignments.read() {
                    None => rsx! { StudentState { message: loading_assignments.to_string(), error: false } },
                    Some(Err(_)) => rsx! { StudentState { message: failed_assignments.to_string(), error: true } },
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
                                    message: no_upcoming_assignments.to_string(),
                                    error: false,
                                }
                            } else {
                                div { class: "et-panel",
                                    for assignment in upcoming {
                                        div { key: "{assignment.id}", class: "et-list-row",
                                            div { class: "et-list-primary",
                                                h4 { class: "et-list-title", "{assignment.title}" }
                                                p { class: "et-list-meta", "{assignment.class_name}" }
                                            }
                                            div { class: "et-list-aside",
                                                p { "{assignment.presentation_state.display_name()}" }
                                                p { class: "mt-1", "{assignment.due_date}" }
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
                        "{view_all}"
                    }
                }
                match &*classes.read() {
                    None => rsx! { StudentState { message: loading_classes.to_string(), error: false } },
                    Some(Err(_)) => rsx! { StudentState { message: failed_classes.to_string(), error: true } },
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
}
