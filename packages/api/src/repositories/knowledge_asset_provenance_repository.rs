//! Source-bound persistence operations for governed knowledge assets.
//!
//! These operations deliberately keep the source-revision precondition and the
//! dependent OCR/embedding read in the same authorized PostgreSQL transaction.
//! Browser-visible source metadata is therefore only an optimistic precondition;
//! database truth remains authoritative at write/read time.

use crate::repositories::{
    AssetForEmbedding, KnowledgeAsset, KnowledgeAssetRepository, KnowledgeAssetStatus, Repository,
    RepositoryError, RepositoryResult,
};
use sqlx::Row;
use uuid::Uuid;

impl KnowledgeAssetRepository {
    #[allow(clippy::too_many_arguments)]
    pub async fn attach_verified_ocr_for_source(
        &self,
        asset_id: Uuid,
        raw_text: &str,
        clean_text: &str,
        provider: &str,
        verified_by: Uuid,
        text_sha256: &str,
        expected_source_file_id: Uuid,
        expected_source_sha256: &str,
        expected_revision: Option<Uuid>,
    ) -> RepositoryResult<()> {
        let expected_source_sha256 = expected_source_sha256.trim().to_ascii_lowercase();
        if expected_source_sha256.len() != 64
            || !expected_source_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(RepositoryError::Validation(
                "Invalid governed source revision".into(),
            ));
        }

