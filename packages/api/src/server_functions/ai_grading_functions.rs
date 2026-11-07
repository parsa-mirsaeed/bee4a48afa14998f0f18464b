//! Teacher-only, non-persistent AI grading preview.
//!
//! The teacher supplies the rubric explicitly. AI-generated personalization
//! rubrics are NOT silently treated as teacher-approved grading criteria.
//! The submitted revision is read under a request-scoped authorization
//! transaction, and model output is never persisted or published here.

use crate::services::ai_grading_contract::{ExpectedCriterion, GradingSuggestion};
use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::rls_context::AuthorizedPool;
#[cfg(feature = "server")]
use crate::services::ai_grading_contract::validate_authoritative_rubric;
#[cfg(feature = "server")]
use crate::services::llm_service::ExternalLlmClient;
#[cfg(feature = "server")]
use sqlx::Row;
#[cfg(feature = "server")]
use uuid::Uuid;

#[cfg(feature = "server")]
const AUTHORIZED_GRADING_SOURCE: &str = r#"
    SELECT
        cs.school_id,
        s.revision,
        s.grade_scale,
        s.content ->> 'text' AS submission_text,
        COALESCE(
            NULLIF(ca.prompt_ctx #>> '{personalized_assignment,body}', ''),
            a.body
        ) AS assignment_instructions
    FROM public.submissions s
    JOIN public.custom_assignments ca
      ON ca.id = s.custom_assignment_id
     AND ca.student_id = s.student_id
    JOIN public.assignments a ON a.id = ca.assignment_id
    JOIN public.class_sections cs ON cs.id = a.class_section_id
    JOIN public.teachers t
      ON t.id = a.teacher_id
     AND t.school_id = cs.school_id
    JOIN public.users teacher_user
      ON teacher_user.id = t.user_id
     AND teacher_user.school_id = cs.school_id
     AND teacher_user.is_active = TRUE
    JOIN public.roles teacher_role ON teacher_role.id = teacher_user.role_id
    JOIN public.teaching_assignments ta
      ON ta.teacher_id = t.id
     AND ta.class_section_id = a.class_section_id
    JOIN public.students st
      ON st.id = ca.student_id
     AND st.school_id = cs.school_id
    JOIN public.users student_user
      ON student_user.id = st.user_id
     AND student_user.school_id = cs.school_id
     AND student_user.is_active = TRUE
    JOIN public.enrollments e
      ON e.student_id = st.id
     AND e.class_section_id = a.class_section_id
    WHERE s.id = $1
      AND teacher_user.id = $2
      AND teacher_role.name::text = 'Teacher'
      AND a.status = 'Published'::assignment_status
      AND ca.status = 'Submitted'::custom_status
      AND ca.submitted_at IS NOT NULL
      AND s.submitted_at IS NOT NULL
      AND ca.graded_at IS NULL
      AND s.grade IS NULL
      AND s.graded_by IS NULL
    LIMIT 1
"#;

/// Create an ephemeral suggestion for the assigned teacher. This endpoint
/// intentionally does not mutate submissions or private grading artifacts.
/// A future persistence workflow must re-check revision and rubric fingerprint
/// *after* the LLM call, under a fresh authorized transaction.
#[post("/api/ai-grading/preview")]
pub async fn preview_teacher_ai_grading(
    submission_id: uuid::Uuid,
    grade_scale: i16,
    rubric: Vec<ExpectedCriterion>,
) -> Result<GradingSuggestion, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (user, pool) =
            crate::server_functions::rls_helpers::extract_user_with_full_rls().await?;
        if user.role != "Teacher" {
            return Err(ServerFnError::new("grading.forbidden"));
        }
        let user_id =
            Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("grading.forbidden"))?;
        if submission_id.is_nil() {
            return Err(ServerFnError::new("grading.invalid_submission"));
        }
        validate_authoritative_rubric(&rubric, grade_scale)
            .map_err(|_| ServerFnError::new("grading.invalid_rubric"))?;

        let source = find_authorized_grading_source(&pool, submission_id, user_id)
            .await
            .map_err(|error| {
                tracing::error!(error = %error, "Teacher grading source lookup failed");
                ServerFnError::new("grading.unavailable")
            })?
            .ok_or_else(|| ServerFnError::new("grading.not_found"))?;

        if source.grade_scale != grade_scale {
            return Err(ServerFnError::new("grading.scale_mismatch"));
        }
        if source.submission_revision.is_nil()
            || source.assignment_instructions.trim().is_empty()
            || source.submission_text.trim().is_empty()
        {
            // Attachment-only submissions require an independently authorized
            // OCR pipeline. Never send an empty answer or guess a grade.
            return Err(ServerFnError::new("grading.text_required"));
        }

        let client =
            ExternalLlmClient::new().map_err(|_| ServerFnError::new("grading.unavailable"))?;
        let suggestion = client
            .suggest_assignment_grade_for_school(
                source.school_id,
                source.submission_revision,
                &source.assignment_instructions,
                &source.submission_text,
                &rubric,
                grade_scale,
            )
            .await
            .map_err(|_| ServerFnError::new("grading.unavailable"))?;

        // A student may resubmit while the gateway is processing. Re-check
        // authorization and the current revision after inference, before
        // returning even a non-persistent preview.
        let latest = find_authorized_grading_source(&pool, submission_id, user_id)
            .await
            .map_err(|_| ServerFnError::new("grading.unavailable"))?
            .ok_or_else(|| ServerFnError::new("grading.stale_submission"))?;
        if latest.submission_revision != source.submission_revision
            || latest.school_id != source.school_id
            || latest.grade_scale != source.grade_scale
            || latest.submission_text != source.submission_text
            || latest.assignment_instructions != source.assignment_instructions
        {
            return Err(ServerFnError::new("grading.stale_submission"));
        }
        Ok(suggestion)
    }

    #[cfg(not(feature = "server"))]
    {
        let _ = (submission_id, grade_scale, rubric);
        Err(ServerFnError::new("server only"))
    }
}

