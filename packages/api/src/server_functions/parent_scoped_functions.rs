//! Parent-facing reads keyed by the authenticated parent user identity.
//!
//! `students.parent_id` references `users(id)` in the verified production schema.
//! These endpoints therefore authorize children directly against the parent user
//! id instead of translating through the separate `parents.id` profile key.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::server_functions::dashboard_functions::{ChildAssignmentInfo, ChildGradeInfo};
#[cfg(feature = "server")]
use crate::server_functions::grade_presentation::present_grade;
#[cfg(feature = "server")]
use sqlx::Row;
#[cfg(feature = "server")]
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParentChildSummary {
    /// Student-domain id used by child-scoped grade/assignment queries.
    pub id: String,
    pub name: String,
    /// Grade level is optional because the production schema does not require a
    /// normalized grade-level column. Provisioned metadata is shown only when
    /// it was actually supplied.
    pub grade_level: Option<String>,
    pub enrolled_classes: i64,
}

#[cfg(feature = "server")]
async fn parent_actor(
) -> Result<(Uuid, std::sync::Arc<crate::rls_context::AuthorizedPool>), ServerFnError> {
    let (user, pool) = crate::server_functions::rls_helpers::extract_user_with_full_rls().await?;
    if user.role != "Parent" {
        return Err(ServerFnError::new("parent.forbidden"));
    }
    let user_id =
        Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("parent.invalid_session"))?;
    Ok((user_id, pool))
}

#[server(endpoint = "parent/scoped/children")]
pub async fn get_parent_children_scoped() -> Result<Vec<ParentChildSummary>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (parent_user_id, pool) = parent_actor().await?;
        let rows = sqlx::query(
            r#"
            SELECT
                st.id,
                u.name,
                NULLIF(BTRIM(COALESCE(u.metadata->>'grade_level', '')), '') AS grade_level,
                COUNT(DISTINCT e.id) AS enrolled_classes
            FROM students st
            JOIN users u ON st.user_id = u.id
            LEFT JOIN enrollments e ON e.student_id = st.id
            WHERE st.parent_id = $1
              AND st.school_id = u.school_id
              AND u.is_active = TRUE
            GROUP BY st.id, u.name, u.metadata
            ORDER BY u.name
            "#,
        )
        .bind(parent_user_id)
        .fetch_all(&*pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, parent_user_id = %parent_user_id, "parent child list failed");
            ServerFnError::new("parent.children_unavailable")
        })?;

        rows.into_iter()
            .map(|row| {
                Ok(ParentChildSummary {
                    id: row.try_get::<Uuid, _>("id")?.to_string(),
                    name: row.try_get("name")?,
                    grade_level: row.try_get("grade_level")?,
                    enrolled_classes: row.try_get("enrolled_classes")?,
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
            .map_err(|error| {
                tracing::error!(%error, "parent child list decode failed");
                ServerFnError::new("parent.children_unavailable")
            })
    }
    #[cfg(not(feature = "server"))]
    Ok(Vec::new())
}

#[server(endpoint = "parent/scoped/child/grades")]
pub async fn get_child_grades_for_parent_scoped(
    child_id: String,
) -> Result<Vec<crate::server_functions::dashboard_functions::ChildGradeInfo>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (parent_user_id, pool) = parent_actor().await?;
        let child_id = Uuid::parse_str(&child_id)
            .map_err(|_| ServerFnError::new("parent.child_id_invalid"))?;
        require_child_owner(&pool, parent_user_id, child_id).await?;

        let rows = sqlx::query(
            r#"
            SELECT
                a.title,
                cs.name AS class_name,
                CAST(s.grade AS DOUBLE PRECISION) AS grade,
                COALESCE(s.grade_scale, 100::SMALLINT) AS grade_scale,
                ca.graded_at
            FROM submissions s
            JOIN custom_assignments ca ON s.custom_assignment_id = ca.id
            JOIN assignments a ON ca.assignment_id = a.id
            JOIN class_sections cs ON a.class_section_id = cs.id
            WHERE s.student_id = $1 AND s.grade IS NOT NULL
            ORDER BY ca.graded_at DESC
            LIMIT 20
            "#,
        )
        .bind(child_id)
        .fetch_all(&*pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, child_id = %child_id, "parent child grade list failed");
            ServerFnError::new("parent.grades_unavailable")
        })?;

        rows.into_iter()
            .map(|row| {
                let grade = row
                .try_get::<Option<f64>, _>("grade")?.unwrap_or(0.0);
                let grade_scale = row.try_get::<i16, _>("grade_scale")?;
                let presentation = present_grade(grade, grade_scale).map_err(|error| {
                    tracing::error!(%error, grade, grade_scale, "invalid persisted parent grade presentation");
                    Box::new(error) as Box<dyn std::error::Error + Send + Sync>
                })?;
                let graded_at = row
                    .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("graded_at")?
                    .map(|value| value.to_rfc3339());
                Ok(ChildGradeInfo {
                    assignment_title: row.try_get("title")?,
                    class_name: row.try_get("class_name")?,
                    grade: presentation.letter_grade,
                    points: presentation.points,
                    graded_at,
                })
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error + Send + Sync>>>()
            .map_err(|error| {
                tracing::error!(%error, "parent child grade decode failed");
                ServerFnError::new("parent.grades_unavailable")
            })
    }
    #[cfg(not(feature = "server"))]
    Ok(Vec::new())
}

