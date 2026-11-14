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
use crate::server_functions::ai_grading_functions::{
    find_authorized_grading_source, AuthorizedGradingSource,
};
#[cfg(feature = "server")]
use crate::services::ai_grading_pdf_source::{
    find_authorized_pdf_metadata, load_authorized_native_pdf, school_pdf_grading_enabled,
    AuthorizedPdfMetadata,
};
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
    submission_revision: Uuid,
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
               s.revision AS submission_revision,
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
            submission_revision: row.get("submission_revision"),
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
      AND NOT EXISTS (
          SELECT 1 FROM public.ai_grading_decisions decision
          WHERE decision.suggestion_id = public.ai_grading_suggestions.id
      )
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
        return generate_authorized_teacher_suggestion(&pool, submission_id, user_id).await;
    }
    #[cfg(not(feature = "server"))]
    {
        let _ = submission_id;
        Err(ServerFnError::new("server only"))
    }
}

/// Shared audited core for both the Teacher HTTP endpoint and a separately
/// authorized background worker. Caller MUST establish a live Teacher-scoped
/// AuthorizedTx; this function never reads through a global system-job actor.
#[cfg(feature = "server")]
pub(crate) async fn generate_authorized_teacher_suggestion(
    pool: &AuthorizedPool,
    submission_id: Uuid,
    user_id: Uuid,
) -> Result<StoredGradingSuggestion, ServerFnError> {
    // The text-only source intentionally excludes finalized attachments.
    // If and only if a current teacher-approved rubric exists, fall back to
    // the independent exact-revision native PDF authorization query.
    let approved = current_approved_rubric(pool, submission_id, user_id)
        .await
        .map_err(|_| ServerFnError::new("grading.suggestion_unavailable"))?
        .ok_or_else(|| ServerFnError::new("grading.rubric_approval_required"))?;
    let text_source = find_authorized_grading_source(pool, submission_id, user_id)
        .await
        .map_err(|_| ServerFnError::new("grading.suggestion_unavailable"))?;
    let pdf_metadata: Option<AuthorizedPdfMetadata> = if text_source.is_none() {
        find_authorized_pdf_metadata(
            pool,
            submission_id,
            user_id,
            approved.submission_revision,
            approved.id,
        )
        .await
        .map_err(|_| ServerFnError::new("grading.suggestion_unavailable"))?
    } else {
        None
    };
    let source = match (text_source, pdf_metadata.as_ref()) {
        (Some(source), None) => source,
        (None, Some(meta)) => AuthorizedGradingSource {
            school_id: meta.school_id,
            submission_revision: meta.submission_revision,
            grade_scale: approved.grade_scale,
            assignment_instructions: meta.assignment_instructions.clone(),
            submission_text: meta.submission_text.clone(),
        },
        _ => return Err(ServerFnError::new("grading.not_found")),
    };
    if source.submission_revision.is_nil()
        || source.assignment_instructions.trim().is_empty()
        || (pdf_metadata.is_none() && source.submission_text.trim().is_empty())
    {
        return Err(ServerFnError::new("grading.text_required"));
    }
    if approved.submission_revision != source.submission_revision {
        return Err(ServerFnError::new("grading.stale_submission"));
    }
    if approved.school_id != source.school_id || approved.grade_scale != source.grade_scale {
        return Err(ServerFnError::new("grading.rubric_conflict"));
    }
    // Existing suggestions are idempotent, but they must never bypass a
    // revoked PDF school opt-in. Check *before* returning the cached result,
    // and continue checking again after inference before writing evidence.
    if pdf_metadata.is_some()
        && !school_pdf_grading_enabled(pool, source.school_id)
            .await
            .map_err(|_| ServerFnError::new("grading.suggestion_unavailable"))?
    {
        return Err(ServerFnError::new("grading.pdf_school_disabled"));
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

    let llm = ExternalLlmClient::new().map_err(|_| ServerFnError::new("grading.unavailable"))?;
    let suggestion = if pdf_metadata.is_some() {
        // The application state only supplies server-owned private Storage
        // credentials. The query itself runs under the live Teacher RLS pool.
        let state = crate::app_state::APP_STATE
            .get()
            .ok_or_else(|| ServerFnError::new("grading.unavailable"))?;
        let original = load_authorized_native_pdf(
            pool,
            state,
            submission_id,
            user_id,
            source.submission_revision,
            approved.id,
        )
        .await
        .map_err(|_| ServerFnError::new("grading.stale_submission"))?;
        if pdf_metadata.as_ref() != Some(&original.metadata) {
            return Err(ServerFnError::new("grading.stale_submission"));
        }
        // A separate Teacher-scoped transaction COMMITs the durable school
        // request/byte/cost-envelope reservation before the provider call.
        // Do not roll the reservation back with a later model error: an
        // ambiguous timeout could already have incurred provider charges.
        crate::services::ai_grading_pdf_budget::reserve_pdf_budget_for_grading_state(
            state,
            user_id,
            source.school_id,
            submission_id,
            source.submission_revision,
            approved.id,
            original.metadata.byte_size,
        )
        .await
        .map_err(|error| match error {
            crate::services::ai_grading_pdf_budget::PdfBudgetError::Denied => {
                ServerFnError::new("grading.pdf_budget_exhausted")
            }
            crate::services::ai_grading_pdf_budget::PdfBudgetError::Unavailable => {
                ServerFnError::new("grading.suggestion_unavailable")
            }
        })?;
        llm.suggest_native_pdf_grade_for_school(&original, &approved.criteria, approved.grade_scale)
            .await
            .map_err(|_| ServerFnError::new("grading.unavailable"))?
    } else {
        llm.suggest_assignment_grade_for_school(
            source.school_id,
            source.submission_revision,
            &source.assignment_instructions,
            &source.submission_text,
            &approved.criteria,
            approved.grade_scale,
        )
        .await
        .map_err(|_| ServerFnError::new("grading.unavailable"))?
    };
    suggestion
        .validate_against_rubric(source.submission_revision, &approved.criteria)
        .map_err(|_| ServerFnError::new("grading.invalid_suggestion"))?;

    // The model request may take seconds. Re-read both authoritative
    // resources and fail closed if the teacher, answer or rubric changed.
    let latest = if let Some(expected_pdf) = pdf_metadata.as_ref() {
        let current_pdf = find_authorized_pdf_metadata(
            pool,
            submission_id,
            user_id,
            source.submission_revision,
            approved.id,
        )
        .await
        .map_err(|_| ServerFnError::new("grading.suggestion_unavailable"))?
        .ok_or_else(|| ServerFnError::new("grading.stale_submission"))?;
        if &current_pdf != expected_pdf
            || !school_pdf_grading_enabled(pool, expected_pdf.school_id)
                .await
                .map_err(|_| ServerFnError::new("grading.suggestion_unavailable"))?
        {
            return Err(ServerFnError::new("grading.stale_submission"));
        }
        AuthorizedGradingSource {
            school_id: current_pdf.school_id,
            submission_revision: current_pdf.submission_revision,
            grade_scale: approved.grade_scale,
            assignment_instructions: current_pdf.assignment_instructions,
            submission_text: current_pdf.submission_text,
        }
    } else {
        find_authorized_grading_source(pool, submission_id, user_id)
            .await
            .map_err(|_| ServerFnError::new("grading.suggestion_unavailable"))?
            .ok_or_else(|| ServerFnError::new("grading.stale_submission"))?
    };
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
        || approved.submission_revision != latest_approval.submission_revision
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
        let approved = current_approved_rubric(&pool, submission_id, user_id)
            .await
            .map_err(|_| ServerFnError::new("grading.suggestion_unavailable"))?;
        let Some(approved) = approved else {
            return Ok(None);
        };
        let text_source = find_authorized_grading_source(&pool, submission_id, user_id)
            .await
            .map_err(|_| ServerFnError::new("grading.suggestion_unavailable"))?;
        let authorized = if let Some(source) = text_source {
            source.school_id == approved.school_id
                && source.grade_scale == approved.grade_scale
                && source.submission_revision == approved.submission_revision
        } else {
            find_authorized_pdf_metadata(
                &pool,
                submission_id,
                user_id,
                approved.submission_revision,
                approved.id,
            )
            .await
            .map_err(|_| ServerFnError::new("grading.suggestion_unavailable"))?
            .is_some_and(|pdf| pdf.school_id == approved.school_id)
        };
        if !authorized {
            return Ok(None);
        }
        current_stored_suggestion(
            &pool,
            submission_id,
            approved.submission_revision,
            approved.id,
        )
        .await
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
            "find_authorized_pdf_metadata",
            "load_authorized_native_pdf",
            "suggest_native_pdf_grade_for_school",
            "current_approved_rubric",
            "validate_authoritative_rubric",
            "validate_against_rubric",
            "latest_approval",
            "if &current_pdf != expected_pdf",
            "school_pdf_grading_enabled",
            "grading.pdf_school_disabled",
            "reserve_pdf_budget_before_egress",
            "ON CONFLICT (submission_id, submission_revision, rubric_approval_id) DO NOTHING",
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
