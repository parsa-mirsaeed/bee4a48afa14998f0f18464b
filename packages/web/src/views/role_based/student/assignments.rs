//! Student assignments with truthful optional values and retry-safe submission UI.

use crate::i18n::{
    assignment_status_label, format_product_date, format_product_date_text, student_translation,
    use_locale, Locale,
};
use crate::views::role_based::components::DashboardSection;
use crate::views::role_based::shared::common::Modal;
use api::server_functions::assignment_functions::{
    get_personalized_assignment, PersonalizedAssignmentResponse,
};
use api::server_functions::dashboard_functions::{
    get_student_assignments, StudentAssignmentInfo, StudentAssignmentPresentationState,
};
use api::server_functions::submission_functions::{
    get_submission_for_assignment, submit_student_assignment, StudentSubmission,
};
use dioxus::prelude::*;

#[component]
pub fn AssignmentsSection() -> Element {
    let locale = use_locale();
    rsx! {
        DashboardSection {
            title: locale.t("assignments.title"),
            description: Some(locale.t("assignments.description")),
            children: rsx! { StudentAssignments {} }
        }
    }
}

#[derive(Clone, PartialEq)]
enum AssignmentModal {
    None,
    Details(String, StudentAssignmentPresentationState),
    Work(String),
}

#[component]
pub fn StudentAssignments() -> Element {
    let locale = use_locale();
    let mut filter = use_signal(|| "all".to_string());
    let mut modal = use_signal(|| AssignmentModal::None);
    let mut resource = use_resource(move || async move { get_student_assignments().await });

    rsx! {
        div { class: "space-y-6",
            div { class: "flex flex-wrap gap-2",
                AssignmentFilter { value: "all", label: locale.t("student.assignments.filter_all"), filter }
                AssignmentFilter { value: "pending", label: locale.t("student.assignments.filter_pending"), filter }
                AssignmentFilter { value: "overdue", label: locale.t("student.assignments.filter_overdue"), filter }
                AssignmentFilter { value: "submitted", label: locale.t("student.assignments.filter_submitted"), filter }
                AssignmentFilter { value: "graded", label: locale.t("student.assignments.filter_graded"), filter }
            }

            match resource.read().as_ref() {
                None => rsx! { AssignmentSkeletonList {} },
                Some(Err(_)) => rsx! {
                    div { class: "et-state-panel et-state-panel--error",
                        p { "{locale.t(\"student.assignments.load_error\")}" }
                        button {
                            class: "et-inline-action mt-3",
                            onclick: move |_| resource.restart(),
                            "{locale.t(\"student.assignments.try_again\")}"
                        }
                    }
                },
                Some(Ok(items)) => {
                    let selected_filter = filter();
                    let visible = items
                        .iter()
                        .filter(|item| assignment_matches_filter(item.presentation_state, &selected_filter))
                        .cloned()
                        .collect::<Vec<_>>();

                    if items.is_empty() {
                        rsx! {
                            div { class: "et-state-panel",
                                h3 { class: "font-semibold text-gray-900 dark:text-white", "{locale.t(\"student.assignments.empty_title\")}" }
                                p { class: "mt-1", "{locale.t(\"student.assignments.empty_description\")}" }
                            }
                        }
                    } else if visible.is_empty() {
                        rsx! {
                            div { class: "et-state-panel",
                                p { "{locale.t(\"student.assignments.no_filter_matches\")}" }
                                button {
                                    class: "et-inline-action mt-3",
                                    onclick: move |_| filter.set("all".to_string()),
                                    "{locale.t(\"student.assignments.clear_filter\")}"
                                }
                            }
                        }
                    } else {
                        rsx! {
                            div { class: "space-y-4",
                                for assignment in visible {
                                    StudentAssignmentCard {
                                        assignment,
                                        on_open: move |(id, state)| modal.set(AssignmentModal::Details(id, state)),
                                    }
                                }
                            }
                        }
                    }
                }
            }

            match modal() {
                AssignmentModal::Details(id, presentation_state) => rsx! {
                    AssignmentDetailModal {
                        assignment_id: id,
                        presentation_state,
                        on_close: move |_| modal.set(AssignmentModal::None),
                        on_work: move |id| modal.set(AssignmentModal::Work(id)),
                    }
                },
                AssignmentModal::Work(id) => rsx! {
                    AssignmentWorkModal {
                        assignment_id: id,
                        on_close: move |_| modal.set(AssignmentModal::None),
                        on_saved: move |_| {
                            modal.set(AssignmentModal::None);
                            resource.restart();
                        }
                    }
                },
                AssignmentModal::None => rsx! {},
            }
        }
    }
}

