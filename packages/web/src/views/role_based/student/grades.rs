use crate::components::skeleton::SkeletonCard;
use crate::i18n::use_locale;
use crate::views::role_based::components::DashboardSection;
use crate::views::role_based::shared::common::{format_grade_date, GradeToken, Modal};
use api::server_functions::dashboard_functions::{
    get_class_grades_for_student, get_student_classes_view, StudentClassView,
};
use dioxus::prelude::*;

#[component]
pub fn GradesSection() -> Element {
    let locale = use_locale();
    rsx! {
        DashboardSection {
            title: locale.t("grades.title"),
            description: Some(locale.t("grades.description")),
            children: rsx! { StudentGrades {} }
        }
    }
}

#[component]
pub fn StudentGrades() -> Element {
    let locale = use_locale();
    let mut selected_class = use_signal(|| None::<StudentClassView>);
    let classes = use_resource(move || async move { get_student_classes_view().await });

    rsx! {
        div { class: "space-y-6",
            div { class: "et-ui-card p-5 border-s-4 border-blue-500",
                h3 { class: "font-semibold text-gray-900 dark:text-white", "{locale.t(\"student.grades.recorded_title\")}" }
                p { class: "mt-1 text-sm text-gray-500 dark:text-gray-400", "{locale.t(\"student.grades.recorded_description\")}" }
            }
            match &*classes.read() {
                None => rsx! { div { class: "grid grid-cols-1 md:grid-cols-2 gap-4", SkeletonCard {} SkeletonCard {} } },
                Some(Err(_)) => rsx! {
                    div { class: "et-ui-card p-8 text-center text-red-600", "{locale.t(\"student.grades.load_error\")}" }
                },
                Some(Ok(items)) if items.is_empty() => rsx! {
                    div { class: "et-ui-card p-8 text-center text-gray-500", "{locale.t(\"grades.no_classes\")}" }
                },
                Some(Ok(items)) => rsx! {
                    div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                        for class in items.iter() {
                            {
                                let selected = class.clone();
                                rsx! {
                                    div { key: "{class.id}", class: "et-ui-card p-5",
                                        h3 { class: "font-semibold text-gray-900 dark:text-white", "{class.name}" }
                                        p { class: "mt-1 text-sm text-gray-500 dark:text-gray-400", "{class.subject_name} · {class.teacher_name}" }
                                        p { class: "mt-1 text-xs text-gray-400", "{class.term}" }
                                        button {
                                            class: "mt-4 text-sm font-medium text-primary hover:underline min-h-[40px]",
                                            onclick: move |_| selected_class.set(Some(selected.clone())),
                                            "{locale.t(\"common.view_details\")}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(class) = selected_class() {
                ClassGradesModal {
                    class,
                    on_close: move |_| selected_class.set(None),
                }
            }
        }
    }
}

#[component]
fn ClassGradesModal(class: StudentClassView, on_close: EventHandler) -> Element {
    let locale = use_locale();
    let class_id = class.id.clone();
    let grades = use_resource(move || {
        let id = class_id.clone();
        async move { get_class_grades_for_student(id).await }
    });

    rsx! {
        Modal {
            title: format!("{} - {}", class.name, locale.t("grades.grade_details")),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div { class: "space-y-3 max-h-96 overflow-y-auto",
                    match &*grades.read() {
                        None => rsx! { p { class: "py-8 text-center text-gray-500", "{locale.t(\"grades.loading\")}" } },
                        Some(Err(_)) => rsx! { p { class: "py-8 text-center text-red-600", "{locale.t(\"student.grades.detail_load_error\")}" } },
                        Some(Ok(items)) if items.is_empty() => rsx! { p { class: "py-8 text-center text-gray-500", "{locale.t(\"grades.no_grades\")}" } },
                        Some(Ok(items)) => rsx! {
                            for grade in items.iter() {
                                div { class: "p-4 border border-gray-200 dark:border-gray-700 rounded-lg flex justify-between gap-4",
                                    div {
                                        h4 { class: "font-semibold text-gray-900 dark:text-white", "{grade.assignment_title}" }
                                        if let Some(graded_at) = grade.graded_at.as_ref() {
                                            if let Some(grade_date) = format_grade_date(graded_at, locale.current()) {
                                                p { class: "text-sm text-gray-500 dark:text-gray-400", "{grade_date}" }
                                            }
                                        }
                                    }
                                    div { class: "text-end",
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

#[cfg(test)]
mod tests {
    #[test]
    fn student_grade_errors_are_bounded_and_translation_driven() {
        let source = include_str!("grades.rs");
        assert!(source.contains("student.grades.load_error"));
        assert!(source.contains("student.grades.detail_load_error"));
        assert!(!source.contains("Some(Err(e))"));
    }
}