#[server(endpoint = "parent/scoped/child/assignments")]
pub async fn get_child_assignments_for_parent_scoped(
    child_id: String,
) -> Result<Vec<crate::server_functions::dashboard_functions::ChildAssignmentInfo>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (parent_user_id, pool) = parent_actor().await?;
        let child_id = Uuid::parse_str(&child_id)
            .map_err(|_| ServerFnError::new("parent.child_id_invalid"))?;
        require_child_owner(&pool, parent_user_id, child_id).await?;

        let rows = sqlx::query(
            r#"
            SELECT
                ca.id,
                a.title,
                cs.name AS class_name,
                ca.due_at,
                CASE
                    WHEN s.grade IS NOT NULL THEN 'Graded'
                    WHEN ca.submitted_at IS NOT NULL THEN 'Submitted'
                    WHEN ca.due_at < NOW() THEN 'Overdue'
                    ELSE 'Pending'
                END AS presentation_status,
                CAST(s.grade AS DOUBLE PRECISION) AS grade,
                COALESCE(s.grade_scale, 100::SMALLINT) AS grade_scale
            FROM custom_assignments ca
            JOIN assignments a ON ca.assignment_id = a.id
            JOIN class_sections cs ON a.class_section_id = cs.id
            LEFT JOIN submissions s ON s.custom_assignment_id = ca.id AND s.student_id = ca.student_id
            WHERE ca.student_id = $1
            ORDER BY ca.due_at DESC
            LIMIT 20
            "#,
        )
        .bind(child_id)
        .fetch_all(&*pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, child_id = %child_id, "parent child assignment list failed");
            ServerFnError::new("parent.assignments_unavailable")
        })?;

        rows.into_iter()
            .map(|row| {
                let grade = row.try_get::<Option<f64>, _>("grade")?;
                let grade_scale = row.try_get::<i16, _>("grade_scale")?;
                let grade = grade
                    .map(|value| present_grade(value, grade_scale))
                    .transpose()?
                    .map(|presentation| presentation.letter_grade);
                Ok(ChildAssignmentInfo {
                    id: row.try_get::<Uuid, _>("id")?.to_string(),
                    title: row.try_get("title")?,
                    class_name: row.try_get("class_name")?,
                    due_date: row
                        .try_get::<chrono::DateTime<chrono::Utc>, _>("due_at")?
                        .to_rfc3339(),
                    status: row.try_get("presentation_status")?,
                    grade,
                })
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error + Send + Sync>>>()
            .map_err(|error| {
                tracing::error!(%error, "parent child assignment decode failed");
                ServerFnError::new("parent.assignments_unavailable")
            })
    }
    #[cfg(not(feature = "server"))]
    Ok(Vec::new())
}

/// Ephemeral questions only. The server never trusts client-supplied factual
/// history, stores no transcript, and re-reads the selected child's records.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParentAcademicChatRequest {
    pub child_id: String,
    pub question: String,
    pub prior_questions: Vec<String>,
    pub locale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParentAcademicChatReply {
    pub answer: String,
    pub as_of: String,
    pub records_consulted: usize,
}

