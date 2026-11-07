//! Teacher-approved grading rubric endpoints for individual student assignments.
//!
//! An AI-generated personalization rubric is never itself authoritative.
//! Only a school-scoped, assigned teacher can create an immutable approval.
//! No grade or LLM suggestion is created by these endpoints.

use crate::ai_grading_contract::ExpectedCriterion;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "server")]
use crate::ai_grading_contract::validate_authoritative_rubric;
#[cfg(feature = "server")]
use crate::rls_context::AuthorizedPool;
#[cfg(feature = "server")]
use sqlx::Row;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TeacherApprovedGradingRubric {
    pub id: Uuid,
    pub custom_assignment_id: Uuid,
    pub grade_scale: i16,
    pub criteria: Vec<ExpectedCriterion>,
    pub rubric_fingerprint: String,
    pub approved_at: String,
}

#[cfg(feature = "server")]
fn parse_approval(
    row: sqlx::postgres::PgRow,
) -> Result<TeacherApprovedGradingRubric, ServerFnError> {
    let criteria = serde_json::from_value(row.get("criteria"))
        .map_err(|_| ServerFnError::new("grading.rubric_corrupt"))?;
    Ok(TeacherApprovedGradingRubric {
        id: row.get("id"),
        custom_assignment_id: row.get("custom_assignment_id"),
        grade_scale: row.get("grade_scale"),
        criteria,
        rubric_fingerprint: row.get("rubric_fingerprint"),
        approved_at: row.get("approved_at"),
    })
}

#[cfg(feature = "server")]
fn teacher_id(user: &crate::domain::UserInfo) -> Result<Uuid, ServerFnError> {
    if user.role != "Teacher" {
        return Err(ServerFnError::new("grading.forbidden"));
    }
    Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("grading.forbidden"))
}

/// An explicitly authorized teacher approves the complete grading scale and
/// criterion weights for one personalized assignment. The DB revalidates
/// teacher assignment, enrollment and school, independently of this query.
/// Existing active approvals cannot be overwritten or silently replaced.
#[post("/api/ai-grading/rubrics/approve")]
pub async fn approve_teacher_grading_rubric(
    custom_assignment_id: Uuid,
    grade_scale: i16,
    rubric: Vec<ExpectedCriterion>,
) -> Result<TeacherApprovedGradingRubric, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (user, pool) =
            crate::server_functions::rls_helpers::extract_user_with_full_rls().await?;
        let teacher_user_id = teacher_id(&user)?;
        if custom_assignment_id.is_nil() {
            return Err(ServerFnError::new("grading.invalid_assignment"));
        }
        validate_authoritative_rubric(&rubric, grade_scale)
            .map_err(|_| ServerFnError::new("grading.invalid_rubric"))?;
        let criteria = serde_json::to_value(&rubric)
            .map_err(|_| ServerFnError::new("grading.invalid_rubric"))?;
        let row = insert_teacher_grading_rubric(
            &pool,
            custom_assignment_id,
            teacher_user_id,
            grade_scale,
            criteria,
        )
        .await
        .map_err(|error| {
            tracing::warn!(%error, "Teacher rubric approval was rejected");
            ServerFnError::new("grading.rubric_unavailable")
        })?
        .ok_or_else(|| ServerFnError::new("grading.rubric_conflict"))?;
        parse_approval(row)
    }

    #[cfg(not(feature = "server"))]
    {
        let _ = (custom_assignment_id, grade_scale, rubric);
        Err(ServerFnError::new("server only"))
    }
}

