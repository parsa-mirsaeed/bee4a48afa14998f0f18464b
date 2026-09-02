use crate::repositories::{BaseRepository, Repository, RepositoryResult};
use crate::rls_context::AuthorizedPool;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct KnowledgeAuditLog {
    pub id: Uuid,
    pub actor_id: Option<Uuid>,
    pub actor_name: Option<String>,
    pub actor_role: String,
    pub action: String,
    pub target_type: String,
    pub target_id: Uuid,
    pub target_name: Option<String>,
    pub school_id: Option<Uuid>,
    pub school_name: Option<String>,
    pub details: Value,
    pub request_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct KnowledgeAuditRepository {
    base: BaseRepository,
}

impl KnowledgeAuditRepository {
    pub fn new<T>(pool: T) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    pub async fn list_recent(&self, limit: i64) -> RepositoryResult<Vec<KnowledgeAuditLog>> {
        let rows = sqlx::query(
            r#"
            SELECT
                log.id,
                log.actor_id,
                actor.name AS actor_name,
                log.actor_role,
                log.action,
                log.target_type,
                log.target_id,
                asset.title AS target_name,
                log.school_id,
                school.name AS school_name,
                log.details_json,
                log.request_id,
                log.created_at
            FROM knowledge_audit_logs AS log
            LEFT JOIN users AS actor ON actor.id = log.actor_id
            LEFT JOIN schools AS school ON school.id = log.school_id
            LEFT JOIN knowledge_assets AS asset ON asset.id = log.target_id
            ORDER BY log.created_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit.clamp(1, 500))
        .fetch_all(&*self.base.pool())
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(KnowledgeAuditLog {
                    id: row.try_get("id")?,
                    actor_id: row.try_get("actor_id")?,
                    actor_name: row.try_get("actor_name")?,
                    actor_role: row.try_get("actor_role")?,
                    action: row.try_get("action")?,
                    target_type: row.try_get("target_type")?,
                    target_id: row.try_get("target_id")?,
                    target_name: row.try_get("target_name")?,
                    school_id: row.try_get("school_id")?,
                    school_name: row.try_get("school_name")?,
                    details: row.try_get("details_json")?,
                    request_id: row.try_get("request_id")?,
                    created_at: row.try_get("created_at")?,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn admin_audit_query_enriches_readable_context_without_dropping_ids() {
        let source = include_str!("knowledge_audit_repository.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert!(production.contains("actor.name AS actor_name"));
        assert!(production.contains("school.name AS school_name"));
        assert!(production.contains("asset.title AS target_name"));
        assert!(production.contains("log.target_id"));
        assert!(production.contains("log.details_json"));
    }
}
