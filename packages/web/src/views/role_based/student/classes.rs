use crate::components::skeleton::SkeletonCard;
use crate::i18n::{assignment_status_label, format_product_date_text, use_locale};
use crate::views::role_based::components::DashboardSection;
use crate::views::role_based::shared::common::{format_grade_date, GradeToken, Modal};
use api::server_functions::dashboard_functions::{
    get_class_assignments_for_student, get_class_grades_for_student,
    get_class_materials_for_student, get_student_classes_view, StudentAssignmentPresentationState,
    StudentClassView,
};
use dioxus::prelude::*;

/// Classes for student
#[component]
pub fn Classes() -> Element {
    let locale_ctx = use_locale();
    let t_my_classes = locale_ctx.t("classes.my_classes");
    let t_description = locale_ctx.t("classes.view_description");

    rsx! {
        DashboardSection {
            title: t_my_classes,
            description: Some(t_description),
            children: rsx! {
                StudentClassesList {}
            }
        }
    }
}

#[derive(Clone, PartialEq)]
enum StudentClassModal {
    None,
    Tasks(StudentClassView),
    Grades(StudentClassView),
    Materials(StudentClassView),
}

#[component]
pub fn StudentClassesList() -> Element {
    let mut active_modal = use_signal(|| StudentClassModal::None);
    let locale_ctx = use_locale();
    let classes_resource = use_resource(move || async move { get_student_classes_view().await });

    rsx! {
        match &*classes_resource.read() {
            None => rsx! {
                div {
                    class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 md:gap-6 animate-fade-in",
                    for _ in 0..3 {
                        SkeletonCard {}
                    }
                }
            },
            Some(Err(_)) => rsx! {
                div {
                    class: "et-state-panel et-state-panel--error text-center py-12",
                    p { "{locale_ctx.t(\"student.classes.load_error\")}" }
                }
            },
            Some(Ok(classes)) if classes.is_empty() => rsx! {
                div {
                    class: "et-ui-card p-12 text-center flex flex-col items-center justify-center min-h-[300px]",
                    div {
                        class: "w-20 h-20 bg-gray-100 dark:bg-gray-800 rounded-full flex items-center justify-center mb-4",
                        span { class: "text-4xl", "📚" }
                    }
                    h3 { class: "text-xl font-bold text-gray-900 dark:text-white mb-2", "{locale_ctx.t(\"classes.no_classes\")}" }
                    p { class: "text-gray-500 dark:text-gray-400", "{locale_ctx.t(\"classes.not_enrolled\")}" }
                }
            },
            Some(Ok(classes)) => rsx! {
                div {
                    class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 md:gap-6 animate-fade-in",
                    for class in classes.iter() {
                        StudentClassCard {
                            key: "{class.id}",
                            class: class.clone(),
                            on_materials: move |c: StudentClassView| active_modal.set(StudentClassModal::Materials(c)),
                            on_tasks: move |c: StudentClassView| active_modal.set(StudentClassModal::Tasks(c)),
                            on_grades: move |c: StudentClassView| active_modal.set(StudentClassModal::Grades(c)),
                        }
                    }
                }

                match active_modal() {
                    StudentClassModal::Tasks(class) => rsx! {
                        ClassTasksModal {
                            class: class.clone(),
                            on_close: move |_| active_modal.set(StudentClassModal::None)
                        }
                    },
                    StudentClassModal::Grades(class) => rsx! {
                        ClassGradesModal {
                            class: class.clone(),
                            on_close: move |_| active_modal.set(StudentClassModal::None)
                        }
                    },
                    StudentClassModal::Materials(class) => rsx! {
                        ClassMaterialsModal {
                            class: class.clone(),
                            on_close: move |_| active_modal.set(StudentClassModal::None)
                        }
                    },
                    StudentClassModal::None => rsx! {}
                }
            }
        }
    }
}

