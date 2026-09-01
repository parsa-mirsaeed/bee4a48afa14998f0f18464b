//! Platform Admin OCR endpoints with explicit governed-source concurrency checks.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::repositories::KnowledgeAssetRepository;
#[cfg(feature = "server")]
use crate::services::knowledge_asset_service::normalize_persian_text;
#[cfg(feature = "server")]
use sha2::{Digest, Sha256};
#[cfg(feature = "server")]
use sqlx::Row;
#[cfg(feature = "server")]
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdminKnowledgeSourceRevisionDto {
    pub asset_id: String,
    pub source_file_id: String,
    pub source_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveAdminVerifiedOcrRequest {
    pub asset_id: String,
    pub raw_text: String,
    pub ocr_provider: String,
    pub expected_source_file_id: String,
    pub expected_source_sha256: String,
    #[serde(default)]
    pub expected_revision: Option<String>,
}

#[cfg(feature = "server")]
async fn authorize_platform_admin() -> Result<
    (
        crate::domain::UserInfo,
        std::sync::Arc<crate::rls_context::AuthorizedPool>,
    ),
    ServerFnError,
> {
    let (user, pool) = crate::server_functions::rls_helpers::extract_user_with_full_rls().await?;
    if user.role != "PlatformAdmin" {
        return Err(ServerFnError::new("Forbidden: insufficient role"));
    }
    Ok((user, pool))
}

#[server(endpoint = "admin/knowledge-assets/source-revision")]
pub async fn get_admin_knowledge_source_revision(
    asset_id: String,
) -> Result<AdminKnowledgeSourceRevisionDto, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (_user, pool) = authorize_platform_admin().await?;
        let asset_id = Uuid::parse_str(&asset_id)
            .map_err(|_| ServerFnError::new("Invalid knowledge asset"))?;
        let row = sqlx::query(
            r#"
            SELECT asset.id AS asset_id,
                   source.id AS source_file_id,
                   lower(source.sha256) AS source_sha256
            FROM knowledge_assets AS asset
            JOIN knowledge_source_files AS source
              ON source.id = asset.current_source_file_id
             AND source.asset_id = asset.id
            WHERE asset.id = $1
              AND source.sha256 ~ '^[0-9A-Fa-f]{64}$'
            "#,
        )
        .bind(asset_id)
        .fetch_optional(&*pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, %asset_id, "platform knowledge source revision read failed");
            ServerFnError::new("Unable to load the governed source revision")
        })?
        .ok_or_else(|| ServerFnError::new("The governed source revision is unavailable"))?;

        Ok(AdminKnowledgeSourceRevisionDto {
            asset_id: row
                .try_get::<Uuid, _>("asset_id")
                .map_err(|_| ServerFnError::new("Unable to load the governed source revision"))?
                .to_string(),
            source_file_id: row
                .try_get::<Uuid, _>("source_file_id")
                .map_err(|_| ServerFnError::new("Unable to load the governed source revision"))?
                .to_string(),
            source_sha256: row
                .try_get("source_sha256")
                .map_err(|_| ServerFnError::new("Unable to load the governed source revision"))?,
        })
    }
    #[cfg(not(feature = "server"))]
    {
        let _ = asset_id;
        Err(ServerFnError::new("Server-only function"))
    }
}

#[server(endpoint = "admin/knowledge-assets/verified-ocr/source-bound")]
pub async fn save_admin_verified_ocr(
    request: SaveAdminVerifiedOcrRequest,
) -> Result<bool, ServerFnError> {
    #[cfg(feature = "server")]
    {
        if request.raw_text.trim().is_empty() || request.ocr_provider.trim().is_empty() {
            return Err(ServerFnError::new(
                "Verified OCR text and provider are required",
            ));
        }
        let (user, pool) = authorize_platform_admin().await?;
        let asset_id = Uuid::parse_str(&request.asset_id)
            .map_err(|_| ServerFnError::new("Invalid knowledge asset"))?;
        let verified_by = Uuid::parse_str(&user.id)
            .map_err(|_| ServerFnError::new("Invalid authenticated user ID"))?;
        let expected_source_file_id = Uuid::parse_str(&request.expected_source_file_id)
            .map_err(|_| ServerFnError::new("Invalid governed source revision"))?;
        let expected_source_sha256 = request.expected_source_sha256.trim().to_ascii_lowercase();
        if expected_source_sha256.len() != 64
            || !expected_source_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(ServerFnError::new("Invalid governed source revision"));
        }
        let expected_revision = request
            .expected_revision
            .as_deref()
            .map(Uuid::parse_str)
            .transpose()
            .map_err(|_| ServerFnError::new("Invalid OCR revision"))?;

        let clean_text = normalize_persian_text(&request.raw_text);
        if clean_text.is_empty() {
            return Err(ServerFnError::new("Verified OCR text is required"));
        }
        let text_sha256 = format!("{:x}", Sha256::digest(clean_text.as_bytes()));

        KnowledgeAssetRepository::new(pool)
            .attach_verified_ocr_for_source(
                asset_id,
                &request.raw_text,
                &clean_text,
                request.ocr_provider.trim(),
                verified_by,
                &text_sha256,
                expected_source_file_id,
                &expected_source_sha256,
                expected_revision,
            )
            .await
            .map_err(|error| {
                tracing::warn!(%error, %asset_id, "source-bound verified OCR save rejected");
                ServerFnError::new(
                    "OCR verification could not be saved. Refresh the asset, review the current source, and try again.",
                )
            })?;
        Ok(true)
    }
    #[cfg(not(feature = "server"))]
    {
        let _ = request;
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_revision_request_requires_explicit_identity_fields() {
        let request = SaveAdminVerifiedOcrRequest {
            asset_id: "asset".to_string(),
            raw_text: "verified".to_string(),
            ocr_provider: "manual-verified".to_string(),
            expected_source_file_id: "source".to_string(),
            expected_source_sha256: "a".repeat(64),
            expected_revision: None,
        };
        let encoded = serde_json::to_value(request).expect("serialize request");
        assert_eq!(encoded["expected_source_file_id"], "source");
        assert_eq!(encoded["expected_source_sha256"], "a".repeat(64));
    }
}
