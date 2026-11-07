//! Persisted, non-authoritative AI grading suggestions for teacher review.
//!
//! Only an active, assigned teacher can initiate generation, using a separately
//! teacher-approved rubric. The exact submission revision and rubric approval
//! are checked both before and after the LLM, then again by database triggers.
//! These functions never update or publish a student's grade.

use crate::ai_grading_contract::GradingSuggestion;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "server")]
use crate::ai_grading_contract::{validate_authoritative_rubric, ExpectedCriterion};
#[cfg(feature = "server")]
use crate::rls_context::AuthorizedPool;
#[cfg(feature = "server")]
use crate::server_functions::ai_grading_functions::find_authorized_grading_source;
#[cfg(feature = "server")]
use crate::services::llm_service::ExternalLlmClient;
#[cfg(feature = "server")]
use sqlx::Row;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredGradingSuggestion {
    pub id: Uuid,
    pub submission_id: Uuid,
    pub rubric_approval_id: Uuid,
    pub created_at: String,
    pub status: String,
    pub suggestion: GradingSuggestion,
}

#[cfg(feature = "server")]
struct ApprovedRubric {
    id: Uuid,
    school_id: Uuid,
    grade_scale: i16,
    fingerprint: String,
    criteria: Vec<ExpectedCriterion>,
}

#[cfg(feature = "server")]
fn map_suggestion(row: sqlx::postgres::PgRow) -> Result<StoredGradingSuggestion, ServerFnError> {
    let criteria = serde_json::from_value(row.get("criteria"))
        .map_err(|_| ServerFnError::new("grading.suggestion_corrupt"))?;
    Ok(StoredGradingSuggestion {
        id: row.get("id"),
        submission_id: row.get("submission_id"),
        rubric_approval_id: row.get("rubric_approval_id"),
        created_at: row.get("created_at"),
        status: row.get("status"),
        suggestion: GradingSuggestion {
            contract_version: row.get::<i16, _>("contract_version") as u16,
            submission_revision: row.get("submission_revision"),
            grade_scale: row.get("grade_scale"),
            suggested_grade: row.get("suggested_grade"),
            confidence: row.get("confidence"),
            feedback: row.get("feedback"),
            criteria,
        },
    })
}

#[cfg(feature = "server")]
async fn current_approved_rubric(
    pool: &AuthorizedPool,
    submission_id: Uuid,
    teacher_user_id: Uuid,
) -> Result<Option<ApprovedRubric>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT approval.id, approval.school_id, approval.grade_scale,
               approval.rubric_fingerprint, approval.criteria
        FROM public.ai_grading_rubric_approvals approval
        JOIN public.submissions s
          ON s.custom_assignment_id = approval.custom_assignment_id
        JOIN public.custom_assignments ca ON ca.id = s.custom_assignment_id
        JOIN public.assignments a ON a.id = ca.assignment_id
        JOIN public.teachers t ON t.id = a.teacher_id
          AND t.id = approval.teacher_id
        JOIN public.users u ON u.id = t.user_id
          AND u.school_id = approval.school_id AND u.is_active
        WHERE s.id = $1 AND u.id = $2 AND approval.revoked_at IS NULL
          AND s.grade_scale = approval.grade_scale
          AND s.grade IS NULL AND s.graded_by IS NULL
          AND ca.graded_at IS NULL
        "#,
    )
    .bind(submission_id)
    .bind(teacher_user_id)
    .fetch_optional(pool)
    .await?;
    row.map(|row| {
        let criteria: Vec<ExpectedCriterion> = serde_json::from_value(row.get("criteria"))
            .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
        Ok(ApprovedRubric {
            id: row.get("id"),
            school_id: row.get("school_id"),
            grade_scale: row.get("grade_scale"),
            fingerprint: row.get("rubric_fingerprint"),
            criteria,
        })
    })
    .transpose()
}

#[cfg(feature = "server")]
const READ_SUGGESTION: &str = r#"
    SELECT id, submission_id, submission_revision, rubric_approval_id,
           contract_version, grade_scale, suggested_grade, confidence,
           feedback, criteria, status, created_at::text AS created_at
    FROM public.ai_grading_suggestions
    WHERE submission_id = $1 AND submission_revision = $2
      AND rubric_approval_id = $3 AND status = 'pending_review'