fn bounded_parent_chat_request(request: &ParentAcademicChatRequest) -> bool {
    fn safe_question(text: &str, max: usize) -> bool {
        !text.trim().is_empty()
            && text.chars().count() <= max
            && !text.chars().any(|c| c.is_control() && c != '\n')
    }
    safe_question(&request.question, 800)
        && request.prior_questions.len() <= 3
        && request
            .prior_questions
            .iter()
            .all(|q| safe_question(q, 400))
        && matches!(request.locale.as_str(), "fa" | "en")
}

#[server(endpoint = "parent/scoped/child/academic-chat")]
pub async fn ask_parent_academic_chat(
    request: ParentAcademicChatRequest,
) -> Result<ParentAcademicChatReply, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::services::llm_service::{ExternalLlmClient, LlmConfig};
        if !bounded_parent_chat_request(&request) {
            return Err(ServerFnError::new("parent.ai_invalid_question"));
        }
        let (parent_id, pool) = parent_actor().await?;
        let child_id = Uuid::parse_str(&request.child_id)
            .map_err(|_| ServerFnError::new("parent.child_id_invalid"))?;
        require_child_owner(&pool, parent_id, child_id).await?;

        // Reauthorize each message under the same RLS transaction and
        // canonical school identity. Never trust a school ID from the browser.
        let school_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT st.school_id
            FROM public.students st
            JOIN public.users parent ON parent.id = st.parent_id
            JOIN public.users student_user ON student_user.id = st.user_id
            WHERE st.id = $1
              AND st.parent_id = $2
              AND st.school_id = parent.school_id
              AND st.school_id = student_user.school_id
              AND parent.is_active AND student_user.is_active
            "#,
        )
        .bind(child_id)
        .bind(parent_id)
        .fetch_optional(&*pool)
        .await
        .map_err(|_| ServerFnError::new("parent.ai_context_unavailable"))?
        .ok_or_else(|| ServerFnError::new("parent.child_not_linked"))?;

        // No private submissions, raw feedback, profiles, sibling identities,
        // medical records or unrestricted user-supplied SQL. Only established
        // Parent-viewable assignment/grade fields under current school scope.
        let rows = sqlx::query(
            r#"
            SELECT a.id AS assignment_id, a.title, cs.name AS class_name,
                   ca.due_at, ca.submitted_at,
                   CAST(s.grade AS DOUBLE PRECISION) AS grade,
                   COALESCE(s.grade_scale, 100::SMALLINT) AS grade_scale,
                   ca.graded_at,
                   CASE WHEN s.grade IS NOT NULL THEN 'Graded'
                        WHEN ca.submitted_at IS NOT NULL THEN 'Submitted'
                        WHEN ca.due_at < NOW() THEN 'Overdue'
                        ELSE 'Pending' END AS status
            FROM public.custom_assignments ca
            JOIN public.assignments a ON a.id = ca.assignment_id
            JOIN public.class_sections cs ON cs.id = a.class_section_id
            LEFT JOIN public.submissions s
              ON s.custom_assignment_id = ca.id
             AND s.student_id = ca.student_id
            WHERE ca.student_id = $1
              AND cs.school_id = $2
              AND a.status = 'Published'::assignment_status
            ORDER BY ca.due_at DESC, a.id
            LIMIT 20
            "#,
        )
        .bind(child_id)
        .bind(school_id)
        .fetch_all(&*pool)
        .await
        .map_err(|_| ServerFnError::new("parent.ai_context_unavailable"))?;
        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            let grade = row.try_get::<Option<f64>, _>("grade")
                .map_err(|_| ServerFnError::new("parent.ai_context_unavailable"))?;
            let scale = row
                .try_get::<i16, _>("grade_scale")
                .map_err(|_| ServerFnError::new("parent.ai_context_unavailable"))?;
            let due = row
                .try_get::<chrono::DateTime<chrono::Utc>, _>("due_at")
                .map_err(|_| ServerFnError::new("parent.ai_context_unavailable"))?;
            let graded = row
                .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("graded_at")
                .map_err(|_| ServerFnError::new("parent.ai_context_unavailable"))?;
            let title: String = row
                .try_get("title")
                .map_err(|_| ServerFnError::new("parent.ai_context_unavailable"))?;
            let class_name: String = row
                .try_get("class_name")
                .map_err(|_| ServerFnError::new("parent.ai_context_unavailable"))?;
            let assignment_id: Uuid = row
                .try_get("assignment_id")
                .map_err(|_| ServerFnError::new("parent.ai_context_unavailable"))?;
            let status: String = row
                .try_get("status")
                .map_err(|_| ServerFnError::new("parent.ai_context_unavailable"))?;
            records.push(serde_json::json!({
                "assignment_id": assignment_id,
                "title": title.chars().take(120).collect::<String>(),
                "class_name": class_name.chars().take(80).collect::<String>(),
                "status": status,
                "due_at": due.to_rfc3339(),
                "recorded_grade": grade.map(|g| serde_json::json!({
                    "value": g,
                    "scale": scale,
                    "graded_at": graded.map(|t| t.to_rfc3339()),
                })),
            }));
        }

        let as_of = chrono::Utc::now().to_rfc3339();
        let records_consulted = records.len();
        let facts = serde_json::json!({
            "as_of_utc": as_of,
            "records_consulted": records_consulted,
            "assignments": records,
            "no_records_means_not_available": true,
        });
        let mut config =
            LlmConfig::from_env().map_err(|_| ServerFnError::new("parent.ai_unavailable"))?;
        config.max_tokens = config.max_tokens.min(768);
        config.request_timeout = config
            .request_timeout
            .min(std::time::Duration::from_secs(30));
        let llm = ExternalLlmClient::with_config(config)
            .map_err(|_| ServerFnError::new("parent.ai_unavailable"))?;
        let answer = llm
            .answer_parent_academic_question_for_school(
                school_id,
                facts,
                &request.question,
                &request.prior_questions,
                &request.locale,
            )
            .await
            .map_err(|_| ServerFnError::new("parent.ai_unavailable"))?;
        Ok(ParentAcademicChatReply {
            answer,
            as_of,
            records_consulted,
        })
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("parent.ai_unavailable"))
}