#[cfg(feature = "server")]
struct AuthorizedGradingSource {
    school_id: Uuid,
    submission_revision: Uuid,
    grade_scale: i16,
    assignment_instructions: String,
    submission_text: String,
}

#[cfg(feature = "server")]
async fn find_authorized_grading_source(
    pool: &AuthorizedPool,
    submission_id: Uuid,
    teacher_user_id: Uuid,
) -> Result<Option<AuthorizedGradingSource>, sqlx::Error> {
    let row = sqlx::query(AUTHORIZED_GRADING_SOURCE)
        .bind(submission_id)
        .bind(teacher_user_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|row| AuthorizedGradingSource {
        school_id: row.get("school_id"),
        submission_revision: row.get("revision"),
        grade_scale: row.get("grade_scale"),
        assignment_instructions: row.get("assignment_instructions"),
        submission_text: row
            .get::<Option<String>, _>("submission_text")
            .unwrap_or_default(),
    }))
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;
    use crate::services::ai_grading_contract::SuggestionValidationError;

    #[test]
    fn request_requires_a_valid_teacher_supplied_rubric() {
        let valid = vec![
            ExpectedCriterion {
                criterion_id: "استدلال".to_string(),
                possible_points: 12.0,
            },
            ExpectedCriterion {
                criterion_id: "accuracy".to_string(),
                possible_points: 8.0,
            },
        ];
        assert_eq!(validate_authoritative_rubric(&valid, 20), Ok(()));
        assert_eq!(
            validate_authoritative_rubric(&valid, 100),
            Err(SuggestionValidationError::InvalidCriteria)
        );
        assert_eq!(
            validate_authoritative_rubric(&[], 20),
            Err(SuggestionValidationError::InvalidCriteria)
        );
    }

    #[test]
    fn teacher_scoped_query_requires_school_membership_and_ungraded_revision() {
        for condition in [
            "teacher_user.id = $2",
            "teacher_role.name::text = 'Teacher'",
            "teacher_user.is_active = TRUE",
            "t.school_id = cs.school_id",
            "student_user.school_id = cs.school_id",
            "student_user.is_active = TRUE",
            "JOIN public.teaching_assignments",
            "JOIN public.enrollments",
            "s.id = $1",
            "ca.status = 'Submitted'",
            "s.submitted_at IS NOT NULL",
            "ca.graded_at IS NULL",
            "s.grade IS NULL",
            "s.graded_by IS NULL",
        ] {
            assert!(
                AUTHORIZED_GRADING_SOURCE.contains(condition),
                "Missing grading authorization condition: {condition}"
            );
        }
    }

    #[test]
    fn preview_never_publishes_or_persists_an_ai_grade() {
        let implementation = include_str!("ai_grading_functions.rs");
        let implementation = implementation
            .split("#[cfg(all(test, feature = \"server\"))]")
            .next()
            .unwrap_or(implementation);
        assert!(!implementation.contains("UPDATE public.submissions"));
        assert!(!implementation.contains("INSERT INTO public.ai_grading_suggestions"));
        assert!(!implementation.contains("UPDATE public.ai_grading_suggestions"));
    }
}

#[cfg(all(test, feature = "server"))]
#[path = "../services/ai_grading_preview_db_tests.rs"]
mod database_tests;
