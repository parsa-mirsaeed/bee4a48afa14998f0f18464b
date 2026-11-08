//! Explicit teacher-authored rubric and review of immutable AI suggestions.
//! Never uses the model output as an official grade without a Teacher decision.
use crate::i18n::use_locale;
use api::ai_grading_contract::{validate_authoritative_rubric, ExpectedCriterion};
use api::server_functions::ai_grading_decision_functions::{
    review_teacher_ai_grading_suggestion, AiGradingDecision,
};
use api::server_functions::ai_grading_rubric_functions::{
    approve_teacher_grading_rubric, get_teacher_grading_rubric,
};
use api::server_functions::ai_grading_suggestion_functions::{
    generate_teacher_grading_suggestion, get_current_teacher_grading_suggestion,
};
use dioxus::prelude::*;
use uuid::Uuid;

/// Teacher-typed rows are the sole authoritative source of grading criteria.
fn parse_teacher_criteria(value: &str, grade_scale: i16) -> Option<Vec<ExpectedCriterion>> {
    let mut entries = Vec::new();
    for line in value.lines().filter(|line| !line.trim().is_empty()) {
        let (label, points) = line.split_once('|')?;
        // Treat trimming as form normalization. Model-produced identifiers are
        // never normalized: their exact IDs must match stored approvals.
        let criterion_id = label.trim().to_string();
        let normalized_points: String = points
            .trim()
            .chars()
            .map(|digit| match digit {
                '۰' | '٠' => '0',
                '۱' | '١' => '1',
                '۲' | '٢' => '2',
                '۳' | '٣' => '3',
                '۴' | '٤' => '4',
                '۵' | '٥' => '5',
                '۶' | '٦' => '6',
                '۷' | '٧' => '7',
                '۸' | '٨' => '8',
                '۹' | '٩' => '9',
                '٫' => '.',
                other => other,
            })
            .collect();
        let possible_points = normalized_points.parse::<f64>().ok()?;
        entries.push(ExpectedCriterion {
            criterion_id,
            possible_points,
        });
    }
    validate_authoritative_rubric(&entries, grade_scale).ok()?;
    Some(entries)
}