fn assignment_matches_filter(state: StudentAssignmentPresentationState, filter: &str) -> bool {
    match filter {
        "all" => true,
        "pending" => state == StudentAssignmentPresentationState::Pending,
        "overdue" => state == StudentAssignmentPresentationState::Overdue,
        "submitted" => state == StudentAssignmentPresentationState::Submitted,
        "graded" => state == StudentAssignmentPresentationState::Graded,
        _ => false,
    }
}

fn assignment_action_label(state: StudentAssignmentPresentationState, locale: Locale) -> String {
    let key = match state {
        StudentAssignmentPresentationState::Pending => "student.assignments.action_start",
        StudentAssignmentPresentationState::Overdue => "student.assignments.action_late",
        StudentAssignmentPresentationState::Submitted => "student.assignments.action_submission",
        StudentAssignmentPresentationState::Graded => "student.assignments.action_feedback",
    };
    student_translation(key, locale).unwrap_or(key).to_string()
}

#[component]
fn AssignmentFilter(value: &'static str, label: String, filter: Signal<String>) -> Element {
    let active = filter() == value;
    rsx! {
        button {
            class: if active {
                "rounded-lg bg-primary px-4 py-2 text-sm font-medium text-white"
            } else {
                "rounded-lg border border-gray-200 bg-white px-4 py-2 text-sm font-medium text-gray-600 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-300"
            },
            "aria-pressed": if active { "true" } else { "false" },
            onclick: move |_| filter.set(value.to_string()),
            "{label}"
        }
    }
}

#[component]
fn StudentAssignmentCard(
    assignment: StudentAssignmentInfo,
    on_open: EventHandler<(String, StudentAssignmentPresentationState)>,
) -> Element {
    let locale = use_locale();
    let id = assignment.id.clone();
    let presentation_state = assignment.presentation_state;
    let points = assignment
        .points
        .as_deref()
        .map(|value| format!("{value} {}", locale.t("student.assignments.points_label")))
        .unwrap_or_else(|| locale.t("student.assignments.points_unspecified"));
    let due_date = format_product_date_text(&assignment.due_date, locale.current());
    let status = assignment_status_label(
        assignment.presentation_state.display_name(),
        locale.current(),
    );
    let action = assignment_action_label(assignment.presentation_state, locale.current());

    rsx! {
        article { class: "et-ui-card p-5",
            div { class: "flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between",
                div {
                    h3 { class: "font-semibold text-gray-900 dark:text-white", "{assignment.title}" }
                    p { class: "mt-1 text-sm text-gray-500 dark:text-gray-400", "{assignment.class_name}" }
                }
                span { class: "rounded-full bg-gray-100 px-2 py-1 text-xs text-gray-700 dark:bg-gray-800 dark:text-gray-300", "{status}" }
            }
            div { class: "mt-4 flex flex-wrap gap-4 text-sm text-gray-500 dark:text-gray-400",
                span { "{locale.t(\"student.assignments.due_label\")}: {due_date}" }
                span { "{points}" }
                if let Some(grade) = assignment.grade.as_ref() {
                    span { class: "font-medium text-green-700 dark:text-green-300", "{locale.t(\"student.assignments.grade_label\")}: {grade}" }
                }
            }
            button {
                class: "mt-4 min-h-[44px] rounded-lg bg-primary px-4 py-2 text-sm font-semibold text-white",
                onclick: move |_| on_open.call((id.clone(), presentation_state)),
                "{action}"
            }
        }
    }
}

#[component]
fn AssignmentSkeletonList() -> Element {
    rsx! {
        div { class: "space-y-4",
            for _ in 0..4 {
                div { class: "et-ui-card animate-pulse p-5",
                    div { class: "h-5 w-2/3 rounded bg-gray-200 dark:bg-gray-700" }
                    div { class: "mt-3 h-4 w-1/2 rounded bg-gray-200 dark:bg-gray-700" }
                }
            }
        }
    }
}