#[cfg(feature = "server")]
async fn insert_teacher_grading_rubric(
    pool: &AuthorizedPool,
    custom_assignment_id: Uuid,
    teacher_user_id: Uuid,
    grade_scale: i16,
    criteria: serde_json::Value,
) -> Result<Option<sqlx::postgres::PgRow>, sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO public.ai_grading_rubric_approvals
            (custom_assignment_id, school_id, teacher_id, grade_scale, criteria,
             rubric_fingerprint)
        SELECT ca.id, cs.school_id, t.id, $3, $4,
               repeat('0', 64)
        FROM public.custom_assignments ca
        JOIN public.assignments a ON a.id = ca.assignment_id
        JOIN public.class_sections cs ON cs.id = a.class_section_id
        JOIN public.teachers t ON t.id = a.teacher_id AND t.school_id = cs.school_id
        JOIN public.users u ON u.id = t.user_id AND u.school_id = cs.school_id
        JOIN public.roles r ON r.id = u.role_id
        JOIN public.teaching_assignments ta
          ON ta.teacher_id = t.id AND ta.class_section_id = a.class_section_id
        JOIN public.students st ON st.id = ca.student_id AND st.school_id = cs.school_id
        JOIN public.users su ON su.id = st.user_id
          AND su.school_id = cs.school_id AND su.is_active
        JOIN public.enrollments e
          ON e.student_id = st.id AND e.class_section_id = a.class_section_id
        WHERE ca.id = $1 AND u.id = $2 AND u.is_active
          AND r.name::text = 'Teacher'
          AND a.status = 'Published'::assignment_status
          AND NOT EXISTS (
              SELECT 1 FROM public.submissions s
              WHERE s.custom_assignment_id = ca.id
                AND (
                    s.grade IS NOT NULL OR s.graded_by IS NOT NULL
                    OR s.grade_scale <> $3
                )
          )
          AND ca.graded_at IS NULL
        ON CONFLICT DO NOTHING
        RETURNING id, custom_assignment_id, grade_scale, criteria,
                  rubric_fingerprint, approved_at::text AS approved_at
        "#,
    )
    .bind(custom_assignment_id)
    .bind(teacher_user_id)
    .bind(grade_scale)
    .bind(criteria)
    .fetch_optional(pool)
    .await
}

/// Retrieve only the teacher's active, previously approved rubric.
/// RLS and matching teacher ownership fail closed for unrelated assignments.
#[get("/api/ai-grading/rubrics/current?custom_assignment_id")]
pub async fn get_teacher_grading_rubric(
    custom_assignment_id: Uuid,
) -> Result<Option<TeacherApprovedGradingRubric>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (user, pool) =
            crate::server_functions::rls_helpers::extract_user_with_full_rls().await?;
        teacher_id(&user)?;
        let row = sqlx::query(
            r#"
            SELECT id, custom_assignment_id, grade_scale, criteria,
                   rubric_fingerprint, approved_at::text AS approved_at
            FROM public.ai_grading_rubric_approvals
            WHERE custom_assignment_id = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(custom_assignment_id)
        .fetch_optional(&*pool)
        .await
        .map_err(|_| ServerFnError::new("grading.rubric_unavailable"))?;
        row.map(parse_approval).transpose()
    }

    #[cfg(not(feature = "server"))]
    {
        let _ = custom_assignment_id;
        Err(ServerFnError::new("server only"))
    }
}

/// Explicitly revoke an approved rubric. Model evidence and approval history
/// remain immutable; a new explicit approval is required to replace it.
#[post("/api/ai-grading/rubrics/revoke")]
pub async fn revoke_teacher_grading_rubric(approval_id: Uuid) -> Result<bool, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (user, pool) =
            crate::server_functions::rls_helpers::extract_user_with_full_rls().await?;
        let teacher_user_id = teacher_id(&user)?;
        let result = sqlx::query(
            r#"
            UPDATE public.ai_grading_rubric_approvals approval
            SET revoked_at = NOW()
            FROM public.teachers t
            WHERE approval.id = $1
              AND approval.revoked_at IS NULL
              AND approval.teacher_id = t.id
              AND t.user_id = $2
              AND t.school_id = approval.school_id
            "#,
        )
        .bind(approval_id)
        .bind(teacher_user_id)
        .execute(&*pool)
        .await
        .map_err(|_| ServerFnError::new("grading.rubric_unavailable"))?;
        Ok(result.rows_affected() == 1)
    }

    #[cfg(not(feature = "server"))]
    {
        let _ = approval_id;
        Err(ServerFnError::new("server only"))
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn teacher_rubric_requires_explicit_valid_points() {
        let input = [
            ExpectedCriterion {
                criterion_id: "logic".to_string(),
                possible_points: 15.0,
            },
            ExpectedCriterion {
                criterion_id: "accuracy".to_string(),
                possible_points: 5.0,
            },
        ];
        assert!(validate_authoritative_rubric(&input, 20).is_ok());
        assert!(validate_authoritative_rubric(&input, 100).is_err());
    }

    #[test]
    fn rubric_writes_never_publish_a_submission_grade() {
        let source = include_str!("ai_grading_rubric_functions.rs");
        let implementation = source
            .split("#[cfg(all(test, feature = \"server\"))]")
            .next()
            .unwrap_or(source);
        assert!(!implementation.contains("UPDATE public.submissions"));
        assert!(!implementation.contains("INSERT INTO public.ai_grading_suggestions"));
        assert!(!implementation.contains("UPDATE public.custom_assignments SET status"));
        assert!(implementation.contains("NOT EXISTS ("));
        assert!(implementation.contains("ca.graded_at IS NULL"));
    }
}
