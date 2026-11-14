//! RLS-scoped, revision-bound original PDF for teacher-authorized grading.
//!
//! Student documents never cross this boundary as URLs or extracted text.
//! The Teacher-authorized suggestion path invokes this source for PDF work.
//! Existing text-only grading continues to exclude finalized attachments.

use crate::app_state::AppState;
use crate::repositories::submission_attachment_repository::object_key;
use crate::rls_context::AuthorizedPool;
use crate::services::ai_grading_native_pdf::{NativePdfGradingInput, NativePdfInputError};
use crate::services::submission_attachment_storage;
use sqlx::Row;
use uuid::Uuid;

/// Pilot ceiling matches the opt-in internal native PDF gateway.
pub const MAX_GRADING_PDF_BYTES: i64 = 1_000_000;

/// RLS-filtered metadata; never return a storage key or original bytes to a
/// browser or serialize this internal type to a client response.
#[derive(Clone, PartialEq, Eq)]
pub struct AuthorizedPdfMetadata {
    pub school_id: Uuid,
    pub submission_id: Uuid,
    pub submission_revision: Uuid,
    pub rubric_approval_id: Uuid,
    pub attachment_id: Uuid,
    pub sha256: String,
    pub byte_size: i64,
    pub assignment_instructions: String,
    pub submission_text: String,
}

/// Exact assigned teacher AND currently approved rubric AND one finalized PDF.
/// A submission with a PNG, JPEG or multiple attachments fails closed until
/// separately qualified multimodal policies have been implemented.
pub const AUTHORIZED_NATIVE_PDF_SOURCE: &str = r#"
SELECT cs.school_id, s.id AS submission_id, s.revision AS submission_revision,
       approval.id AS rubric_approval_id, attachment.id AS attachment_id,
       attachment.sha256, attachment.byte_size,
       COALESCE(NULLIF(ca.prompt_ctx #>> '{personalized_assignment,body}', ''), a.body)
           AS assignment_instructions,
       COALESCE(s.content ->> 'text', '') AS submission_text
FROM public.submissions s
JOIN public.custom_assignments ca
  ON ca.id=s.custom_assignment_id AND ca.student_id=s.student_id
JOIN public.assignments a ON a.id=ca.assignment_id
JOIN public.class_sections cs ON cs.id=a.class_section_id
JOIN public.teachers t ON t.id=a.teacher_id AND t.school_id=cs.school_id
JOIN public.users teacher_user
  ON teacher_user.id=t.user_id AND teacher_user.school_id=cs.school_id
 AND teacher_user.is_active=TRUE
JOIN public.roles teacher_role ON teacher_role.id=teacher_user.role_id
JOIN public.teaching_assignments ta
  ON ta.teacher_id=t.id AND ta.class_section_id=a.class_section_id
JOIN public.students st ON st.id=ca.student_id AND st.school_id=cs.school_id
JOIN public.users student_user
  ON student_user.id=st.user_id AND student_user.school_id=cs.school_id
 AND student_user.is_active=TRUE
JOIN public.enrollments e
  ON e.student_id=st.id AND e.class_section_id=a.class_section_id
JOIN public.ai_grading_rubric_approvals approval
  ON approval.custom_assignment_id=ca.id AND approval.teacher_id=t.id
 AND approval.school_id=cs.school_id AND approval.revoked_at IS NULL
 AND approval.grade_scale=s.grade_scale
JOIN public.submission_attachments attachment
  ON attachment.submission_id=s.id
 AND attachment.custom_assignment_id=ca.id
 AND attachment.student_id=ca.student_id
 AND attachment.school_id=cs.school_id
 AND attachment.status='submitted'
 AND attachment.media_type='application/pdf'
 AND attachment.byte_size BETWEEN 1 AND 1000000
WHERE s.id=$1 AND teacher_user.id=$2
  AND s.revision=$3 AND approval.id=$4
  AND teacher_role.name::text='Teacher'
  AND a.status='Published'::assignment_status
  AND ca.status='Submitted'::custom_status
  AND ca.submitted_at IS NOT NULL
  AND s.submitted_at IS NOT NULL
  AND ca.graded_at IS NULL AND s.grade IS NULL AND s.graded_by IS NULL
  AND (SELECT count(*) FROM public.submission_attachments other_attachment
       WHERE other_attachment.custom_assignment_id=ca.id
         AND other_attachment.student_id=ca.student_id
         AND other_attachment.school_id=cs.school_id
         AND other_attachment.status <> 'removed')=1
LIMIT 1
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorizedPdfError {
    NotAuthorizedOrStale,
    StorageUnavailable,
    InvalidOriginal,
}

pub async fn find_authorized_pdf_metadata(
    pool: &AuthorizedPool,
    submission_id: Uuid,
    teacher_user_id: Uuid,
    revision: Uuid,
    approval_id: Uuid,
) -> Result<Option<AuthorizedPdfMetadata>, sqlx::Error> {
    if [submission_id, teacher_user_id, revision, approval_id]
        .iter()
        .any(Uuid::is_nil)
    {
        return Ok(None);
    }
    let row = sqlx::query(AUTHORIZED_NATIVE_PDF_SOURCE)
        .bind(submission_id)
        .bind(teacher_user_id)
        .bind(revision)
        .bind(approval_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|row| AuthorizedPdfMetadata {
        school_id: row.get("school_id"),
        submission_id: row.get("submission_id"),
        submission_revision: row.get("submission_revision"),
        rubric_approval_id: row.get("rubric_approval_id"),
        attachment_id: row.get("attachment_id"),
        sha256: row.get("sha256"),
        byte_size: row.get("byte_size"),
        assignment_instructions: row.get("assignment_instructions"),
        submission_text: row.get("submission_text"),
    }))
}

/// Operator-controlled school consent to send any original PDF to an LLM.
/// This is checked before storage/model egress and again after the model.
/// The table starts empty and Teacher/Student actors cannot mutate it.
pub async fn school_pdf_grading_enabled(
    pool: &AuthorizedPool,
    school_id: Uuid,
) -> Result<bool, sqlx::Error> {
    if school_id.is_nil() {
        return Ok(false);
    }
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM public.ai_grading_pdf_school_optins \
         WHERE school_id=$1 AND enabled IS TRUE)",
    )
    .bind(school_id)
    .fetch_one(pool)
    .await
}