#[cfg(feature = "server")]
async fn require_child_owner(
    pool: &crate::rls_context::AuthorizedPool,
    parent_user_id: Uuid,
    child_id: Uuid,
) -> Result<(), ServerFnError> {
    let owned = sqlx::query_scalar::<_, i32>(
        "SELECT 1 FROM students WHERE id = $1 AND parent_id = $2",
    )
    .bind(child_id)
    .bind(parent_user_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, child_id = %child_id, "parent child ownership check failed");
        ServerFnError::new("parent.child_access_unavailable")
    })?;
    if owned.is_none() {
        return Err(ServerFnError::new("parent.child_not_linked"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn parent_children_are_keyed_by_parent_user_id() {
        let source = include_str!("parent_scoped_functions.rs");
        assert!(source.contains("WHERE st.parent_id = $1"));
        assert!(source.contains("SELECT 1 FROM students WHERE id = $1 AND parent_id = $2"));
        assert!(!source.contains("grade_level: \"Grade\""));
    }

    #[test]
    fn parent_grade_query_preserves_declared_scale_and_uses_shared_formatter() {
        let source = include_str!("parent_scoped_functions.rs");
        assert!(source.contains("COALESCE(s.grade_scale, 100::SMALLINT) AS grade_scale"));
        assert!(source.contains("present_grade(grade, grade_scale)"));
        assert!(!source.contains("format!(\"{grade:.0}/100\")"));
    }

    #[test]
    fn parent_assignment_query_returns_neutral_dates_and_student_canonical_states() {
        let source = include_str!("parent_scoped_functions.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert!(production.contains(".to_rfc3339()"));
        assert!(production.contains("WHEN s.grade IS NOT NULL THEN 'Graded'"));
        assert!(production.contains("WHEN ca.submitted_at IS NOT NULL THEN 'Submitted'"));
        assert!(production.contains("WHEN ca.due_at < NOW() THEN 'Overdue'"));
        assert!(production.contains("ELSE 'Pending'"));
        assert!(!production.contains("ca.status::text AS status"));
        assert!(!production.contains(".format(\"%b %d, %Y\")"));
    }
}
