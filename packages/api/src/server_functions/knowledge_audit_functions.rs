use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(feature = "server")]
use crate::repositories::{KnowledgeAuditLog, KnowledgeAuditRepository};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeAuditLogDto {
    pub id: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub actor_role: String,
    pub action: String,
    pub target_type: String,
    pub target_id: String,
    pub target_name: Option<String>,
    pub school_id: Option<String>,
    pub school_name: Option<String>,
    pub details: Value,
    pub request_id: Option<String>,
    pub created_at: String,
}

#[cfg(feature = "server")]
impl From<KnowledgeAuditLog> for KnowledgeAuditLogDto {
    fn from(log: KnowledgeAuditLog) -> Self {
        Self {
            id: log.id.to_string(),
            actor_id: log.actor_id.map(|id| id.to_string()),
            actor_name: log.actor_name,
            actor_role: log.actor_role,
            action: log.action,
            target_type: log.target_type,
            target_id: log.target_id.to_string(),
            target_name: log.target_name,
            school_id: log.school_id.map(|id| id.to_string()),
            school_name: log.school_name,
            details: log.details,
            request_id: log.request_id,
            created_at: log.created_at.to_rfc3339(),
        }
    }
}

#[server(endpoint = "admin/knowledge-audit")]
pub async fn list_admin_knowledge_audit(
    limit: i64,
) -> Result<Vec<KnowledgeAuditLogDto>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (user, pool) =
            crate::server_functions::rls_helpers::extract_user_with_full_rls().await?;
        if user.role != "PlatformAdmin" {
            return Err(ServerFnError::new("Forbidden: insufficient role"));
        }

        let logs = KnowledgeAuditRepository::new(pool.clone())
            .list_recent(limit)
            .await
            .map_err(|error| {
                tracing::error!(%error, "platform knowledge audit list failed");
                ServerFnError::new("Unable to load knowledge audit events")
            })?;
        Ok(logs.into_iter().map(Into::into).collect())
    }
    #[cfg(not(feature = "server"))]
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    #[test]
    fn audit_endpoint_preserves_technical_evidence_and_readable_context() {
        let source = include_str!("knowledge_audit_functions.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert!(production.contains("pub actor_name: Option<String>"));
        assert!(production.contains("pub target_name: Option<String>"));
        assert!(production.contains("pub school_name: Option<String>"));
        assert!(production.contains("pub target_id: String"));
        assert!(production.contains("pub details: Value"));
        assert!(production.contains("pub request_id: Option<String>"));
        assert!(production.contains("extract_user_with_full_rls"));
    }
}
