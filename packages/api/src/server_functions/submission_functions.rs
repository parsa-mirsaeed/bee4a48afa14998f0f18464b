//! Authorized submission server functions.

use dioxus::fullstack::extract;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::app_state::extract_server_state;
#[cfg(feature = "server")]
use crate::domain::UserInfo;
#[cfg(feature = "server")]
use axum::Extension;
#[cfg(feature = "server")]
use chrono::Utc;
#[cfg(feature = "server")]
use sqlx::Row;
#[cfg(feature = "server")]
use uuid::Uuid;

const MAX_SUBMISSION_CONTENT_BYTES: usize = 100_000;

#[cfg(feature = "server")]
fn format_grade_points(grade: f64, grade_scale: i16) -> String {
    format!("{grade:.0}/{grade_scale}")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubmissionResponse {
    pub id: String,
    pub status: String,
    pub submitted_at: Option<String>,
    pub message: String,
}

#[server(endpoint = "submissions/submit")]
pub async fn submit_student_assignment(
    assignment_id: String,
    content: String,
) -> Result<SubmissionResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        submit_work(assignment_id, content, None, None, None).await
    }
    #[cfg(not(feature = "server"))]
    {
        unreachable!()
    }
}

#[post("/api/submissions/finalize")]
pub async fn finalize_student_submission(
    assignment_id: String,
    content: String,
    request_id: uuid::Uuid,
    expected_revision: Option<uuid::Uuid>,
    attachment_ids: Vec<uuid::Uuid>,
) -> Result<SubmissionResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        submit_work(
            assignment_id,
            content,
            Some(request_id),
            expected_revision,
            Some(attachment_ids),
        )
        .await
    }
    #[cfg(not(feature = "server"))]
    {
        unreachable!()
    }
}

