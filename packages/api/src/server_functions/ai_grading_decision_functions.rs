//! Authorized and auditable teacher decision on an immutable AI suggestion.
//! The non-authoritative model artifact stays unchanged. Accept publishes
//! its exact grade and teacher-facing feedback atomically; reject publishes
//! nothing. No student/parent or model caller can review a suggestion.
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiGradingDecision {
    Accept,
    Reject,
}

#[post("/api/ai-grading/suggestions/review")]
pub async fn review_teacher_ai_grading_suggestion(
    suggestion_id: Uuid,
    decision: AiGradingDecision,
) -> Result<bool, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (user, pool) =
            crate::server_functions::rls_helpers::extract_user_with_full_rls().await?;
        if user.role != "Teacher" || suggestion_id.is_nil() {
            return Err(ServerFnError::new("grading.forbidden"));
        }
        let action = match decision {
            AiGradingDecision::Accept => "accept",
            AiGradingDecision::Reject => "reject",
        };
        return sqlx::query_scalar::<_, bool>("SELECT public.decide_ai_grading_suggestion($1,$2)")
            .bind(suggestion_id)
            .bind(action)
            .fetch_one(&*pool)
            .await
            .map_err(|error| {
                tracing::warn!(%error, "AI suggestion review rejected by database");
                ServerFnError::new("grading.review_unavailable")
            });
    }
    #[cfg(not(feature = "server"))]
    {
        let _ = (suggestion_id, decision);
        Err(ServerFnError::new("server only"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decisions_have_only_bounded_teacher_actions() {
        assert_eq!(
            serde_json::to_string(&AiGradingDecision::Accept).unwrap(),
            "\"accept\""
        );
        assert_eq!(
            serde_json::to_string(&AiGradingDecision::Reject).unwrap(),
            "\"reject\""
        );
        assert!(serde_json::from_str::<AiGradingDecision>("\"automatic\"").is_err());
    }
}