"#;

#[cfg(feature = "server")]
async fn current_stored_suggestion(
    pool: &AuthorizedPool,
    submission_id: Uuid,
    revision: Uuid,
    approval_id: Uuid,
) -> Result<Option<StoredGradingSuggestion>, ServerFnError> {
    let row = sqlx::query(READ_SUGGESTION)
        .bind(submission_id)
        .bind(revision)
        .bind(approval_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ServerFnError::new("grading.suggestion_unavailable"))?;
    row.map(map_suggestion).transpose()
}

/// Teacher-initiated generation: one immutable, pending-review suggestion per
/// submission revision. An existing suggestion is reused (idempotent). No
/// caller-supplied rubric, confidence or grade can enter the persistence path.
#[post("/api/ai-grading/suggestions/generate")]
pub async fn generate_teacher_grading_suggestion(
    submission_id: Uuid,
) -> Result<StoredGradingSuggestion, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (user, pool) =
            crate::server_functions::rls_helpers::extract_user_with_full_rls().await?;
        if user.role != "Teacher" || submission_id.is_nil() {
            return Err(ServerFnError::new("grading.forbidden"));
        }
        let user_id =
            Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("grading.forbidden"))?;
        let source = find_authorized_grading_source(&pool, submission_id, user_id)
            .await
            .map_err(|_| ServerFnError::new("grading.suggestion_unavailable"))?
            .ok_or_else(|| ServerFnError::new("grading.not_found"))?;
        if source.submission_revision.is_nil()
            || source.submission_text.trim().is_empty()
            || source.assignment_instructions.trim().is_empty()
        {
            return Err(ServerFnError::new("grading.text_required"));
        }
        let approved = current_approved_rubric(&pool, submission_id, user_id)
            .await
            .map_err(|_| ServerFnError::new("grading.suggestion_unavailable"))?
            .ok_or_else(|| ServerFnError::new("grading.rubric_approval_required"))?;
        if approved.school_id != source.school_id || approved.grade_scale != source.grade_scale {
            return Err(ServerFnError::new("grading.rubric_conflict"));
        }
        validate_authoritative_rubric(&approved.criteria, approved.grade_scale)
            .map_err(|_| ServerFnError::new("grading.invalid_rubric"))?;
        if let Some(previous) = current_stored_suggestion(
            &pool,
            submission_id,
            source.submission_revision,
            approved.id,
        )
        .await?
        {
            return Ok(previous);
        }

        let llm =
            ExternalLlmClient::new().map_err(|_| ServerFnError::new("grading.unavailable"))?;
        let suggestion = llm
            .suggest_assignment_grade_for_school(
                source.school_id,
                source.submission_revision,
                &source.assignment_instructions,
                &source.submission_text,
                &approved.criteria,
                approved.grade_scale,
            )
            .await
            .map_err(|_| ServerFnError::new("grading.unavailable"))?;
        suggestion
            .validate_against_rubric(source.submission_revision, &approved.criteria)
            .map_err(|_| ServerFnError::new("grading.invalid_suggestion"))?;

        // The model request may take seconds. Re-read both authoritative
        // resources and fail closed if the teacher, answer or rubric changed.
        let latest = find_authorized_grading_source(&pool, submission_id, user_id)
            .await
            .map_err(|_| ServerFnError::new("grading.suggestion_unavailable"))?
            .ok_or_else(|| ServerFnError::new("grading.stale_submission"))?;
        let latest_approval = current_approved_rubric(&pool, submission_id, user_id)
            .await
            .map_err(|_| ServerFnError::new("grading.suggestion_unavailable"))?
            .ok_or_else(|| ServerFnError::new("grading.rubric_revoked"))?;
        if source.submission_revision != latest.submission_revision
            || source.school_id != latest.school_id
            || source.grade_scale != latest.grade_scale
            || source.submission_text != latest.submission_text
            || source.assignment_instructions != latest.assignment_instructions
            || approved.id != latest_approval.id
            || approved.fingerprint != latest_approval.fingerprint
            || approved.criteria != latest_approval.criteria
        {
            return Err(ServerFnError::new("grading.stale_submission"));
        }

        let evidence = serde_json::to_value(&suggestion.criteria)
            .map_err(|_| ServerFnError::new("grading.invalid_suggestion"))?;
        let row = sqlx::query(
            r#"
            INSERT INTO public.ai_grading_suggestions (
                submission_id, submission_revision, school_id,
                rubric_approval_id, rubric_fingerprint, contract_version,
                grade_scale, suggested_grade, confidence, feedback, criteria
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            ON CONFLICT (submission_id, submission_revision, rubric_approval_id) DO NOTHING
            RETURNING id, submission_id, submission_revision, rubric_approval_id,
                      contract_version, grade_scale, suggested_grade, confidence,
                      feedback, criteria, status, created_at::text AS created_at
            "#,
        )
        .bind(submission_id)
        .bind(suggestion.submission_revision)
        .bind(source.school_id)
        .bind(approved.id)
        .bind(&approved.fingerprint)
        .bind(suggestion.contract_version as i16)
        .bind(suggestion.grade_scale)
        .bind(suggestion.suggested_grade)
        .bind(suggestion.confidence)
        .bind(&suggestion.feedback)
        .bind(evidence)
        .fetch_optional(&*pool)
        .await
        .map_err(|error| {
            tracing::warn!(%error, "Authorized AI grading suggestion insertion rejected");
            ServerFnError::new("grading.suggestion_unavailable")
        })?;

        if let Some(row) = row {
            return map_suggestion(row);
        }
        // A concurrent identical request may have committed first. Reuse the
        // same approved artifact only; never overwrite or relabel evidence.
        current_stored_suggestion(
            &pool,
            submission_id,
            suggestion.submission_revision,
            approved.id,
        )
        .await?
        .ok_or_else(|| ServerFnError::new("grading.suggestion_conflict"))
    }
    #[cfg(not(feature = "server"))]
    {
        let _ = submission_id;
        Err(ServerFnError::new("server only"))
    }
}