/// Returned bytes stay server-local, with an authenticated school identity.
/// No Debug/Serialize to prevent accidental PDF or key leakage.
pub struct AuthorizedNativePdf {
    pub metadata: AuthorizedPdfMetadata,
    pub document: NativePdfGradingInput,
}

pub async fn load_authorized_native_pdf(
    pool: &AuthorizedPool,
    state: &AppState,
    submission_id: Uuid,
    teacher_user_id: Uuid,
    revision: Uuid,
    approval_id: Uuid,
) -> Result<AuthorizedNativePdf, AuthorizedPdfError> {
    let source =
        find_authorized_pdf_metadata(pool, submission_id, teacher_user_id, revision, approval_id)
            .await
            .map_err(|_| AuthorizedPdfError::NotAuthorizedOrStale)?
            .ok_or(AuthorizedPdfError::NotAuthorizedOrStale)?;

    if !school_pdf_grading_enabled(pool, source.school_id)
        .await
        .map_err(|_| AuthorizedPdfError::NotAuthorizedOrStale)?
    {
        return Err(AuthorizedPdfError::NotAuthorizedOrStale);
    }

    // Storage service owns its private bucket and bounded, server-authorized
    // read. The key is derived solely from RLS-authorized metadata.
    let key = object_key(source.school_id, source.attachment_id);
    let bytes = submission_attachment_storage::read(state, &key)
        .await
        .map_err(|_| AuthorizedPdfError::StorageUnavailable)?;
    if i64::try_from(bytes.len()).ok() != Some(source.byte_size) {
        return Err(AuthorizedPdfError::InvalidOriginal);
    }
    let document = NativePdfGradingInput::from_private_original(
        source.school_id,
        source.submission_id,
        source.submission_revision,
        source.attachment_id,
        "application/pdf",
        &source.sha256,
        bytes,
    )
    .map_err(|_: NativePdfInputError| AuthorizedPdfError::InvalidOriginal)?;

    // Storage I/O is asynchronous: authorization can be revoked, or the
    // student can submit a different revision before the bytes arrive.
    let latest =
        find_authorized_pdf_metadata(pool, submission_id, teacher_user_id, revision, approval_id)
            .await
            .map_err(|_| AuthorizedPdfError::NotAuthorizedOrStale)?
            .ok_or(AuthorizedPdfError::NotAuthorizedOrStale)?;
    if latest != source
        || !school_pdf_grading_enabled(pool, source.school_id)
            .await
            .map_err(|_| AuthorizedPdfError::NotAuthorizedOrStale)?
    {
        return Err(AuthorizedPdfError::NotAuthorizedOrStale);
    }
    Ok(AuthorizedNativePdf {
        metadata: source,
        document,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_requires_teacher_rubric_tenant_revision_and_single_pdf() {
        for clause in [
            "teacher_user.id=$2",
            "teacher_role.name::text='Teacher'",
            "teacher_user.is_active=TRUE",
            "student_user.is_active=TRUE",
            "JOIN public.teaching_assignments",
            "JOIN public.enrollments",
            "approval.teacher_id=t.id",
            "approval.revoked_at IS NULL",
            "s.revision=$3 AND approval.id=$4",
            "approval.school_id=cs.school_id",
            "attachment.school_id=cs.school_id",
            "attachment.status='submitted'",
            "attachment.media_type='application/pdf'",
            "attachment.byte_size BETWEEN 1 AND 1000000",
            "other_attachment.status <> 'removed')=1",
            "ca.graded_at IS NULL AND s.grade IS NULL",
        ] {
            assert!(
                AUTHORIZED_NATIVE_PDF_SOURCE.contains(clause),
                "missing clause: {clause}"
            );
        }
        assert!(MAX_GRADING_PDF_BYTES <= 1_000_000);
    }
}