        let mut tx = self.pool().begin().await?;
        let asset_row = sqlx::query(
            r#"
            SELECT asset.school_id,
                   asset.status::text AS status,
                   asset.current_source_file_id,
                   lower(source.sha256) AS source_sha256
            FROM knowledge_assets AS asset
            JOIN knowledge_source_files AS source
              ON source.id = asset.current_source_file_id
             AND source.asset_id = asset.id
            WHERE asset.id = $1
            FOR UPDATE OF asset, source
            "#,
        )
        .bind(asset_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "KnowledgeAsset".into(),
            id: asset_id.to_string(),
        })?;

        let school_id: Uuid = asset_row.try_get("school_id")?;
        let current_status =
            KnowledgeAssetStatus::parse(&asset_row.try_get::<String, _>("status")?)?;
        let current_source_file_id: Uuid = asset_row.try_get("current_source_file_id")?;
        let current_source_sha256: String = asset_row.try_get("source_sha256")?;

        if current_source_file_id != expected_source_file_id
            || current_source_sha256 != expected_source_sha256
        {
            return Err(RepositoryError::Validation(
                "The governed source changed while OCR was being reviewed; refresh and review the current source"
                    .into(),
            ));
        }
        if !current_status.accepts_verified_ocr() {
            return Err(RepositoryError::Validation(
                "Verified OCR can only be attached while an asset is awaiting review".into(),
            ));
        }

        let reviewed = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM knowledge_source_reviews AS review
                WHERE review.asset_id = $1
                  AND review.source_file_id = $2
                  AND review.source_sha256 = $3
                  AND review.reviewed_by = $4
            )
            "#,
        )
        .bind(asset_id)
        .bind(current_source_file_id)
        .bind(&current_source_sha256)
        .bind(verified_by)
        .fetch_one(&mut *tx)
        .await?;
        if !reviewed {
            return Err(RepositoryError::Validation(
                "Review the current source document before saving verified OCR".into(),
            ));
        }

        // The database trigger requires the exact optimistic source revision for
        // PlatformAdmin OCR writes. Keeping these transaction-local settings on
        // the same locked transaction means legacy/direct endpoints that omit the
        // precondition fail closed instead of silently binding stale text to a
        // newly reviewed replacement source.
        sqlx::query(
            r#"
            SELECT
                set_config('app.knowledge_expected_source_file_id', $1, true),
                set_config('app.knowledge_expected_source_sha256', $2, true)
            "#,
        )
        .bind(current_source_file_id.to_string())
        .bind(&current_source_sha256)
        .execute(&mut *tx)
        .await?;

        let current_revision = sqlx::query_scalar::<_, Uuid>(
            "SELECT revision FROM knowledge_ocr_texts WHERE asset_id = $1 FOR UPDATE",
        )
        .bind(asset_id)
        .fetch_optional(&mut *tx)
        .await?;
        let next_revision = Uuid::new_v4();

        match current_revision {
            Some(current_revision) if expected_revision == Some(current_revision) => {
                let updated = sqlx::query(
                    r#"
                    UPDATE knowledge_ocr_texts
                    SET raw_text = $2,
                        clean_text = $3,
                        ocr_provider = $4,
                        ocr_verified_by = $5,
                        ocr_verified_at = NOW(),
                        text_sha256 = $6,
                        revision = $7,
                        updated_at = NOW()
                    WHERE asset_id = $1 AND revision = $8
                    "#,
                )
                .bind(asset_id)
                .bind(raw_text)
                .bind(clean_text)
                .bind(provider)
                .bind(verified_by)
                .bind(text_sha256)
                .bind(next_revision)
                .bind(current_revision)
                .execute(&mut *tx)
                .await?
                .rows_affected();
                if updated != 1 {
                    return Err(RepositoryError::Validation(
                        "Verified OCR changed while it was being reviewed; refresh and try again"
                            .into(),
                    ));
                }
            }
            Some(_) => {
                return Err(RepositoryError::Validation(
                    "Verified OCR changed while it was being reviewed; refresh and try again"
                        .into(),
                ));
            }
            None if expected_revision.is_some() => {
                return Err(RepositoryError::Validation(
                    "Verified OCR is no longer available; refresh and try again".into(),
                ));
            }
            None => {
                let inserted = sqlx::query(
                    r#"
                    INSERT INTO knowledge_ocr_texts (
                        asset_id, raw_text, clean_text, ocr_provider, ocr_verified_by,
                        ocr_verified_at, text_sha256, revision
                    ) VALUES ($1, $2, $3, $4, $5, NOW(), $6, $7)
                    ON CONFLICT (asset_id) DO NOTHING
                    "#,
                )
                .bind(asset_id)
                .bind(raw_text)
                .bind(clean_text)
                .bind(provider)
                .bind(verified_by)
                .bind(text_sha256)
                .bind(next_revision)
                .execute(&mut *tx)
                .await?
                .rows_affected();
                if inserted != 1 {
                    return Err(RepositoryError::Validation(
                        "Verified OCR changed while it was being reviewed; refresh and try again"
                            .into(),
                    ));
                }
            }
        }

        let updated = sqlx::query(
            r#"
            UPDATE knowledge_assets
            SET status = 'ocr_ready', reviewed_by = $2, failure_reason = NULL
            WHERE id = $1
              AND current_source_file_id = $3
              AND status IN ('submitted', 'ocr_pending', 'ocr_ready', 'failed')
            "#,
        )
        .bind(asset_id)
        .bind(verified_by)
        .bind(current_source_file_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if updated != 1 {
            return Err(RepositoryError::Validation(
                "The asset or governed source changed before verified OCR could be saved".into(),
            ));
        }

        sqlx::query(
            r#"
            INSERT INTO knowledge_audit_logs (
                actor_id, actor_role, action, target_type, target_id, school_id, details_json
            ) VALUES (
                $1, 'PlatformAdmin', 'knowledge_asset.ocr_verified', 'knowledge_asset', $2, $3, $4
            )
            "#,
        )
        .bind(verified_by)
        .bind(asset_id)
        .bind(school_id)
        .bind(serde_json::json!({
            "ocr_provider": provider,
            "text_sha256": text_sha256,
            "ocr_revision": next_revision,
            "source_file_id": current_source_file_id,
            "source_sha256": current_source_sha256,
            "previous_status": current_status.as_str(),
            "status": "ocr_ready"
        }))
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_for_embedding_current(
        &self,
        asset_id: Uuid,
    ) -> RepositoryResult<AssetForEmbedding> {
        let row = sqlx::query(
            r#"
            SELECT asset.id,
                   asset.school_id,
                   asset.title,
                   asset.description,
                   asset.source_type,
                   asset.status::text AS status,
                   asset.language,
                   asset.subject,
                   asset.grade,
                   asset.template_type,
                   asset.tags,
                   asset.created_by,
                   asset.reviewed_by,
                   asset.published_at,
                   asset.failure_reason,
                   asset.created_at,
                   asset.updated_at,
                   ocr.clean_text
            FROM knowledge_assets AS asset
            JOIN knowledge_source_files AS source
              ON source.id = asset.current_source_file_id
             AND source.asset_id = asset.id
            JOIN knowledge_ocr_texts AS ocr
              ON ocr.asset_id = asset.id
             AND ocr.source_file_id = source.id
             AND lower(ocr.source_sha256) = lower(source.sha256)
            WHERE asset.id = $1
              AND asset.status IN ('ocr_ready', 'failed', 'embedded', 'embedding_pending')
              AND EXISTS (
                  SELECT 1
                  FROM knowledge_source_reviews AS review
                  WHERE review.asset_id = asset.id
                    AND review.source_file_id = source.id
                    AND review.source_sha256 = lower(source.sha256)
              )
            "#,
        )
        .bind(asset_id)
        .fetch_optional(&*self.pool())
        .await?
        .ok_or_else(|| {
            RepositoryError::Validation(
                "Asset must have reviewed current-source OCR before embedding".into(),
            )
        })?;

        let status = KnowledgeAssetStatus::parse(&row.try_get::<String, _>("status")?)?;
        Ok(AssetForEmbedding {
            asset: KnowledgeAsset {
                id: row.try_get("id")?,
                school_id: row.try_get("school_id")?,
                title: row.try_get("title")?,
                description: row.try_get("description")?,
                source_type: row.try_get("source_type")?,
                status,
                language: row.try_get("language")?,
                subject: row.try_get("subject")?,
                grade: row.try_get("grade")?,
                template_type: row.try_get("template_type")?,
                tags: row.try_get("tags")?,
                created_by: row.try_get("created_by")?,
                reviewed_by: row.try_get("reviewed_by")?,
                published_at: row.try_get("published_at")?,
                failure_reason: row.try_get("failure_reason")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            },
            clean_text: row.try_get("clean_text")?,
        })
    }
}
