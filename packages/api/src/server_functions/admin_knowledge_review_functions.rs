use crate::server_functions::knowledge_functions::KnowledgeAssetDto;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::repositories::KnowledgeAssetRepository;
#[cfg(feature = "server")]
use crate::services::{EmbeddingConfig, EmbeddingProfile, LOCAL_BGE_V1, OPENAI_V1};
#[cfg(feature = "server")]
use sqlx::Row;
#[cfg(feature = "server")]
use std::collections::{HashMap, HashSet};
#[cfg(feature = "server")]
use uuid::Uuid;

const KNOWLEDGE_SOURCE_BUCKET: &str = "edutalent-knowledge-sources";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdminKnowledgeVectorMethodDto {
    pub profile_id: String,
    pub provider: String,
    pub model: String,
    pub dimensions: u64,
    pub collection: String,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AdminKnowledgeVectorizationDto {
    pub methods: Vec<AdminKnowledgeVectorMethodDto>,
    pub job_id: Option<String>,
    pub job_status: Option<String>,
    pub selected_profile: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub dimensions: Option<i32>,
    pub collection: Option<String>,
    pub chunk_size: Option<i32>,
    pub chunk_overlap: Option<i32>,
    pub attempts: i32,
    pub stored_chunks: i64,
    pub last_error: Option<String>,
    pub queued_at: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdminKnowledgeReviewAssetDto {
    pub asset: KnowledgeAssetDto,
    /// Safe product identity for the owning school. The UUID remains available
    /// through `asset.school_id` for technical diagnostics, but is not the
    /// primary Platform Admin label.
    pub school_name: String,
    /// True when the canonical source has complete governed metadata and can be
    /// attempted through the protected review endpoint. Storage/object health
    /// is verified only by that endpoint and failures remain bounded product UI.
    pub source_review_available: bool,
    pub original_filename: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub has_verified_ocr: bool,
    #[serde(default)]
    pub has_source_review: bool,
    #[serde(default)]
    pub vectorization: AdminKnowledgeVectorizationDto,
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

#[cfg(feature = "server")]
fn method_dto(
    profile: EmbeddingProfile,
    active_profile: Option<&str>,
) -> AdminKnowledgeVectorMethodDto {
    AdminKnowledgeVectorMethodDto {
        profile_id: profile.id.to_string(),
        provider: profile.provider.as_str().to_string(),
        model: profile.model.to_string(),
        dimensions: profile.vector_size,
        collection: profile.collection.to_string(),
        available: active_profile == Some(profile.id),
    }
}

#[cfg(feature = "server")]
fn vectorization_methods() -> Vec<AdminKnowledgeVectorMethodDto> {
    let active_profile = EmbeddingConfig::from_env()
        .ok()
        .map(|config| config.profile.id.to_string());
    [OPENAI_V1, LOCAL_BGE_V1]
        .into_iter()
        .map(|profile| method_dto(profile, active_profile.as_deref()))
        .collect()
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
        let reviewer_id = Uuid::parse_str(&user.id)
            .map_err(|_| ServerFnError::new("Invalid authenticated user ID"))?;

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
        let school_ids = assets
            .iter()
            .map(|asset| asset.school_id)
            .collect::<Vec<_>>();
        let school_rows = sqlx::query("SELECT id, name FROM schools WHERE id = ANY($1)")
            .bind(&school_ids)
            .fetch_all(&*pool)
            .await
            .map_err(|error| {
                tracing::error!(%error, "platform knowledge school identity list failed");
                ServerFnError::new("Unable to load school identity")
            })?;
        let mut school_by_id = HashMap::<Uuid, String>::new();
        for row in school_rows {
            let school_id: Uuid = row.try_get("id").map_err(|error| {
                tracing::error!(%error, "platform knowledge school ID decode failed");
                ServerFnError::new("Unable to load school identity")
            })?;
            let school_name: String = row.try_get("name").map_err(|error| {
                tracing::error!(%error, "platform knowledge school name decode failed");
                ServerFnError::new("Unable to load school identity")
            })?;
            school_by_id.insert(school_id, school_name);
        }

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

        let reviewed_source_rows = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT review.asset_id
            FROM knowledge_source_reviews AS review
            JOIN knowledge_assets AS asset ON asset.id = review.asset_id
            JOIN knowledge_source_files AS source
              ON source.id = asset.current_source_file_id
             AND source.asset_id = asset.id
            WHERE review.asset_id = ANY($1)
              AND review.source_file_id = source.id
              AND lower(review.source_sha256) = lower(source.sha256)
              AND review.reviewed_by = $2
            "#,
        )
        .bind(&asset_ids)
        .bind(reviewer_id)
        .fetch_all(&*pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, "platform knowledge source review lookup failed");
            ServerFnError::new("Unable to load governed source review status")
        })?;
        let reviewed_asset_ids = reviewed_source_rows.into_iter().collect::<HashSet<_>>();

        let vector_rows = sqlx::query(
            r#"
            SELECT target.asset_id,
                   job.id AS job_id,
                   job.status::text AS job_status,
                   job.embedding_profile,
                   job.embedding_provider,
                   job.embedding_model,
                   job.embedding_dimensions,
                   job.embedding_collection,
                   job.chunk_size,
                   job.chunk_overlap,
                   COALESCE(job.attempts, 0) AS attempts,
                   job.error_message,
                   job.created_at AS queued_at,
                   job.started_at,
                   job.finished_at,
                   (
                       SELECT COUNT(*)::bigint
                       FROM knowledge_chunks AS chunk
                       WHERE chunk.asset_id = target.asset_id
                   ) AS stored_chunks
            FROM unnest($1::uuid[]) AS target(asset_id)
            LEFT JOIN LATERAL (
                SELECT id, status, embedding_profile, embedding_provider,
                       embedding_model, embedding_dimensions, embedding_collection,
                       chunk_size, chunk_overlap, attempts, error_message,
                       created_at, started_at, finished_at
                FROM ingestion_jobs
                WHERE asset_id = target.asset_id
                  AND stage = 'embed'
                ORDER BY created_at DESC
                LIMIT 1
            ) AS job ON TRUE
            "#,
        )
        .bind(&asset_ids)
        .fetch_all(&*pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, "platform knowledge vectorization status lookup failed");
            ServerFnError::new("Unable to load vectorization status")
        })?;
        let methods = vectorization_methods();
        let mut vectorization_by_asset = HashMap::<Uuid, AdminKnowledgeVectorizationDto>::new();
        for row in vector_rows {
            let asset_id: Uuid = row.try_get("asset_id").map_err(|error| {
                tracing::error!(%error, "platform vectorization asset ID decode failed");
                ServerFnError::new("Unable to load vectorization status")
            })?;
            let queued_at = row
                .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("queued_at")
                .map_err(|error| {
                    tracing::error!(%error, "platform vectorization queued time decode failed");
                    ServerFnError::new("Unable to load vectorization status")
                })?
                .map(|value| value.to_rfc3339());
            let started_at = row
                .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("started_at")
                .map_err(|error| {
                    tracing::error!(%error, "platform vectorization start time decode failed");
                    ServerFnError::new("Unable to load vectorization status")
                })?
                .map(|value| value.to_rfc3339());
            let finished_at = row
                .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("finished_at")
                .map_err(|error| {
                    tracing::error!(%error, "platform vectorization finish time decode failed");
                    ServerFnError::new("Unable to load vectorization status")
                })?
                .map(|value| value.to_rfc3339());
            vectorization_by_asset.insert(
                asset_id,
                AdminKnowledgeVectorizationDto {
                    methods: methods.clone(),
                    job_id: row
                        .try_get::<Option<Uuid>, _>("job_id")
                        .map_err(|_| ServerFnError::new("Unable to load vectorization status"))?
                        .map(|id| id.to_string()),
                    job_status: row
                        .try_get("job_status")
                        .map_err(|_| ServerFnError::new("Unable to load vectorization status"))?,
                    selected_profile: row
                        .try_get("embedding_profile")
                        .map_err(|_| ServerFnError::new("Unable to load vectorization status"))?,
                    provider: row
                        .try_get("embedding_provider")
                        .map_err(|_| ServerFnError::new("Unable to load vectorization status"))?,
                    model: row
                        .try_get("embedding_model")
                        .map_err(|_| ServerFnError::new("Unable to load vectorization status"))?,
                    dimensions: row
                        .try_get("embedding_dimensions")
                        .map_err(|_| ServerFnError::new("Unable to load vectorization status"))?,
                    collection: row
                        .try_get("embedding_collection")
                        .map_err(|_| ServerFnError::new("Unable to load vectorization status"))?,
                    chunk_size: row
                        .try_get("chunk_size")
                        .map_err(|_| ServerFnError::new("Unable to load vectorization status"))?,
                    chunk_overlap: row
                        .try_get("chunk_overlap")
                        .map_err(|_| ServerFnError::new("Unable to load vectorization status"))?,
                    attempts: row
                        .try_get("attempts")
                        .map_err(|_| ServerFnError::new("Unable to load vectorization status"))?,
                    stored_chunks: row
                        .try_get("stored_chunks")
                        .map_err(|_| ServerFnError::new("Unable to load vectorization status"))?,
                    last_error: row
                        .try_get("error_message")
                        .map_err(|_| ServerFnError::new("Unable to load vectorization status"))?,
                    queued_at,
                    started_at,
                    finished_at,
                },
            );
        }

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
                let school_name = school_by_id
                    .get(&asset.school_id)
                    .cloned()
                    .unwrap_or_else(|| "Unknown school".to_string());
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
                let has_source_review = reviewed_asset_ids.contains(&asset.id);
                let vectorization = vectorization_by_asset.remove(&asset.id).unwrap_or_else(|| {
                    AdminKnowledgeVectorizationDto {
                        methods: methods.clone(),
                        ..Default::default()
                    }
                });
                AdminKnowledgeReviewAssetDto {
                    asset: asset.into(),
                    school_name,
                    source_review_available,
                    original_filename: source
                        .as_ref()
                        .map(|source| source.original_filename.clone()),
                    file_size_bytes: source.and_then(|source| source.file_size_bytes),
                    has_verified_ocr,
                    has_source_review,
                    vectorization,
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

    #[test]
    fn admin_review_read_model_carries_school_display_identity() {
        let source = include_str!("admin_knowledge_review_functions.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert!(production.contains("pub school_name: String"));
        assert!(production.contains("SELECT id, name FROM schools"));
    }
}
