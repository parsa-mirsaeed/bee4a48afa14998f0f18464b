//! SchoolManager metadata editing for an existing governed knowledge asset.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(feature = "server")]
use crate::repositories::{
    KnowledgeAssetEditRepository, RepositoryError, UpdateManagerKnowledgeMetadata,
};
#[cfg(feature = "server")]
use crate::services::KnowledgeVectorStoreService;
#[cfg(feature = "server")]
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManagerKnowledgeAssetEditState {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub subject: Option<String>,
    pub grade: Option<String>,
    pub language: String,
    pub template_type: Option<String>,
    pub tags: Value,
    pub status: String,
    pub asset_revision: i64,
    pub current_source_file_id: Option<String>,
    pub current_source_filename: Option<String>,
    pub current_source_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateKnowledgeAssetMetadataRequest {
    pub asset_id: String,
    pub expected_revision: i64,
    pub title: String,
    pub description: Option<String>,
    pub subject: Option<String>,
    pub grade: Option<String>,
    pub language: String,
    pub template_type: Option<String>,
    pub tags: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeAssetEditResult {
    pub asset_revision: i64,
    pub status: String,
    pub vectors_invalidated: bool,
    pub vector_cleanup_succeeded: bool,
}

#[cfg(feature = "server")]
async fn manager_actor() -> Result<(Uuid, Uuid), ServerFnError> {
    let (user, pool) = crate::server_functions::rls_helpers::extract_user_with_full_rls().await?;
    if user.role != "SchoolManager" {
        return Err(ServerFnError::new("Forbidden: insufficient role"));
    }
    let user_id = Uuid::parse_str(&user.id)
        .map_err(|_| ServerFnError::new("Invalid authenticated user ID"))?;
    let school_id = sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT school_id FROM users WHERE id = $1 AND is_active = TRUE",
    )
    .bind(user_id)
    .fetch_optional(&*pool)
    .await
    .map_err(|_| ServerFnError::new("Unable to resolve school scope"))?
    .flatten()
    .ok_or_else(|| ServerFnError::new("School manager has no school scope"))?;
    Ok((user_id, school_id))
}

#[server(endpoint = "manager/knowledge-assets/editable")]
pub async fn list_manager_knowledge_assets_for_editing(
) -> Result<Vec<ManagerKnowledgeAssetEditState>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (_, school_id) = manager_actor().await?;
        let items = KnowledgeAssetEditRepository::new(())
            .list_for_manager(school_id)
            .await
            .map_err(safe_edit_error)?;
        Ok(items
            .into_iter()
            .map(|item| ManagerKnowledgeAssetEditState {
                id: item.id.to_string(),
                title: item.title,
                description: item.description,
                subject: item.subject,
                grade: item.grade,
                language: item.language,
                template_type: item.template_type,
                tags: item.tags,
                status: item.status,
                asset_revision: item.asset_revision,
                current_source_file_id: item.current_source_file_id.map(|id| id.to_string()),
                current_source_filename: item.current_source_filename,
                current_source_sha256: item.current_source_sha256,
            })
            .collect())
    }
    #[cfg(not(feature = "server"))]
    Ok(Vec::new())
}

#[server(endpoint = "manager/knowledge-assets/update-metadata")]
pub async fn update_manager_knowledge_asset_metadata(
    request: UpdateKnowledgeAssetMetadataRequest,
) -> Result<KnowledgeAssetEditResult, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let _ = manager_actor().await?;
        let asset_id = Uuid::parse_str(&request.asset_id)
            .map_err(|_| ServerFnError::new("Invalid knowledge asset ID"))?;
        if request.expected_revision <= 0 {
            return Err(ServerFnError::new("Invalid knowledge asset revision"));
        }
        if request.title.trim().is_empty() || request.title.trim().len() > 255 {
            return Err(ServerFnError::new("Knowledge asset title is invalid"));
        }
        if !request.tags.is_object() {
            return Err(ServerFnError::new("Knowledge asset tags must be an object"));
        }

        let mutation = KnowledgeAssetEditRepository::new(())
            .update_metadata(UpdateManagerKnowledgeMetadata {
                asset_id,
                expected_revision: request.expected_revision,
                title: request.title,
                description: request.description,
                subject: request.subject,
                grade: request.grade,
                language: request.language,
                template_type: request.template_type,
                tags: request.tags,
            })
            .await
            .map_err(safe_edit_error)?;

        let vector_cleanup_succeeded = if mutation.vectors_invalidated {
            match KnowledgeVectorStoreService::new().await {
                Ok(store) => match store.delete_asset(&asset_id.to_string()).await {
                    Ok(()) => true,
                    Err(error) => {
                        tracing::error!(%asset_id, %error, "edited knowledge asset vectors require deferred cleanup");
                        false
                    }
                },
                Err(error) => {
                    tracing::error!(%asset_id, %error, "knowledge vector store unavailable during edit cleanup");
                    false
                }
            }
        } else {
            true
        };

        Ok(KnowledgeAssetEditResult {
            asset_revision: mutation.asset_revision,
            status: mutation.status,
            vectors_invalidated: mutation.vectors_invalidated,
            vector_cleanup_succeeded,
        })
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server-only function"))
}

#[cfg(feature = "server")]
fn safe_edit_error(error: RepositoryError) -> ServerFnError {
    let message = match error {
        RepositoryError::Unauthorized => {
            "Knowledge asset is not available in your school".to_string()
        }
        RepositoryError::NotFound { .. } => {
            "Knowledge asset is not available in your school".to_string()
        }
        RepositoryError::Validation(message) => message,
        RepositoryError::Duplicate { .. } => {
            "Knowledge asset edit conflicted with existing data".to_string()
        }
        RepositoryError::Database(error) => {
            tracing::error!(%error, "knowledge asset edit database failure");
            "Knowledge asset could not be updated".to_string()
        }
    };
    ServerFnError::new(message)
}
