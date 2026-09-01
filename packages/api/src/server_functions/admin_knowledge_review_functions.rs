use crate::server_functions::knowledge_functions::KnowledgeAssetDto;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::repositories::KnowledgeAssetRepository;
#[cfg(feature = "server")]
use sqlx::Row;
#[cfg(feature = "server")]
use std::collections::{HashMap, HashSet};
#[cfg(feature = "server")]
use uuid::Uuid;

const KNOWLEDGE_SOURCE_BUCKET: &str = "edutalent-knowledge-sources";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdminKnowledgeReviewAssetDto {
    pub asset: KnowledgeAssetDto,
    /// True when the canonical source has complete governed metadata and can be
    /// attempted through the protected review endpoint. Storage/object health
    /// is verified only by that endpoint and failures remain bounded product UI.
    pub source_review_available: bool,
    pub original_filename: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub has_verified_ocr: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdminVerifiedOcrDto {
    pub asset_id: String,
    pub raw_text: String,
    pub ocr_provider: String,
    pub verified_at: String,
    pub verified_by: String,
    pub revision: String,
    pub text_sha256: Option<String>,
    pub source_sha256: Option<String>,
}

#[cfg(feature = "server")]
#[derive(Debug)]
struct SourceReviewMetadata {
    original_file_url: Option<String>,
    original_filename: String,
    mime_type: String,
    file_size_bytes: Option<i64>,
    sha256: Option<String>,
}

#[server(endpoint = "admin/knowledge-assets/review-list")]
pub async fn list_admin_knowledge_assets_for_review(
) -> Result<Vec<AdminKnowledgeReviewAssetDto>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (user, pool) =
            crate::server_functions::rls_helpers::extract_user_with_full_rls().await?;
        if user.role != "PlatformAdmin" {
            return Err(ServerFnError::new("Forbidden: insufficient role"));
        }

        let assets = KnowledgeAssetRepository::new(pool.clone())
            .list_for_admin()
            .await
            .map_err(|error| {
                tracing::error!(%error, "platform knowledge review list failed");
                ServerFnError::new("Unable to load governed knowledge assets")
            })?;
        if assets.is_empty() {
            return Ok(Vec::new());
        }

        let asset_ids = assets.iter().map(|asset| asset.id).collect::<Vec<_>>();
        let source_rows = sqlx::query(
            r#"
            SELECT
                asset.id AS asset_id,
                source.original_file_url,
                source.original_filename,
                source.mime_type,
                source.file_size_bytes,
                source.sha256
            FROM knowledge_assets AS asset
            JOIN knowledge_source_files AS source
              ON source.id = asset.current_source_file_id
             AND source.asset_id = asset.id
            WHERE asset.id = ANY($1)
            "#,
        )
        .bind(&asset_ids)
        .fetch_all(&*pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, "platform knowledge source metadata list failed");
            ServerFnError::new("Unable to load governed source metadata")
        })?;

        let ocr_rows = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT ocr.asset_id
            FROM knowledge_ocr_texts AS ocr
            JOIN knowledge_assets AS asset ON asset.id = ocr.asset_id
            JOIN knowledge_source_files AS source
              ON source.id = asset.current_source_file_id
             AND source.asset_id = asset.id
            WHERE ocr.asset_id = ANY($1)
              AND ocr.source_file_id = source.id
              AND lower(ocr.source_sha256) = lower(source.sha256)
            "#,
        )
        .bind(&asset_ids)
        .fetch_all(&*pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, "platform knowledge OCR readiness list failed");
            ServerFnError::new("Unable to load governed OCR readiness")
        })?;
        let ocr_asset_ids = ocr_rows.into_iter().collect::<HashSet<_>>();

        let mut source_by_asset = HashMap::<Uuid, SourceReviewMetadata>::new();
        for row in source_rows {
            let asset_id: Uuid = row.try_get("asset_id").map_err(|error| {
                tracing::error!(%error, "platform knowledge source metadata decode failed");
                ServerFnError::new("Unable to load governed source metadata")
            })?;
            source_by_asset.insert(
                asset_id,
                SourceReviewMetadata {
                    original_file_url: row.try_get("original_file_url").map_err(|error| {
                        tracing::error!(%error, "platform knowledge source URL decode failed");
                        ServerFnError::new("Unable to load governed source metadata")
                    })?,
                    original_filename: row.try_get("original_filename").map_err(|error| {
                        tracing::error!(%error, "platform knowledge source filename decode failed");
                        ServerFnError::new("Unable to load governed source metadata")
                    })?,
                    mime_type: row.try_get("mime_type").map_err(|error| {
                        tracing::error!(%error, "platform knowledge source MIME decode failed");
                        ServerFnError::new("Unable to load governed source metadata")
                    })?,
                    file_size_bytes: row.try_get("file_size_bytes").map_err(|error| {
                        tracing::error!(%error, "platform knowledge source size decode failed");
                        ServerFnError::new("Unable to load governed source metadata")
                    })?,
                    sha256: row.try_get("sha256").map_err(|error| {
                        tracing::error!(%error, "platform knowledge source hash decode failed");
                        ServerFnError::new("Unable to load governed source metadata")
                    })?,
                },
            );
        }

        Ok(assets
            .into_iter()
            .map(|asset| {
                let source = source_by_asset.remove(&asset.id);
                let source_review_available = source.as_ref().is_some_and(|source| {
                    source.mime_type == "application/pdf"
                        && source.sha256.as_deref().is_some_and(is_sha256)
                        && source
                            .original_file_url
                            .as_deref()
                            .is_some_and(|reference| {
                                let expected_prefix = format!(
                                    "storage://{KNOWLEDGE_SOURCE_BUCKET}/{}/",
                                    asset.school_id
                                );
                                reference.starts_with(&expected_prefix)
                            })
                });
                let has_verified_ocr = ocr_asset_ids.contains(&asset.id);
                AdminKnowledgeReviewAssetDto {
                    asset: asset.into(),
                    source_review_available,
                    original_filename: source
                        .as_ref()
                        .map(|source| source.original_filename.clone()),
                    file_size_bytes: source.and_then(|source| source.file_size_bytes),
                    has_verified_ocr,
                }
            })
            .collect())
    }
    #[cfg(not(feature = "server"))]
    Ok(Vec::new())
}

