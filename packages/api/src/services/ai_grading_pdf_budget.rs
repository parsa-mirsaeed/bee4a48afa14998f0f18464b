//! Durable admission reservation for native PDF inference.
//!
//! This transaction commits BEFORE model egress and is independent of the
//! teacher's longer-lived request transaction. No refund on ambiguous
//! timeouts: every provider attempt consumes its own capped reservation.
//! Private originals, teacher prompts and API keys never enter this ledger.

use crate::rls_context::{AuthorizedActor, AuthorizedTx};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfBudgetError {
    Denied,
    Unavailable,
}

fn valid_reservation_inputs(
    teacher_user_id: Uuid,
    school_id: Uuid,
    submission_id: Uuid,
    revision: Uuid,
    approval_id: Uuid,
    original_pdf_size: i64,
) -> bool {
    ![teacher_user_id, school_id, submission_id, revision, approval_id]
        .iter()
        .any(Uuid::is_nil)
        && (1..=1_000_000).contains(&original_pdf_size)
}

pub async fn reserve_pdf_budget_before_egress(
    raw: &PgPool,
    teacher_user_id: Uuid,
    school_id: Uuid,
    submission_id: Uuid,
    revision: Uuid,
    approval_id: Uuid,
    original_pdf_size: i64,
) -> Result<(), PdfBudgetError> {
    if !valid_reservation_inputs(
        teacher_user_id,
        school_id,
        submission_id,
        revision,
        approval_id,
        original_pdf_size,
    ) {
        return Err(PdfBudgetError::Denied);
    }

    let actor = AuthorizedActor::new(teacher_user_id, "Teacher", Some(school_id))
        .map_err(|_| PdfBudgetError::Denied)?;
    let transaction = AuthorizedTx::begin(raw, actor)
        .await
        .map_err(|_| PdfBudgetError::Unavailable)?;
    let scoped = transaction.authorized_pool();
    let reserved = transaction
        .scope(
            sqlx::query_scalar::<_, bool>(
                "SELECT public.reserve_ai_grading_pdf_budget($1,$2,$3,$4,$5)",
            )
            .bind(school_id)
            .bind(submission_id)
            .bind(revision)
            .bind(approval_id)
            .bind(original_pdf_size)
            .fetch_one(&scoped),
            Result::is_ok,
        )
        .await
        .map_err(|_| PdfBudgetError::Unavailable)?
        .map_err(|_| PdfBudgetError::Unavailable)?;
    if reserved {
        Ok(())
    } else {
        Err(PdfBudgetError::Denied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reservation_fails_closed_for_identity_and_document_limits() {
        let school = Uuid::new_v4();
        let teacher = Uuid::new_v4();
        let submission = Uuid::new_v4();
        let revision = Uuid::new_v4();
        let approval = Uuid::new_v4();
        assert!(valid_reservation_inputs(
            teacher, school, submission, revision, approval, 100
        ));
        assert!(!valid_reservation_inputs(
            Uuid::nil(), school, submission, revision, approval, 100
        ));
        assert!(!valid_reservation_inputs(
            teacher, school, Uuid::nil(), revision, approval, 100
        ));
        assert!(!valid_reservation_inputs(
            teacher, school, submission, revision, approval, 0
        ));
        assert!(!valid_reservation_inputs(
            teacher, school, submission, revision, approval, 1_000_001
        ));
    }
}