/// The assigned teacher can inspect only the pending suggestion for the
/// current revision and *currently active* approved rubric. Never expose
/// pending AI evidence to Student or Parent endpoints.
#[get("/api/ai-grading/suggestions/current?submission_id")]
pub async fn get_current_teacher_grading_suggestion(
    submission_id: Uuid,
) -> Result<Option<StoredGradingSuggestion>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (user, pool) =
            crate::server_functions::rls_helpers::extract_user_with_full_rls().await?;
        if user.role != "Teacher" || submission_id.is_nil() {
            return Err(ServerFnError::new("grading.forbidden"));
        }
        let user_id =
            Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("grading.forbidden"))?;
        let source = find_authorized_grading_source(&pool, submission_id, user_id)
            .await
            .map_err(|_| ServerFnError::new("grading.suggestion_unavailable"))?;
        let approved = current_approved_rubric(&pool, submission_id, user_id)
            .await
            .map_err(|_| ServerFnError::new("grading.suggestion_unavailable"))?;
        match (source, approved) {
            (Some(source), Some(approved))
                if source.school_id == approved.school_id
                    && source.grade_scale == approved.grade_scale =>
            {
                current_stored_suggestion(
                    &pool,
                    submission_id,
                    source.submission_revision,
                    approved.id,
                )
                .await
            }
            _ => Ok(None),
        }
    }
    #[cfg(not(feature = "server"))]
    {
        let _ = submission_id;
        Err(ServerFnError::new("server only"))
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn suggestion_generation_requires_approved_authority_and_never_publishes() {
        let source = include_str!("ai_grading_suggestion_functions.rs");
        let production = source
            .split("#[cfg(all(test, feature = \"server\"))]")
            .next()
            .unwrap_or(source);
        for required in [
            "find_authorized_grading_source",
            "current_approved_rubric",
            "validate_authoritative_rubric",
            "validate_against_rubric",
            "latest_approval",
            "ON CONFLICT (submission_id, submission_revision) DO NOTHING",
        ] {
            assert!(production.contains(required));
        }
        for forbidden in [
            "UPDATE public.submissions",
            "UPDATE public.custom_assignments",
        ] {
            assert!(!production.contains(forbidden));
        }
    }
}