#[component]
pub(super) fn AiGradingPanel(
    submission_id: Uuid,
    custom_assignment_id: Uuid,
    grade_scale: i16,
    on_graded: EventHandler,
) -> Element {
    let locale = use_locale();
    let mut criteria_lines = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(|| None::<String>);
    let mut revision = use_signal(|| 0_u64);
    let mut evidence = use_resource(move || {
        let _revision = revision();
        async move {
            let rubric = get_teacher_grading_rubric(custom_assignment_id).await;
            let suggestion = get_current_teacher_grading_suggestion(submission_id).await;
            (rubric, suggestion)
        }
    });

    let current = evidence.read();
    let loading = current.is_none();
    let read_failed = current
        .as_ref()
        .is_some_and(|(r, s)| r.is_err() || s.is_err());
    let rubric = current
        .as_ref()
        .and_then(|(r, _)| r.as_ref().ok())
        .and_then(|v| v.clone());
    let suggestion = current
        .as_ref()
        .and_then(|(_, s)| s.as_ref().ok())
        .and_then(|v| v.clone());
    drop(current);

    rsx! {
        section {
            class: "rounded-2xl border border-indigo-200 dark:border-indigo-800/60 bg-gradient-to-br from-indigo-50/70 to-white dark:from-indigo-950/30 dark:to-gray-900 p-4 md:p-5 space-y-3",
            h4 { class: "text-base font-semibold text-gray-900 dark:text-white flex items-center gap-2",
                span { class: "material-icons-outlined text-indigo-600", "auto_awesome" }
                "{locale.t(\"ai_grading.heading\")}"
            }
            p { class: "text-sm text-gray-600 dark:text-gray-300", "{locale.t(\"ai_grading.description\")}" }

            if loading {
                p { class: "text-sm text-gray-500", "{locale.t(\"common.loading\")}" }
            } else if read_failed {
                p { class: "text-sm text-red-600", "{locale.t(\"ai_grading.load_failed\")}" }
            } else if rubric.is_none() {
                label { class: "block text-sm font-medium text-gray-700 dark:text-gray-200",
                    "{locale.t(\"ai_grading.rubric_label\")}"
                }
                textarea {
                    class: "w-full min-h-24 rounded-xl border border-indigo-200 dark:border-indigo-800 p-3 bg-white dark:bg-gray-950 text-sm text-gray-900 dark:text-white",
                    placeholder: "{locale.t(\"ai_grading.rubric_placeholder\")}",
                    disabled: busy(),
                    value: "{criteria_lines}",
                    oninput: move |e| criteria_lines.set(e.value()),
                }
                button {
                    class: "rounded-xl px-4 py-2.5 bg-indigo-600 hover:bg-indigo-700 text-white text-sm font-semibold disabled:opacity-50",
                    disabled: busy() || criteria_lines().trim().is_empty(),
                    onclick: move |_| {
                        if let Some(criteria) = parse_teacher_criteria(&criteria_lines(), grade_scale) {
                            notice.set(None);
                            busy.set(true);
                            spawn(async move {
                                let result = approve_teacher_grading_rubric(
                                    custom_assignment_id, grade_scale, criteria,
                                ).await;
                                busy.set(false);
                                match result {
                                    Ok(_) => {
                                        revision += 1;
                                        evidence.restart();
                                    }
                                    Err(_) => notice.set(Some(locale.t("ai_grading.failed"))),
                                }
                            });
                        } else {
                            notice.set(Some(locale.t("ai_grading.invalid_rubric")));
                        }
                    },
                    "{locale.t(\"ai_grading.approve\")}"
                }
            } else if let Some(stored) = suggestion.clone() {
                div {
                    class: "rounded-xl bg-white dark:bg-gray-900 p-4 border border-indigo-200 dark:border-indigo-800 space-y-3",
                    div { class: "flex items-center justify-between gap-3 flex-wrap",
                        div {
                            p { class: "text-xs uppercase tracking-wide text-indigo-700 dark:text-indigo-300", "{locale.t(\"ai_grading.suggestion\")}" }
                            p { class: "text-2xl font-bold text-gray-900 dark:text-white",
                                "{stored.suggestion.suggested_grade:.2} / {stored.suggestion.grade_scale}"
                            }
                        }
                        p { class: "text-xs text-gray-500 dark:text-gray-400",
                            "{locale.t(\"ai_grading.confidence\")}: {stored.suggestion.confidence:.0}%"
                        }
                    }
                    p { class: "text-sm font-medium text-gray-900 dark:text-white", "{locale.t(\"ai_grading.feedback\")}" }
                    p { class: "whitespace-pre-wrap text-sm text-gray-700 dark:text-gray-300", "{stored.suggestion.feedback}" }
                    p { class: "text-sm font-medium text-gray-900 dark:text-white", "{locale.t(\"ai_grading.evidence\")}" }
                    for scored in stored.suggestion.criteria.iter() {
                        div { key: "{scored.criterion_id}", class: "rounded-lg bg-gray-50 dark:bg-gray-800 p-3",
                            p { class: "text-sm font-semibold text-gray-800 dark:text-gray-100",
                                "{scored.criterion_id}: {scored.awarded_points:.2}/{scored.possible_points:.2}"
                            }
                            p { class: "text-xs text-gray-600 dark:text-gray-300", "{scored.evidence}" }
                        }
                    }
                    div { class: "flex gap-2 flex-wrap",
                        button {
                            class: "rounded-xl px-4 py-2.5 bg-emerald-700 hover:bg-emerald-800 text-white text-sm font-semibold disabled:opacity-50",
                            disabled: busy(),
                            onclick: move |_| {
                                busy.set(true);
                                notice.set(None);
                                let id = stored.id;
                                let graded = on_graded.clone();
                                spawn(async move {
                                    let result = review_teacher_ai_grading_suggestion(id, AiGradingDecision::Accept).await;
                                    busy.set(false);
                                    if matches!(result, Ok(true)) {
                                        graded.call(());
                                    } else {
                                        notice.set(Some(locale.t("ai_grading.failed")));
                                        revision += 1;
                                        evidence.restart();
                                    }
                                });
                            },
                            "{locale.t(\"ai_grading.accept\")}"
                        }
                        button {
                            class: "rounded-xl px-4 py-2.5 border border-red-300 dark:border-red-800 text-red-700 dark:text-red-300 text-sm font-semibold hover:bg-red-50 dark:hover:bg-red-950 disabled:opacity-50",
                            disabled: busy(),
                            onclick: move |_| {
                                busy.set(true);
                                notice.set(None);
                                let id = stored.id;
                                spawn(async move {
                                    let result = review_teacher_ai_grading_suggestion(id, AiGradingDecision::Reject).await;
                                    busy.set(false);
                                    if matches!(result, Ok(true)) {
                                        notice.set(Some(locale.t("ai_grading.rejected")));
                                    } else {
                                        notice.set(Some(locale.t("ai_grading.failed")));
                                    }
                                    revision += 1;
                                    evidence.restart();
                                });
                            },
                            "{locale.t(\"ai_grading.reject\")}"
                        }
                    }
                }
            } else {
                p { class: "text-sm text-gray-600 dark:text-gray-300",
                    "{locale.t(\"ai_grading.pending\")}"
                }
                button {
                    class: "rounded-xl px-4 py-2.5 bg-indigo-600 hover:bg-indigo-700 text-white text-sm font-semibold disabled:opacity-50",
                    disabled: busy(),
                    onclick: move |_| {
                        busy.set(true);
                        notice.set(None);
                        spawn(async move {
                            let result = generate_teacher_grading_suggestion(submission_id).await;
                            busy.set(false);
                            if result.is_err() {
                                notice.set(Some(locale.t("ai_grading.failed")));
                            }
                            revision += 1;
                            evidence.restart();
                        });
                    },
                    "{locale.t(\"ai_grading.generate\")}"
                }
            }
            if busy() {
                p { class: "text-xs font-medium text-indigo-600 dark:text-indigo-300",
                    "{locale.t(\"ai_grading.busy\")}"
                }
            }
            if let Some(message) = notice() {
                p { role: "status", class: "text-sm text-amber-700 dark:text-amber-300", "{message}" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_explicit_valid_teacher_criteria_pass() {
        assert!(parse_teacher_criteria("reasoning | 12\naccuracy | 8", 20).is_some());
        assert!(parse_teacher_criteria("استدلال | ۱۲\nدقت | ۸", 20).is_some());
        assert!(parse_teacher_criteria(" | 20", 20).is_none());
        assert!(parse_teacher_criteria("wrong | 30", 20).is_none());
        assert!(parse_teacher_criteria("duplicated | 10\nduplicated | 10", 20).is_none());
    }
}