#[cfg(feature = "server")]
async fn submit_work(
    assignment_id: String,
    content: String,
    request_id: Option<Uuid>,
    expected_revision: Option<Uuid>,
    expected_attachments: Option<Vec<Uuid>>,
) -> Result<SubmissionResponse, ServerFnError> {
    let Extension(user): Extension<UserInfo> = extract()
        .await
        .map_err(|_| ServerFnError::new("Unauthorized"))?;
    require_student(&user)?;
    if content.len() > MAX_SUBMISSION_CONTENT_BYTES {
        return Err(ServerFnError::new("Invalid submission content"));
    }

    let state = extract_server_state().map_err(map_state_error)?;
    let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Unauthorized"))?;
    let custom_assignment_id =
        Uuid::parse_str(&assignment_id).map_err(|_| ServerFnError::new("Invalid assignment ID"))?;

    let authorized = sqlx::query(
        r#"
            SELECT ca.id, ca.student_id
            FROM custom_assignments ca
            JOIN assignments a ON a.id = ca.assignment_id
            JOIN class_sections cs ON cs.id = a.class_section_id
            JOIN students student ON student.id = ca.student_id
            JOIN users student_user ON student_user.id = student.user_id
            JOIN roles student_role ON student_role.id = student_user.role_id
            JOIN enrollments enrollment
              ON enrollment.student_id = student.id
             AND enrollment.class_section_id = a.class_section_id
            WHERE ca.id = $1
              AND student.user_id = $2
              AND student_user.is_active = TRUE
              AND student_role.name::text = 'Student'
              AND student.school_id = student_user.school_id
              AND cs.school_id = student_user.school_id
              AND a.status = 'Published'::assignment_status
            "#,
    )
    .bind(custom_assignment_id)
    .bind(user_id)
    .fetch_optional(&*state.services.pool)
    .await
    .map_err(map_database_error)?
    .ok_or_else(|| ServerFnError::new("Assignment not found"))?;

    let student_id: Uuid = authorized.get("student_id");
    let now = Utc::now();
    let content_json = serde_json::json!({ "text": content });
    let mut transaction = state
        .services
        .pool
        .begin()
        .await
        .map_err(map_database_error)?;

    // Lock the authoritative custom-assignment row before editing a
    // submission. This makes the graded state terminal for students even
    // when a stale browser page still offers an edit action.
    let assignment_state = sqlx::query(
        "SELECT graded_at FROM custom_assignments WHERE id = $1 AND student_id = $2 FOR UPDATE",
    )
    .bind(custom_assignment_id)
    .bind(student_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(map_database_error)?
    .ok_or_else(|| ServerFnError::new("Assignment not found"))?;
    let graded_at: Option<chrono::DateTime<chrono::Utc>> = assignment_state.get("graded_at");
    if graded_at.is_some() {
        return Err(ServerFnError::new(
            "This assignment has already been graded",
        ));
    }

    let existing_submission = sqlx::query(
            "SELECT id, revision, last_submit_request, last_submit_fingerprint, submitted_at FROM submissions WHERE custom_assignment_id = $1 AND student_id = $2 FOR UPDATE",
        )
        .bind(custom_assignment_id)
        .bind(student_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_database_error)?;

    let attachments = sqlx::query("SELECT id,status FROM submission_attachments WHERE custom_assignment_id=$1 AND status<>'removed' ORDER BY id FOR UPDATE")
            .bind(custom_assignment_id).fetch_all(&mut *transaction).await.map_err(map_database_error)?;
    if attachments
        .iter()
        .any(|row| row.get::<String, _>("status") == "pending")
    {
        return Err(ServerFnError::new("attachment_incomplete"));
    }
    let mut actual_ids: Vec<Uuid> = attachments.iter().map(|r| r.get("id")).collect();
    actual_ids.sort_unstable();
    if expected_attachments.is_none() && !actual_ids.is_empty() {
        return Err(ServerFnError::new("attachment_conflict"));
    }
    if let Some(mut expected) = expected_attachments {
        expected.sort_unstable();
        if expected != actual_ids {
            return Err(ServerFnError::new("attachment_conflict"));
        }
    }
    if content_json["text"]
        .as_str()
        .unwrap_or_default()
        .trim()
        .is_empty()
        && actual_ids.is_empty()
    {
        return Err(ServerFnError::new("Invalid submission content"));
    }
    let fingerprint = crate::services::submission_attachment_storage::hash(
        serde_json::to_string(&serde_json::json!([content_json, actual_ids]))
            .map_err(|_| ServerFnError::new("Invalid submission content"))?
            .as_bytes(),
    );
    if let Some(existing) = existing_submission.as_ref() {
        if request_id.is_some()
            && existing.get::<Option<Uuid>, _>("last_submit_request") == request_id
        {
            if existing
                .get::<Option<String>, _>("last_submit_fingerprint")
                .as_deref()
                != Some(fingerprint.as_str())
            {
                return Err(ServerFnError::new("submission_conflict"));
            }
            return Ok(SubmissionResponse {
                id: existing.get::<Uuid, _>("id").to_string(),
                status: "submitted".into(),
                submitted_at: existing
                    .get::<Option<chrono::DateTime<Utc>>, _>("submitted_at")
                    .map(|v| v.to_rfc3339()),
                message: "Assignment submitted successfully".into(),
            });
        }
        if request_id.is_some() && Some(existing.get::<Uuid, _>("revision")) != expected_revision {
            return Err(ServerFnError::new("submission_conflict"));
        }
    } else if expected_revision.is_some() {
        return Err(ServerFnError::new("submission_conflict"));
    }

    let submission_id = if let Some(existing) = existing_submission {
        let existing_id: Uuid = existing.get("id");
        sqlx::query(
                "UPDATE submissions SET content = $1, submitted_at = $2 WHERE id = $3 AND student_id = $4",
            )
            .bind(&content_json)
            .bind(now)
            .bind(existing_id)
            .bind(student_id)
            .execute(&mut *transaction)
            .await
            .map_err(map_database_error)?;
        existing_id
    } else {
        let submission_id = Uuid::new_v4();
        sqlx::query(
            r#"
                INSERT INTO submissions (
                    id, custom_assignment_id, student_id, content, submitted_at
                ) VALUES ($1, $2, $3, $4, $5)
                "#,
        )
        .bind(submission_id)
        .bind(custom_assignment_id)
        .bind(student_id)
        .bind(&content_json)
        .bind(now)
        .execute(&mut *transaction)
        .await
        .map_err(map_database_error)?;
        submission_id
    };

    sqlx::query(
        r#"
            UPDATE custom_assignments
            SET status = 'Submitted'::custom_status, submitted_at = $1
            WHERE id = $2 AND student_id = $3
            "#,
    )
    .bind(now)
    .bind(custom_assignment_id)
    .bind(student_id)
    .execute(&mut *transaction)
    .await
    .map_err(map_database_error)?;

    // PostgreSQL stores microsecond precision. Return its authoritative timestamp
    // so a successful retry has exactly the same response as the first finalize.
    let submitted_at = sqlx::query_scalar::<_, chrono::DateTime<Utc>>("UPDATE submissions SET revision=gen_random_uuid(),last_submit_request=$2,last_submit_fingerprint=$3 WHERE id=$1 RETURNING submitted_at")
            .bind(submission_id).bind(request_id).bind(fingerprint).fetch_one(&mut *transaction).await.map_err(map_database_error)?;
    sqlx::query("UPDATE submission_attachments SET status='submitted',submission_id=$2,finalized_at=NOW() WHERE custom_assignment_id=$1 AND status='ready'")
            .bind(custom_assignment_id).bind(submission_id).execute(&mut *transaction).await.map_err(map_database_error)?;

    transaction.commit().await.map_err(map_database_error)?;

    Ok(SubmissionResponse {
        id: submission_id.to_string(),
        status: "submitted".to_string(),
        submitted_at: Some(submitted_at.to_rfc3339()),
        message: "Assignment submitted successfully".to_string(),
    })
}

#[server(endpoint = "submissions/get_for_assignment")]
pub async fn get_submission_for_assignment(
    assignment_id: String,
) -> Result<Option<StudentSubmission>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized"))?;
        require_student(&user)?;

        let state = extract_server_state().map_err(map_state_error)?;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Unauthorized"))?;
        let custom_assignment_id = Uuid::parse_str(&assignment_id)
            .map_err(|_| ServerFnError::new("Invalid assignment ID"))?;

        let submission = sqlx::query(
            r#"
            SELECT submission.id, submission.revision,
                   submission.content,
                   submission.submitted_at,
                   CAST(submission.grade AS DOUBLE PRECISION) AS grade,
                   COALESCE(submission.grade_scale, 100::SMALLINT) AS grade_scale,
                   submission.feedback
            FROM custom_assignments ca
            JOIN assignments a ON a.id = ca.assignment_id
            JOIN class_sections cs ON cs.id = a.class_section_id
            JOIN students student ON student.id = ca.student_id
            JOIN users student_user ON student_user.id = student.user_id
            JOIN roles student_role ON student_role.id = student_user.role_id
            JOIN enrollments enrollment
              ON enrollment.student_id = student.id
             AND enrollment.class_section_id = a.class_section_id
            LEFT JOIN submissions submission
              ON submission.custom_assignment_id = ca.id
             AND submission.student_id = student.id
            WHERE ca.id = $1
              AND student.user_id = $2
              AND student_user.is_active = TRUE
              AND student_role.name::text = 'Student'
              AND student.school_id = student_user.school_id
              AND cs.school_id = student_user.school_id
              AND a.status = 'Published'::assignment_status
              AND submission.id IS NOT NULL
            "#,
        )
        .bind(custom_assignment_id)
        .bind(user_id)
        .fetch_optional(&*state.services.pool)
        .await
        .map_err(map_database_error)?;

        Ok(submission.map(|row| {
            let content_json: serde_json::Value = row.get("content");
            let submitted_at: Option<chrono::DateTime<chrono::Utc>> = row.get("submitted_at");
            let grade: Option<f64> = row.get("grade");
            let grade_scale: i16 = row.get("grade_scale");
            StudentSubmission {
                id: row.get::<Uuid, _>("id").to_string(),
                revision: row.get("revision"),
                content: content_json
                    .get("text")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                submitted_at: submitted_at.map(|value| value.to_rfc3339()),
                grade: grade.map(|value| format_grade_points(value, grade_scale)),
                feedback: row.get("feedback"),
            }
        }))
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new(
            "This function can only be called on the server",
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StudentSubmission {
    pub id: String,
    pub revision: uuid::Uuid,
    pub content: String,
    pub submitted_at: Option<String>,
    pub grade: Option<String>,
    pub feedback: Option<String>,
}

#[cfg(feature = "server")]
fn require_student(user: &UserInfo) -> Result<(), ServerFnError> {
    if user.role == "Student" {
        Ok(())
    } else {
        Err(ServerFnError::new("Forbidden"))
    }
}

#[cfg(feature = "server")]
fn map_state_error(error: ServerFnError) -> ServerFnError {
    tracing::error!(%error, "Unable to access submission server state");
    ServerFnError::new("Unable to process submission")
}

#[cfg(feature = "server")]
fn map_database_error(error: sqlx::Error) -> ServerFnError {
    tracing::error!(%error, "Submission database operation failed");
    ServerFnError::new("Unable to process submission")
}

#[cfg(all(test, feature = "server"))]
mod grade_scale_tests {
    use super::format_grade_points;

    #[test]
    fn grade_display_preserves_the_declared_scale() {
        assert_eq!(format_grade_points(18.0, 20), "18/20");
        assert_eq!(format_grade_points(90.0, 100), "90/100");
    }

    #[test]
    fn graded_assignments_are_locked_before_student_submission_mutation() {
        let source = include_str!("submission_functions.rs");
        let submit = &source[..source
            .find("pub async fn get_submission_for_assignment")
            .expect("submission reader follows writer")];
        assert!(submit.contains("FOR UPDATE"));
        assert!(submit.contains("This assignment has already been graded"));
    }
}