#[server(endpoint = "admin/knowledge-assets/verified-ocr")]
pub async fn get_admin_verified_ocr(
    asset_id: String,
) -> Result<Option<AdminVerifiedOcrDto>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (user, pool) =
            crate::server_functions::rls_helpers::extract_user_with_full_rls().await?;
        if user.role != "PlatformAdmin" {
            return Err(ServerFnError::new("Forbidden: insufficient role"));
        }
        let asset_id = Uuid::parse_str(&asset_id)
            .map_err(|_| ServerFnError::new("Invalid knowledge asset"))?;
        let row = sqlx::query(
            r#"
            SELECT ocr.asset_id, ocr.raw_text, ocr.ocr_provider, ocr.ocr_verified_at,
                   ocr.ocr_verified_by, ocr.revision, ocr.text_sha256, ocr.source_sha256
            FROM knowledge_ocr_texts AS ocr
            JOIN knowledge_assets AS asset ON asset.id = ocr.asset_id
            JOIN knowledge_source_files AS source
              ON source.id = asset.current_source_file_id
             AND source.asset_id = asset.id
            WHERE ocr.asset_id = $1
              AND ocr.source_file_id = source.id
              AND lower(ocr.source_sha256) = lower(source.sha256)
            "#,
        )
        .bind(asset_id)
        .fetch_optional(&*pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, %asset_id, "platform OCR record read failed");
            ServerFnError::new("Unable to load verified OCR")
        })?;

        row.map(|row| {
            let verified_at: chrono::DateTime<chrono::Utc> = row.try_get("ocr_verified_at")?;
            let verified_by: Uuid = row.try_get("ocr_verified_by")?;
            let revision: Uuid = row.try_get("revision")?;
            Ok(AdminVerifiedOcrDto {
                asset_id: row.try_get::<Uuid, _>("asset_id")?.to_string(),
                raw_text: row.try_get("raw_text")?,
                ocr_provider: row.try_get("ocr_provider")?,
                verified_at: verified_at.to_rfc3339(),
                verified_by: verified_by.to_string(),
                revision: revision.to_string(),
                text_sha256: row.try_get("text_sha256")?,
                source_sha256: row.try_get("source_sha256")?,
            })
        })
        .transpose()
        .map_err(|error: sqlx::Error| {
            tracing::error!(%error, %asset_id, "platform OCR record decode failed");
            ServerFnError::new("Unable to load verified OCR")
        })
    }
    #[cfg(not(feature = "server"))]
    {
        let _ = asset_id;
        Ok(None)
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_bucket_name_is_fixed() {
        assert_eq!(KNOWLEDGE_SOURCE_BUCKET, "edutalent-knowledge-sources");
    }

    #[test]
    fn review_metadata_requires_a_sha256_digest() {
        assert!(is_sha256(
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        ));
        assert!(!is_sha256("missing"));
        assert!(!is_sha256(
            "za7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        ));
    }
}
