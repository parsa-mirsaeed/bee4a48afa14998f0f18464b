use crate::components::skeleton::SkeletonCard;
use crate::i18n::{assignment_status_label, format_product_date_text, use_locale};
use crate::views::role_based::components::DashboardSection;
use crate::views::role_based::shared::common::Modal;
use crate::views::role_based::shared::common::{format_grade_date, GradeToken};
use api::server_functions::parent_scoped_functions::{
    get_child_assignments_for_parent_scoped, get_child_grades_for_parent_scoped,
    get_parent_children_scoped, ParentChildSummary,
};
use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
enum ChildModal {
    Grades(ParentChildSummary),
    Assignments(ParentChildSummary),
}

#[component]
pub fn ChildrenSection() -> Element {
    let locale = use_locale();
    rsx! {
        DashboardSection {
            title: locale.t("parent.children.title"),
            description: Some(locale.t("parent.children.desc")),
            children: rsx! { ChildrenDetail {} }
        }
    }
}

#[component]
pub fn ChildrenDetail() -> Element {
    let locale = use_locale();
    let mut active_modal = use_signal(|| None::<ChildModal>);
    let mut children = use_resource(move || async move { get_parent_children_scoped().await });

    rsx! {
        div { class: "space-y-6",
            match children.read().as_ref() {
                None => rsx! { div { class: "grid grid-cols-1 xl:grid-cols-3 gap-4", SkeletonCard {} SkeletonCard {} } },
                Some(Err(_)) => rsx! {
                    div { class: "et-state-panel et-state-panel--error",
                        p { "{locale.t(\"parent.children.load_error\")}" }
                        button { class: "et-inline-action mt-3", onclick: move |_| children.restart(), "{locale.t(\"common.retry\")}" }
                    }
                },
                Some(Ok(items)) if items.is_empty() => rsx! {
                    div { class: "et-ui-card p-10 text-center text-gray-500 dark:text-gray-400",
                        h3 { class: "font-semibold text-gray-900 dark:text-white", "{locale.t(\"parent.dashboard.empty_title\")}" }
                        p { class: "mt-2 text-sm", "{locale.t(\"parent.dashboard.empty_description\")}" }
                    }
                },
                Some(Ok(items)) => rsx! {
                    div { class: "grid grid-cols-1 xl:grid-cols-3 gap-4",
                        for child in items {
                            {
                                let for_grades = child.clone();
                                let for_assignments = child.clone();
                                rsx! {
                                    div { key: "{child.id}", class: "et-ui-card p-5",
                                        h3 { class: "text-lg font-bold text-gray-900 dark:text-white", "{child.name}" }
                                        p { class: "mt-1 text-sm text-gray-500 dark:text-gray-400",
                                            if let Some(grade) = child.grade_level.as_ref() {
                                                "{grade}"
                                            } else {
                                                "{locale.t(\"parent.child.grade_not_recorded\")}"
                                            }
                                        }
                                        div { class: "mt-4 rounded-lg bg-gray-50 dark:bg-gray-800/50 p-3",
                                            p { class: "text-xs text-gray-500", "{locale.t(\"parent.dashboard.child_card.classes\")}" }
                                            p { class: "text-xl font-bold text-gray-900 dark:text-white", "{child.enrolled_classes}" }
                                        }
                                        div { class: "mt-4 grid grid-cols-2 gap-2",
                                            button {
                                                class: "min-h-[44px] rounded-lg bg-blue-50 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300 text-sm font-medium",
                                                onclick: move |_| active_modal.set(Some(ChildModal::Grades(for_grades.clone()))),
                                                "{locale.t(\"parent.children.actions.view_grades\")}"
                                            }
                                            button {
                                                class: "min-h-[44px] rounded-lg bg-purple-50 dark:bg-purple-900/20 text-purple-700 dark:text-purple-300 text-sm font-medium",
                                                onclick: move |_| active_modal.set(Some(ChildModal::Assignments(for_assignments.clone()))),
                                                "{locale.t(\"parent.children.actions.assignments\")}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(modal) = active_modal() {
                match modal {
                    ChildModal::Grades(child) => rsx! {
                        ChildGradesModal { child, on_close: move |_| active_modal.set(None) }
                    },
                    ChildModal::Assignments(child) => rsx! {
                        ChildAssignmentsModal { child, on_close: move |_| active_modal.set(None) }
                    },
                }
            }
        }
    }
}

#[component]
fn ChildGradesModal(child: ParentChildSummary, on_close: EventHandler) -> Element {
    let locale = use_locale();
    let child_id = child.id.clone();
    let grades = use_resource(move || {
        let id = child_id.clone();
        async move { get_child_grades_for_parent_scoped(id).await }
    });
    rsx! {
        Modal {
            title: format!("{} - {}", child.name, locale.t("parent.children.actions.view_grades")),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div { class: "space-y-3 max-h-96 overflow-y-auto",
                    match grades.read().as_ref() {
                        None => rsx! { p { class: "py-8 text-center text-gray-500", "{locale.t(\"parent.children.grades.loading\")}" } },
                        Some(Err(_)) => rsx! { p { class: "py-8 text-center text-red-600", "{locale.t(\"parent.children.grades.load_error\")}" } },
                        Some(Ok(items)) if items.is_empty() => rsx! { p { class: "py-8 text-center text-gray-500", "{locale.t(\"parent.children.grades.empty\")}" } },
                        Some(Ok(items)) => rsx! {
                            for grade in items {
                                div { class: "p-4 border border-gray-200 dark:border-gray-700 rounded-lg flex justify-between gap-3",
                                    div {
                                        h4 { class: "font-semibold text-gray-900 dark:text-white", "{grade.assignment_title}" }
                                        p { class: "text-sm text-gray-500", "{grade.class_name}" }
                                        if let Some(graded_at) = grade.graded_at.as_ref() {
                                            if let Some(grade_date) = format_grade_date(graded_at, locale.current()) {
                                                p { class: "text-sm text-gray-500", "{grade_date}" }
                                            }
                                        }
                                    }
                                    div { class: "text-right",
                                        GradeToken { value: grade.grade.clone(), class: Some("font-bold text-primary".to_string()) }
                                        GradeToken { value: grade.points.clone(), class: Some("text-xs text-gray-500".to_string()) }
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
fn ChildAssignmentsModal(child: ParentChildSummary, on_close: EventHandler) -> Element {
    let locale = use_locale();
    let child_id = child.id.clone();
    let assignments = use_resource(move || {
        let id = child_id.clone();
        async move { get_child_assignments_for_parent_scoped(id).await }
    });
    rsx! {
        Modal {
            title: format!("{} - {}", child.name, locale.t("parent.children.actions.assignments")),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div { class: "space-y-3 max-h-96 overflow-y-auto",
                    match assignments.read().as_ref() {
                        None => rsx! { p { class: "py-8 text-center text-gray-500", "{locale.t(\"parent.children.assignments.loading\")}" } },
                        Some(Err(_)) => rsx! { p { class: "py-8 text-center text-red-600", "{locale.t(\"parent.children.assignments.load_error\")}" } },
                        Some(Ok(items)) if items.is_empty() => rsx! { p { class: "py-8 text-center text-gray-500", "{locale.t(\"parent.children.assignments.empty\")}" } },
                        Some(Ok(items)) => rsx! {
                            for assignment in items {
                                {
                                    let status = assignment_status_label(
                                        &assignment.status,
                                        locale.current(),
                                    );
                                    let due_date = format_product_date_text(
                                        &assignment.due_date,
                                        locale.current(),
                                    );
                                    rsx! {
                                        div { key: "{assignment.id}", class: "p-4 border border-gray-200 dark:border-gray-700 rounded-lg",
                                            div { class: "flex justify-between gap-3",
                                                div {
                                                    h4 { class: "font-semibold text-gray-900 dark:text-white", "{assignment.title}" }
                                                    p { class: "text-sm text-gray-500", "{assignment.class_name}" }
                                                }
                                                span { class: "text-xs text-gray-500", "{status}" }
                                            }
                                            p { class: "mt-2 text-xs text-gray-400", "{locale.t(\"parent.children.assignments.due_label\")}: {due_date}" }
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
    fn parent_children_use_shared_localized_status_date_and_grade_primitives() {
        let source = include_str!("children.rs");
        let implementation = source
            .split("#[cfg(test)]")
            .next()
            .expect("parent children implementation before tests");
        assert!(implementation.contains("assignment_status_label"));
        assert!(implementation.contains("format_product_date_text"));
        assert!(implementation.contains("GradeToken"));
        assert!(!implementation.contains("not enabled in this release"));
        assert!(!implementation.contains("\"{assignment.status}\""));
        assert!(!implementation.contains("\"{assignment.due_date}\""));
        assert!(!implementation.contains("Grade not recorded"));
    }
}