#[component]
fn StudentClassCard(
    class: StudentClassView,
    on_materials: EventHandler<StudentClassView>,
    on_tasks: EventHandler<StudentClassView>,
    on_grades: EventHandler<StudentClassView>,
) -> Element {
    let class_for_materials = class.clone();
    let class_for_tasks = class.clone();
    let class_for_grades = class.clone();
    let locale_ctx = use_locale();

    let color_class = match class.name.len() % 4 {
        0 => ("from-blue-600", "to-blue-400"),
        1 => ("from-indigo-600", "to-purple-500"),
        2 => ("from-emerald-600", "to-teal-500"),
        _ => ("from-amber-600", "to-orange-400"),
    };

    rsx! {
        div {
            class: "et-ui-card p-0 flex flex-col h-full overflow-hidden group hover:-translate-y-1 transition-transform duration-300",

            div {
                class: "p-4 md:p-6 bg-gradient-to-br {color_class.0} {color_class.1} text-white relative overflow-hidden",
                div { class: "absolute -right-6 -top-6 w-24 h-24 bg-white/20 rounded-full blur-xl" }
                h3 { class: "text-lg md:text-xl font-bold mb-1 relative z-10", "{class.name}" }
                p {
                    class: "text-xs md:text-sm font-medium opacity-90 relative z-10",
                    {format!("{}{}", locale_ctx.t("classes.with_teacher_prefix"), class.teacher_name)}
                }
            }

            div {
                class: "p-4 md:p-6 flex-1 flex flex-col gap-3 md:gap-4",
                div {
                    class: "flex justify-between items-center text-sm",
                    div {
                        class: "flex items-center gap-2 text-gray-500 dark:text-gray-400 font-medium",
                        span { class: "material-icons-outlined text-base", "book" }
                        "{locale_ctx.t(\"classes.subject\")}"
                    }
                    span { class: "text-gray-900 dark:text-white font-medium", "{class.subject_name}" }
                }
                div {
                    class: "flex justify-between items-center text-sm",
                    div {
                        class: "flex items-center gap-2 text-gray-500 dark:text-gray-400 font-medium",
                        span { class: "material-icons-outlined text-base", "calendar_today" }
                        "{locale_ctx.t(\"classes.term\")}"
                    }
                    span { class: "text-gray-900 dark:text-white font-medium", "{class.term}" }
                }

                div {
                    class: "grid grid-cols-3 gap-1 md:gap-2 mt-auto pt-3 md:pt-4 border-t border-gray-100 dark:border-gray-800",
                    button {
                        class: "flex flex-col items-center justify-center gap-0.5 md:gap-1 p-2 md:p-3 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors text-gray-500 dark:text-gray-400 hover:text-primary dark:hover:text-primary-light min-h-[48px]",
                        onclick: move |_| on_materials.call(class_for_materials.clone()),
                        span { class: "material-icons-outlined text-lg md:text-xl", "folder_open" }
                        span { class: "text-[8px] md:text-[10px] font-medium uppercase", "{locale_ctx.t(\"nav.materials\")}" }
                    }
                    button {
                        class: "flex flex-col items-center justify-center gap-0.5 md:gap-1 p-2 md:p-3 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors text-gray-500 dark:text-gray-400 hover:text-primary dark:hover:text-primary-light min-h-[48px]",
                        onclick: move |_| on_tasks.call(class_for_tasks.clone()),
                        span { class: "material-icons-outlined text-lg md:text-xl", "assignment" }
                        span { class: "text-[8px] md:text-[10px] font-medium uppercase", "{locale_ctx.t(\"assignments.title\")}" }
                    }
                    button {
                        class: "flex flex-col items-center justify-center gap-0.5 md:gap-1 p-2 md:p-3 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors text-gray-500 dark:text-gray-400 hover:text-primary dark:hover:text-primary-light min-h-[48px]",
                        onclick: move |_| on_grades.call(class_for_grades.clone()),
                        span { class: "material-icons-outlined text-lg md:text-xl", "show_chart" }
                        span { class: "text-[8px] md:text-[10px] font-medium uppercase", "{locale_ctx.t(\"grades.title\")}" }
                    }
                }
            }
        }
    }
}

