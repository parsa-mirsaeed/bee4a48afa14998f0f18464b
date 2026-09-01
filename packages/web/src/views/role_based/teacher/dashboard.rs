use crate::application::AuthHooks;
use crate::i18n::{
    assignment_status_label as localized_status_label, format_product_date_text, use_locale, Locale,
};
use crate::ui::{DataState, DataStateKind};
use crate::views::role_based::components::ResponsiveDashboardLayout;
use api::server_functions::dashboard_functions::{
    get_teacher_assignments, get_teacher_dashboard_stats,
};
use dioxus::prelude::*;

use super::TeacherKnowledgeAssetsScoped;

fn assignment_presentation_label(
    assignment: &api::server_functions::dashboard_functions::TeacherAssignmentInfo,
    locale: Locale,
) -> String {
    let lifecycle = localized_status_label(&assignment.lifecycle_status.to_string(), locale);
    match assignment.progress_state {
        Some(progress_state) => format!(
            "{} · {}",
            lifecycle,
            localized_status_label(progress_state.display_name(), locale)
        ),
        None => lifecycle,
    }
}

#[component]
pub fn TeacherDashboard(section: String) -> Element {
    let locale = use_locale();
    let current_user = AuthHooks::use_current_user().ok().flatten();
    let nav = use_navigator();

    if let Some(user) = current_user {
        let content = match section.as_str() {
            "classes" => rsx! { super::classes::Classes {} },
            "assignments" => rsx! { super::assignments::Assignments {} },
            "knowledge-assets" => rsx! { TeacherKnowledgeAssetsScoped {} },
            "students" => rsx! { super::students::Students {} },
            "submissions" => rsx! { super::submissions::Submissions {} },
            _ => {
                let nav = nav.clone();
                rsx! {
                    TeacherOverviewSection {
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
        rsx! {
            DataState {
                kind: DataStateKind::Loading,
                title: locale.t("common.loading"),
                description: locale.t("session.checking"),
            }
        }
    }
}

#[component]
pub fn TeacherOverviewSection(on_navigate: EventHandler<String>) -> Element {
    let locale = use_locale();
    let stats = use_resource(move || async move { get_teacher_dashboard_stats().await });
    let assignments = use_resource(move || async move { get_teacher_assignments().await });

    rsx! {
        div { class: "et-page-stack",
            header { class: "et-overview-intro",
                h2 { class: "et-overview-title", "{locale.t(\"dashboard.overview\")}" }
                p { class: "et-overview-copy", "{locale.t(\"teachers.classes.manage_description\")}" }
            }

            match &*stats.read() {
                None => rsx! { div { class: "et-state-panel", "{locale.t(\"common.loading\")}" } },
                Some(Err(_)) => rsx! { div { class: "et-state-panel et-state-panel--error", "{locale.t(\"errors.generic_description\")}" } },
                Some(Ok(value)) => rsx! {
                    div { class: "et-stat-grid",
                        TeacherStateBlock { label: locale.t("dashboard.pending_grading"), value: value.pending_grading.to_string() }
                        TeacherStateBlock { label: locale.t("classes.my_classes"), value: value.total_classes.to_string() }
                        TeacherStateBlock { label: locale.t("dashboard.total_students"), value: value.total_students.to_string() }
                    }
                }
            }

            section { class: "et-section",
                div { class: "et-section-heading",
                    h3 { class: "et-section-title", "{locale.t(\"dashboard.quick_actions\")}" }
                }
                div { class: "grid grid-cols-1 md:grid-cols-3 gap-4",
                    TeacherAction {
                        icon: "grading".to_string(),
                        title: locale.t("nav.grading"),
                        description: locale.t("teachers.quick_actions.grade_submissions_desc"),
                        on_click: move |_| on_navigate.call("submissions".to_string()),
                    }
                    TeacherAction {
                        icon: "assignment".to_string(),
                        title: locale.t("assignments.title"),
                        description: locale.t("teachers.quick_actions.create_assignment_desc"),
                        on_click: move |_| on_navigate.call("assignments".to_string()),
                    }
                    TeacherAction {
                        icon: "library_books".to_string(),
                        title: locale.t("teacher.knowledge_assets.title"),
                        description: locale.t("teacher.knowledge_assets.description"),
                        on_click: move |_| on_navigate.call("knowledge-assets".to_string()),
                    }
                }
            }

            section { class: "et-section",
                div { class: "et-section-heading",
                    h3 { class: "et-section-title", "{locale.t(\"assignments.title\")}" }
                    button {
                        class: "et-inline-action",
                        r#type: "button",
                        onclick: move |_| on_navigate.call("assignments".to_string()),
                        "{locale.t(\"common.view_all\")}"
                    }
                }
                match &*assignments.read() {
                    None => rsx! { div { class: "et-state-panel", "{locale.t(\"common.loading\")}" } },
                    Some(Err(_)) => rsx! { div { class: "et-state-panel et-state-panel--error", "{locale.t(\"errors.generic_description\")}" } },
                    Some(Ok(items)) if items.is_empty() => rsx! { div { class: "et-state-panel", "{locale.t(\"teachers.dashboard.no_assignments_created\")}" } },
                    Some(Ok(items)) => rsx! {
                        div { class: "et-panel",
                            for assignment in items.iter().take(5) {
                                div { key: "{assignment.id}", class: "et-list-row",
                                    div { class: "et-list-primary",
                                        h4 { class: "et-list-title", "{assignment.title}" }
                                        p {
                                            class: "et-list-meta",
                                            "{format!(\"{} · {}/{} {}\", assignment.class_name, assignment.submitted_count, assignment.total_count, locale.t(\"teachers.students.submitted_label\"))}"
                                        }
                                    }
                                    div { class: "et-list-aside",
                                        p { "{assignment_presentation_label(assignment, locale.current())}" }
                                        p { class: "mt-1", "{format_product_date_text(&assignment.due_date, locale.current())}" }
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

#[component]
fn TeacherStateBlock(label: String, value: String) -> Element {
    rsx! {
        div { class: "et-stat",
            p { class: "et-stat-label", "{label}" }
            p { class: "et-stat-value", "{value}" }
        }
    }
}

#[component]
fn TeacherAction(
    icon: String,
    title: String,
    description: String,
    on_click: EventHandler,
) -> Element {
    rsx! {
        button {
            class: "et-action-card",
            r#type: "button",
            onclick: move |_| on_click.call(()),
            div { class: "et-action-card-top",
                span { class: "et-action-icon",
                    span { class: "material-icons-outlined text-xl", "aria-hidden": "true", "{icon}" }
                }
                span { class: "material-icons-outlined et-action-arrow", "aria-hidden": "true", "arrow_forward" }
            }
            div {
                h3 { class: "et-action-title", "{title}" }
                p { class: "et-action-description", "{description}" }
            }
        }
    }
}
