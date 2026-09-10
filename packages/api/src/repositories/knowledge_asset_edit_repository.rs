//! Lifecycle-aware SchoolManager editing/versioning for governed knowledge assets.

use crate::repositories::{BaseRepository, Repository, RepositoryError, RepositoryResult};
use crate::rls_context::AuthorizedPool;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerEditableKnowledgeAsset {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub subject: Option<String>,
    pub grade: Option<String>,
    pub language: String,
    pub template_type: Option<String>,
    pub tags: Value,
    pub status: String,
    pub asset_revision: i64,
    pub current_source_file_id: Option<Uuid>,
    pub current_source_filename: Option<String>,
    pub current_source_sha256: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateManagerKnowledgeMetadata {
    pub asset_id: Uuid,
    pub expected_revision: i64,
    pub title: String,
    pub description: Option<String>,
    pub subject: Option<String>,
    pub grade: Option<String>,
    pub language: String,
    pub template_type: Option<String>,
    pub tags: Value,
}

#[derive(Debug, Clone)]
pub struct ReplaceManagerKnowledgeSource {
    pub asset_id: Uuid,
    pub expected_revision: i64,
    pub original_file_url: String,
    pub original_filename: String,
    pub mime_type: String,
    pub file_size_bytes: i64,
    pub sha256: String,
    pub page_count: Option<i32>,
    pub is_scanned_pdf: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeMetadataMutation {
    pub asset_revision: i64,
    pub status: String,
    pub vectors_invalidated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSourceMutation {
    pub asset_revision: i64,
    pub status: String,
    pub source_file_id: Uuid,
    pub vectors_invalidated: bool,
}

#[derive(Clone)]
pub struct KnowledgeAssetEditRepository {
    base: BaseRepository,
}

impl KnowledgeAssetEditRepository {
    pub fn new<T>(pool: T) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    pub async fn list_for_manager(
        &self,
        school_id: Uuid,
    ) -> RepositoryResult<Vec<ManagerEditableKnowledgeAsset>> {
        let rows = sqlx::query(
            r#"
            SELECT asset.id,
                   asset.title,
                   asset.description,
                   asset.subject,
                   asset.grade,
                   asset.language,
                   asset.template_type,
                   asset.tags,
                   asset.status::text AS status,
                   asset.asset_revision,
                   asset.current_source_file_id,
                   source.original_filename AS current_source_filename,
                   lower(source.sha256) AS current_source_sha256
            FROM knowledge_assets AS asset
            LEFT JOIN knowledge_source_files AS source
              ON source.id = asset.current_source_file_id
             AND source.asset_id = asset.id
            WHERE asset.school_id = $1
            ORDER BY asset.updated_at DESC, asset.created_at DESC
            LIMIT 500
            "#,
        )
        .bind(school_id)
        .fetch_all(&*self.base.pool())
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(ManagerEditableKnowledgeAsset {
                    id: row.try_get("id")?,
                    title: row.try_get("title")?,
                    description: row.try_get("description")?,
                    subject: row.try_get("subject")?,
                    grade: row.try_get("grade")?,
                    language: row.try_get("language")?,
                    template_type: row.try_get("template_type")?,
                    tags: row.try_get("tags")?,
                    status: row.try_get("status")?,
                    asset_revision: row.try_get("asset_revision")?,
                    current_source_file_id: row.try_get("current_source_file_id")?,
                    current_source_filename: row.try_get("current_source_filename")?,
                    current_source_sha256: row.try_get("current_source_sha256")?,
                })
            })
            .collect()
    }

    pub async fn update_metadata(
        &self,
        request: UpdateManagerKnowledgeMetadata,
    ) -> RepositoryResult<KnowledgeMetadataMutation> {
        let row = sqlx::query(
            r#"
            SELECT asset_revision, status, vectors_invalidated
            FROM manager_update_knowledge_asset_metadata(
                $1, $2, $3, $4, $5, $6, $7, $8, $9
            )
            "#,
        )
        .bind(request.asset_id)
        .bind(request.expected_revision)
        .bind(request.title)
        .bind(request.description)
        .bind(request.subject)
        .bind(request.grade)
        .bind(request.language)
        .bind(request.template_type)
        .bind(request.tags)
        .fetch_one(&*self.base.pool())
        .await
        .map_err(map_governed_edit_error)?;

        Ok(KnowledgeMetadataMutation {
            asset_revision: row.try_get("asset_revision")?,
            status: row.try_get("status")?,
            vectors_invalidated: row.try_get("vectors_invalidated")?,
        })
    }

    pub async fn replace_source(
        &self,
        request: ReplaceManagerKnowledgeSource,
    ) -> RepositoryResult<KnowledgeSourceMutation> {
        let row = sqlx::query(
            r#"
            SELECT asset_revision, status, source_file_id, vectors_invalidated
            FROM manager_replace_knowledge_source_revision(
                $1, $2, $3, $4, $5, $6, $7, $8, $9
            )
            "#,
        )
        .bind(request.asset_id)
        .bind(request.expected_revision)
        .bind(request.original_file_url)
        .bind(request.original_filename)
        .bind(request.mime_type)
        .bind(request.file_size_bytes)
        .bind(request.sha256)
        .bind(request.page_count)
        .bind(request.is_scanned_pdf)
        .fetch_one(&*self.base.pool())
        .await
        .map_err(map_governed_edit_error)?;

        Ok(KnowledgeSourceMutation {
            asset_revision: row.try_get("asset_revision")?,
            status: row.try_get("status")?,
            source_file_id: row.try_get("source_file_id")?,
            vectors_invalidated: row.try_get("vectors_invalidated")?,
        })
    }
}

fn map_governed_edit_error(error: sqlx::Error) -> RepositoryError {
    if let sqlx::Error::Database(database) = &error {
        let code = database.code().as_deref().unwrap_or_default();
        let message = database.message();
        return match (code, message) {
            ("40001", _) | (_, "knowledge_asset_revision_conflict") => RepositoryError::Validation(
                "Knowledge asset changed while it was being edited; refresh and try again".into(),
            ),
            ("P0002", _) | (_, "knowledge_asset_not_found") => RepositoryError::NotFound {
                entity: "KnowledgeAsset".into(),
                id: "scoped".into(),
            },
            ("42501", _) | (_, "knowledge_asset_forbidden") => RepositoryError::Unauthorized,
            ("22023", _) | (_, "knowledge_asset_metadata_invalid") | (_, "knowledge_source_revision_invalid") => {
                RepositoryError::Validation("Knowledge asset edit is invalid".into())
            }
            ("23514", _) if message.contains("archived_knowledge_asset_source_is_terminal") => {
                RepositoryError::Validation(
                    "Archived knowledge assets cannot replace their source document".into(),
                )
            }
            _ => RepositoryError::Database(error),
        };
    }
    RepositoryError::Database(error)
}

impl Repository for KnowledgeAssetEditRepository {
    fn pool(&self) -> Arc<AuthorizedPool> {
        self.base.pool()
    }
}