#[component]
fn AssignmentDetailModal(
    assignment_id: String,
    presentation_state: StudentAssignmentPresentationState,
    on_close: EventHandler,
    on_work: EventHandler<String>,
) -> Element {
    let locale = use_locale();
    let id_for_fetch = assignment_id.clone();
    let id_for_work = assignment_id.clone();
    let details = use_resource(move || {
        let id = id_for_fetch.clone();
        async move { get_personalized_assignment(id).await }
    });

    rsx! {
        Modal {
            title: locale.t("student.assignments.details_title"),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                match details.read().as_ref() {
                    None => rsx! { p { class: "py-8 text-center text-gray-500", "{locale.t(\"student.assignments.details_loading\")}" } },
                    Some(Err(_)) => rsx! { p { class: "py-8 text-center text-red-600", "{locale.t(\"student.assignments.details_load_error\")}" } },
                    Some(Ok(None)) => rsx! { p { class: "py-8 text-center text-gray-500", "{locale.t(\"student.assignments.details_unavailable\")}" } },
                    Some(Ok(Some(item))) => rsx! {
                        AssignmentDetails {
                            item: item.clone(),
                            presentation_state,
                            on_work: move |_| on_work.call(id_for_work.clone()),
                        }
                    },
                }
            }
        }
    }
}

#[component]
fn AssignmentDetails(
    item: PersonalizedAssignmentResponse,
    presentation_state: StudentAssignmentPresentationState,
    on_work: EventHandler,
) -> Element {
    let locale = use_locale();
    let submission_id = item.id.clone();
    let submission = use_resource(move || {
        let id = submission_id.clone();
        async move { get_submission_for_assignment(id).await }
    });
    let latest_is_graded = matches!(
        submission.read().as_ref(),
        Some(Ok(Some(saved))) if saved.grade.is_some()
    );
    let current_state = if latest_is_graded {
        StudentAssignmentPresentationState::Graded
    } else {
        presentation_state
    };
    let status = assignment_status_label(current_state.display_name(), locale.current());
    let due_date = format_product_date(item.due_at, locale.current());

    rsx! {
        div { class: "space-y-5",
            div {
                h3 { class: "text-xl font-bold text-gray-900 dark:text-white", "{item.title}" }
                p { class: "mt-1 text-sm text-gray-500", "{locale.t(\"student.assignments.status_label\")}: {status}" }
            }
            div { class: "max-h-72 overflow-y-auto whitespace-pre-wrap rounded-lg bg-gray-50 p-4 text-sm dark:bg-gray-800", "{item.body}" }
            p { class: "text-sm text-gray-500", "{locale.t(\"student.assignments.due_label\")}: {due_date}" }
            if current_state == StudentAssignmentPresentationState::Graded {
                match submission.read().as_ref() {
                    Some(Ok(Some(saved))) => rsx! {
                        div { class: "rounded-lg bg-green-50 p-4 text-sm text-green-900 dark:bg-green-900/20 dark:text-green-100",
                            if let Some(grade) = saved.grade.as_ref() {
                                p { class: "font-semibold", "{locale.t(\"student.assignments.grade_label\")}: {grade}" }
                            }
                            if let Some(feedback) = saved.feedback.as_ref() {
                                p { class: "mt-2 whitespace-pre-wrap", "{feedback}" }
                            } else {
                                p { class: "mt-2", "{locale.t(\"student.assignments.no_written_feedback\")}" }
                            }
                        }
                    },
                    Some(Ok(None)) | Some(Err(_)) => rsx! {
                        p { class: "text-sm text-gray-500", "{locale.t(\"student.assignments.feedback_unavailable\")}" }
                    },
                    None => rsx! { p { class: "text-sm text-gray-500", "{locale.t(\"student.assignments.feedback_loading\")}" } },
                }
            } else {
                button {
                    class: "rounded-lg bg-primary px-4 py-2 font-semibold text-white",
                    onclick: move |_| on_work.call(()),
                    "{locale.t(\"student.assignments.open_submission\")}"
                }
            }
        }
    }
}