#[component]
fn ClassTasksModal(class: StudentClassView, on_close: EventHandler) -> Element {
    let class_id = class.id.clone();
    let tasks_resource = use_resource(move || {
        let id = class_id.clone();
        async move { get_class_assignments_for_student(id).await }
    });
    let locale = use_locale();

    rsx! {
        Modal {
            title: format!("{} - {}", class.name, locale.t("assignments.title")),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div { class: "space-y-4 max-h-96 overflow-y-auto",
                    match &*tasks_resource.read() {
                        None => rsx! {
                            div { class: "text-center py-8 text-gray-500", "{locale.t(\"assignments.loading\")}" }
                        },
                        Some(Err(_)) => rsx! {
                            div { class: "text-center py-8 text-red-500", "{locale.t(\"student.classes.tasks_load_error\")}" }
                        },
                        Some(Ok(tasks)) if tasks.is_empty() => rsx! {
                            div { class: "text-center py-8 text-gray-500", "{locale.t(\"assignments.no_class_assignments\")}" }
                        },
                        Some(Ok(tasks)) => rsx! {
                            for task in tasks.iter() {
                                {
                                    let status = assignment_status_label(
                                        task.presentation_state.display_name(),
                                        locale.current(),
                                    );
                                    let due_date = format_product_date_text(
                                        &task.due_date,
                                        locale.current(),
                                    );
                                    rsx! {
                                        div { class: "p-4 border border-gray-200 dark:border-gray-700 rounded-lg",
                                            div { class: "flex justify-between items-start gap-3",
                                                div {
                                                    h4 { class: "font-semibold text-gray-900 dark:text-white", "{task.title}" }
                                                    p { class: "text-sm text-gray-500 dark:text-gray-400", "{locale.t(\"student.assignments.due_label\")}: {due_date}" }
                                                }
                                                div { class: "flex gap-2",
                                                    span {
                                                        class: match task.presentation_state {
                                                            StudentAssignmentPresentationState::Graded => "px-2 py-1 text-xs font-semibold rounded bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400",
                                                            StudentAssignmentPresentationState::Submitted => "px-2 py-1 text-xs font-semibold rounded bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400",
                                                            StudentAssignmentPresentationState::Overdue => "px-2 py-1 text-xs font-semibold rounded bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400",
                                                            StudentAssignmentPresentationState::Pending => "px-2 py-1 text-xs font-semibold rounded bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400",
                                                        },
                                                        "{status}"
                                                    }
                                                    if let Some(grade) = &task.grade {
                                                        span {
                                                            class: "px-2 py-1 text-xs font-semibold rounded bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400",
                                                            "{grade}"
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
        }
    }
}

#[component]
fn ClassGradesModal(class: StudentClassView, on_close: EventHandler) -> Element {
    let class_id = class.id.clone();
    let grades_resource = use_resource(move || {
        let id = class_id.clone();
        async move { get_class_grades_for_student(id).await }
    });
    let locale = use_locale();

    rsx! {
        Modal {
            title: format!("{} - {}", class.name, locale.t("grades.title")),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div { class: "space-y-4 max-h-96 overflow-y-auto",
                    match &*grades_resource.read() {
                        None => rsx! {
                            div { class: "text-center py-8 text-gray-500", "{locale.t(\"grades.loading\")}" }
                        },
                        Some(Err(_)) => rsx! {
                            div { class: "text-center py-8 text-red-500", "{locale.t(\"student.classes.grades_load_error\")}" }
                        },
                        Some(Ok(grades)) if grades.is_empty() => rsx! {
                            div { class: "text-center py-8 text-gray-500", "{locale.t(\"grades.no_grades\")}" }
                        },
                        Some(Ok(grades)) => rsx! {
                            if !grades.is_empty() {
                                div { class: "p-4 bg-gradient-to-r from-primary/10 to-purple-500/10 rounded-xl mb-4",
                                    div { class: "flex justify-between items-center",
                                        span { class: "text-gray-600 dark:text-gray-300 font-medium", "{locale.t(\"grades.total_graded\")}" }
                                        span { class: "text-2xl font-bold text-primary", "{grades.len()}" }
                                    }
                                }
                            }
                            for grade in grades.iter() {
                                div { class: "p-4 border border-gray-200 dark:border-gray-700 rounded-lg flex justify-between items-center gap-4",
                                    div {
                                        h4 { class: "font-semibold text-gray-900 dark:text-white", "{grade.assignment_title}" }
                                        if let Some(graded_at) = grade.graded_at.as_ref() {
                                            if let Some(grade_date) = format_grade_date(graded_at, locale.current()) {
                                                p { class: "text-sm text-gray-500 dark:text-gray-400", "{grade_date}" }
                                            }
                                        }
                                    }
                                    div { class: "text-end",
                                        GradeToken { value: grade.grade.clone(), class: Some("text-xl font-bold text-primary".to_string()) }
                                        GradeToken { value: grade.points.clone(), class: Some("text-sm text-gray-500".to_string()) }
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
fn ClassMaterialsModal(class: StudentClassView, on_close: EventHandler) -> Element {
    let class_id = class.id.clone();
    let materials_resource = use_resource(move || {
        let id = class_id.clone();
        async move { get_class_materials_for_student(id).await }
    });
    let locale = use_locale();

    rsx! {
        Modal {
            title: format!("{} - {}", class.name, locale.t("classes.materials")),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div { class: "space-y-4 max-h-96 overflow-y-auto",
                    match &*materials_resource.read() {
                        None => rsx! {
                            div { class: "text-center py-8 text-gray-500", "{locale.t(\"materials.loading\")}" }
                        },
                        Some(Err(_)) => rsx! {
                            div { class: "text-center py-8 text-red-500", "{locale.t(\"student.classes.materials_load_error\")}" }
                        },
                        Some(Ok(materials)) if materials.is_empty() => rsx! {
                            div { class: "py-12 text-center",
                                div {
                                    class: "w-16 h-16 bg-gray-100 dark:bg-gray-800 rounded-full flex items-center justify-center mx-auto mb-4",
                                    span { class: "text-3xl", "📁" }
                                }
                                h3 { class: "text-lg font-semibold text-gray-900 dark:text-white mb-2", "{locale.t(\"materials.no_materials_title\")}" }
                                p { class: "text-gray-500 dark:text-gray-400", "{locale.t(\"materials.no_materials_desc\")}" }
                            }
                        },
                        Some(Ok(materials)) => rsx! {
                            for material in materials.iter() {
                                {
                                    let icon = match material.material_type.as_str() {
                                        "document" => "description",
                                        "video" => "play_circle",
                                        "link" => "link",
                                        "image" => "image",
                                        "audio" => "audio_file",
                                        _ => "folder",
                                    };
                                    let icon_color = match material.material_type.as_str() {
                                        "document" => "text-blue-500",
                                        "video" => "text-red-500",
                                        "link" => "text-green-500",
                                        "image" => "text-purple-500",
                                        "audio" => "text-orange-500",
                                        _ => "text-gray-500",
                                    };
                                    let created_at = format_product_date_text(
                                        &material.created_at,
                                        locale.current(),
                                    );

                                    rsx! {
                                        div { class: "p-4 border border-gray-200 dark:border-gray-700 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors",
                                            div { class: "flex items-start gap-4",
                                                div {
                                                    class: "w-10 h-10 rounded-lg bg-gray-100 dark:bg-gray-800 flex items-center justify-center flex-shrink-0",
                                                    span { class: "material-icons-outlined {icon_color}", "{icon}" }
                                                }
                                                div { class: "flex-1 min-w-0",
                                                    div { class: "flex items-center gap-2",
                                                        h4 { class: "font-semibold text-gray-900 dark:text-white truncate", "{material.title}" }
                                                        if material.is_required {
                                                            span {
                                                                class: "px-1.5 py-0.5 text-[10px] font-bold bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400 rounded",
                                                                "{locale.t(\"common.required\")}"
                                                            }
                                                        }
                                                    }
                                                    if let Some(desc) = &material.description {
                                                        p { class: "text-sm text-gray-500 dark:text-gray-400 mt-1 line-clamp-2", "{desc}" }
                                                    }
                                                    p { class: "text-xs text-gray-400 dark:text-gray-500 mt-2", "{locale.t(\"materials.added_prefix\")}{created_at}" }
                                                }
                                                div { class: "flex-shrink-0",
                                                    if material.file_url.is_some() || material.external_link.is_some() {
                                                        {
                                                            let url = material.file_url.clone()
                                                                .or_else(|| material.external_link.clone())
                                                                .unwrap_or_default();
                                                            rsx! {
                                                                a {
                                                                    href: "{url}",
                                                                    target: "_blank",
                                                                    rel: "noopener noreferrer",
                                                                    class: "flex items-center gap-1 px-3 py-1.5 bg-primary/10 text-primary hover:bg-primary/20 rounded-lg text-sm font-medium transition-colors",
                                                                    span { class: "material-icons-outlined text-base", "download" }
                                                                    "{locale.t(\"common.open\")}"
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
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn student_class_dialogs_do_not_render_backend_error_bodies() {
        let source = include_str!("classes.rs");
        let implementation = source
            .split("#[cfg(test)]")
            .next()
            .expect("class implementation before tests");
        assert!(!implementation.contains("Some(Err(e))"));
        assert!(!implementation.contains("{e}"));
    }

    #[test]
    fn class_assignment_and_material_dates_are_localized_at_the_web_boundary() {
        let source = include_str!("classes.rs");
        let implementation = source
            .split("#[cfg(test)]")
            .next()
            .expect("class implementation before tests");
        assert!(implementation.contains("assignment_status_label"));
        assert!(implementation.contains("format_product_date_text"));
        assert!(!implementation.contains("\"{task.presentation_state.display_name()}\""));
        assert!(!implementation.contains("\"{task.due_date}\""));
        assert!(!implementation.contains("\"{material.created_at}\""));
    }
}