#[component]
fn AssignmentWorkModal(
    assignment_id: String,
    on_close: EventHandler,
    on_saved: EventHandler,
) -> Element {
    let locale = use_locale();
    let id_for_existing = assignment_id.clone();
    let id_for_submit = assignment_id.clone();
    let mut content = use_signal(String::new);
    let mut initialized = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let existing = use_resource(move || {
        let id = id_for_existing.clone();
        async move { get_submission_for_assignment(id).await }
    });

    if !initialized() {
        if let Some(Ok(Some(StudentSubmission {
            content: existing_content,
            ..
        }))) = existing.read().as_ref()
        {
            content.set(existing_content.clone());
            initialized.set(true);
        } else if matches!(existing.read().as_ref(), Some(Ok(None))) {
            initialized.set(true);
        }
    }

    let empty_work_error = locale.t("student.assignments.enter_work");
    let save_failed_error = locale.t("student.assignments.save_failed");
    let submit = move |_| {
        if busy() {
            return;
        }
        let text = content().trim().to_string();
        if text.is_empty() {
            error.set(Some(empty_work_error.clone()));
            return;
        }
        busy.set(true);
        error.set(None);
        let id = id_for_submit.clone();
        let save_failed_error = save_failed_error.clone();
        spawn(async move {
            match submit_student_assignment(id, text).await {
                Ok(_) => on_saved.call(()),
                Err(_) => {
                    error.set(Some(save_failed_error));
                    busy.set(false);
                }
            }
        });
    };

    rsx! {
        Modal {
            title: locale.t("student.assignments.work_title"),
            open: true,
            on_close: move |_| if !busy() { on_close.call(()) },
            children: rsx! {
                div { class: "space-y-5",
                    match existing.read().as_ref() {
                        None => rsx! { p { class: "text-sm text-gray-500", "{locale.t(\"student.assignments.saved_work_loading\")}" } },
                        Some(Err(_)) => rsx! { p { class: "text-sm text-amber-700", "{locale.t(\"student.assignments.saved_work_load_error\")}" } },
                        _ => rsx! {},
                    }
                    if let Some(message) = error() {
                        div { class: "rounded-lg bg-red-50 p-3 text-sm text-red-800 dark:bg-red-900/20 dark:text-red-200", role: "alert", "{message}" }
                    }
                    div {
                        label {
                            r#for: "student-assignment-work",
                            class: "mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300",
                            "{locale.t(\"student.assignments.work_title\")}"
                        }
                        textarea {
                            id: "student-assignment-work",
                            class: "min-h-64 w-full rounded-lg border border-gray-300 bg-white px-3 py-2.5 dark:border-gray-700 dark:bg-gray-900",
                            value: "{content}",
                            oninput: move |event| content.set(event.value()),
                            disabled: busy(),
                        }
                    }
                    div { class: "flex justify-end gap-3",
                        button {
                            class: "rounded-lg border border-gray-300 px-4 py-2 dark:border-gray-700",
                            disabled: busy(),
                            onclick: move |_| on_close.call(()),
                            "{locale.t(\"common.cancel\")}"
                        }
                        button {
                            class: "rounded-lg bg-primary px-4 py-2 font-semibold text-white disabled:opacity-50",
                            disabled: busy(),
                            onclick: submit,
                            if busy() {
                                "{locale.t(\"student.assignments.submitting\")}"
                            } else {
                                "{locale.t(\"student.assignments.submit_work\")}"
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

    #[test]
    fn missing_points_are_not_invented_and_are_localized() {
        let source = include_str!("assignments.rs");
        assert!(source.contains("student.assignments.points_unspecified"));
        assert!(!source.contains("unwrap_or_else(|| \"100\""));
    }

    #[test]
    fn filters_use_the_canonical_presentation_state() {
        assert!(assignment_matches_filter(
            StudentAssignmentPresentationState::Pending,
            "pending"
        ));
        assert!(assignment_matches_filter(
            StudentAssignmentPresentationState::Overdue,
            "overdue"
        ));
        assert!(!assignment_matches_filter(
            StudentAssignmentPresentationState::Overdue,
            "pending"
        ));
        assert!(assignment_matches_filter(
            StudentAssignmentPresentationState::Submitted,
            "submitted"
        ));
        assert!(assignment_matches_filter(
            StudentAssignmentPresentationState::Graded,
            "graded"
        ));
    }

    #[test]
    fn overdue_work_uses_localized_late_submission_action() {
        assert_eq!(
            assignment_action_label(StudentAssignmentPresentationState::Overdue, Locale::En),
            "Submit late"
        );
        assert_eq!(
            assignment_action_label(StudentAssignmentPresentationState::Overdue, Locale::Fa),
            "ارسال با تأخیر"
        );
    }

    #[test]
    fn graded_detail_uses_submission_feedback_instead_of_persistence_status() {
        let source = include_str!("assignments.rs");
        let implementation = source
            .split("#[cfg(test)]")
            .next()
            .expect("assignment implementation before tests");
        assert!(implementation.contains("get_submission_for_assignment"));
        assert!(!implementation.contains("Status: {item.status}"));
        assert!(!implementation.contains("{e}"));
    }

    #[test]
    fn assignment_filters_and_submission_editor_expose_accessibility_semantics() {
        let source = include_str!("assignments.rs");
        let implementation = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert!(implementation.contains("\"aria-pressed\""));
        assert!(implementation.contains("r#for: \"student-assignment-work\""));
        assert!(implementation.contains("id: \"student-assignment-work\""));
        assert!(!implementation.contains("\"aria-label\": \"{locale.t(\\\"student.assignments.work_title\\\")}\""));
    }
}
