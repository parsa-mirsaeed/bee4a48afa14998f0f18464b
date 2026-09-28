// PR-03: protected database access is transaction-scoped through AuthorizedPool.
use crate::repositories::{
    BaseRepository, PersistedChunk, Repository, RepositoryError, RepositoryResult,
};
use crate::rls_context::AuthorizedPool;
use sqlx::{postgres::PgConnection, Row};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ClaimedKnowledgeIngestionJob {
    pub id: Uuid,
    pub asset_id: Uuid,
    pub requested_by: Uuid,
    pub attempts: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingJobConfiguration {
    pub profile_id: String,
    pub provider: String,
    pub model: String,
    pub vector_size: i32,
    pub collection: String,
    pub chunk_size: i32,
    pub chunk_overlap: i32,
}

impl EmbeddingJobConfiguration {
    fn validate(&self) -> RepositoryResult<()> {
        if self.profile_id.trim().is_empty()
            || self.provider.trim().is_empty()
            || self.model.trim().is_empty()
            || self.collection.trim().is_empty()
            || self.vector_size <= 0
            || !(100..=20_000).contains(&self.chunk_size)
            || self.chunk_overlap < 0
            || self.chunk_overlap > self.chunk_size / 2
        {
            return Err(RepositoryError::Validation(
                "Embedding job configuration is invalid".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingFailureDisposition {
    Requeued,
    FailedPermanently,
    IgnoredInactive,
}

#[derive(Clone)]
pub struct KnowledgeIngestionJobRepository {
    base: BaseRepository,
}

impl KnowledgeIngestionJobRepository {
    pub fn new<T>(pool: T) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    /// Queue a legacy embedding job idempotently. New Platform Admin calls use
    /// `enqueue_embedding_configured` so retries keep an immutable vectorization
    /// contract; this compatibility path remains for older internal callers.
    pub async fn enqueue_embedding(
        &self,
        asset_id: Uuid,
        requested_by: Uuid,
    ) -> RepositoryResult<Uuid> {
        self.enqueue_embedding_inner(asset_id, requested_by, None)
            .await
    }

    /// Queue an embedding job with the exact registered profile and chunking
    /// contract selected by the administrator. The configuration is persisted on
    /// the durable job so worker retries cannot silently change model, dimensions
    /// or Qdrant collection after a deployment/configuration change.
    pub async fn enqueue_embedding_configured(
        &self,
        asset_id: Uuid,
        requested_by: Uuid,
        configuration: &EmbeddingJobConfiguration,
    ) -> RepositoryResult<Uuid> {
        configuration.validate()?;
        self.enqueue_embedding_inner(asset_id, requested_by, Some(configuration))
            .await
    }

    async fn enqueue_embedding_inner(
        &self,
        asset_id: Uuid,
        requested_by: Uuid,
        configuration: Option<&EmbeddingJobConfiguration>,
    ) -> RepositoryResult<Uuid> {
        let mut tx = self.base.pool().begin().await?;
        Self::lock_asset(&mut *tx, asset_id).await?;

        let status = sqlx::query_scalar::<_, String>(
            "SELECT status::text FROM knowledge_assets WHERE id = $1 FOR UPDATE",
        )
        .bind(asset_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "KnowledgeAsset".into(),
            id: asset_id.to_string(),
        })?;

        if !matches!(
            status.as_str(),
            "ocr_ready" | "embedded" | "failed" | "embedding_pending"
        ) {
            return Err(RepositoryError::Validation(
                "Only OCR-ready, embedded, failed, or pending assets can be queued for embedding"
                    .into(),
            ));
        }

        if let Some(existing) = sqlx::query(
            r#"
            SELECT id, embedding_profile, embedding_provider, embedding_model,
                   embedding_dimensions, embedding_collection, chunk_size, chunk_overlap
            FROM ingestion_jobs
            WHERE asset_id = $1
              AND stage = 'embed'
              AND status IN ('queued', 'running')
            FOR UPDATE
            "#,
        )
        .bind(asset_id)
        .fetch_optional(&mut *tx)
        .await?
        {
            let existing_id: Uuid = existing.try_get("id")?;
            if let Some(requested) = configuration {
                let existing_configuration =
                    Self::configuration_from_row(&existing)?.ok_or_else(|| {
                        RepositoryError::Validation(
                            "An active legacy embedding job has no persisted vectorization profile"
                                .into(),
                        )
                    })?;
                if &existing_configuration != requested {
                    return Err(RepositoryError::Validation(
                        "An active embedding job already uses a different vectorization profile"
                            .into(),
                    ));
                }
            }
            tx.commit().await?;
            return Ok(existing_id);
        }

        sqlx::query(
            "UPDATE knowledge_assets SET status = 'embedding_pending', failure_reason = NULL WHERE id = $1",
        )
        .bind(asset_id)
        .execute(&mut *tx)
        .await?;

        let (
            embedding_profile,
            embedding_provider,
            embedding_model,
            embedding_dimensions,
            embedding_collection,
            chunk_size,
            chunk_overlap,
        ) = configuration
            .map(|configuration| {
                (
                    Some(configuration.profile_id.clone()),
                    Some(configuration.provider.clone()),
                    Some(configuration.model.clone()),
                    Some(configuration.vector_size),
                    Some(configuration.collection.clone()),
                    Some(configuration.chunk_size),
                    Some(configuration.chunk_overlap),
                )
            })
            .unwrap_or((None, None, None, None, None, None, None));

        let job_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO ingestion_jobs (
                asset_id, stage, status, attempts, requested_by, available_at,
                embedding_profile, embedding_provider, embedding_model,
                embedding_dimensions, embedding_collection, chunk_size, chunk_overlap
            )
            VALUES (
                $1, 'embed', 'queued', 0, $2, NOW(),
                $3, $4, $5, $6, $7, $8, $9
            )
            RETURNING id
            "#,
        )
        .bind(asset_id)
        .bind(requested_by)
        .bind(embedding_profile)
        .bind(embedding_provider)
        .bind(embedding_model)
        .bind(embedding_dimensions)
        .bind(embedding_collection)
        .bind(chunk_size)
        .bind(chunk_overlap)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(job_id)
    }

    pub async fn get_embedding_job_configuration(
        &self,
        job_id: Uuid,
    ) -> RepositoryResult<Option<EmbeddingJobConfiguration>> {
        let row = sqlx::query(
            r#"
            SELECT embedding_profile, embedding_provider, embedding_model,
                   embedding_dimensions, embedding_collection, chunk_size, chunk_overlap
            FROM ingestion_jobs
            WHERE id = $1 AND stage = 'embed'
            "#,
        )
        .bind(job_id)
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "IngestionJob".into(),
            id: job_id.to_string(),
        })?;

        Self::configuration_from_row(&row)
    }

    fn configuration_from_row(row: &sqlx::postgres::PgRow) -> RepositoryResult<Option<EmbeddingJobConfiguration>> {
        let profile_id: Option<String> = row.try_get("embedding_profile")?;
        let Some(profile_id) = profile_id else {
            return Ok(None);
        };
        let configuration = EmbeddingJobConfiguration {
            profile_id,
            provider: row
                .try_get::<Option<String>, _>("embedding_provider")?
                .ok_or_else(|| RepositoryError::Validation("Embedding job provider is missing".into()))?,
            model: row
                .try_get::<Option<String>, _>("embedding_model")?
                .ok_or_else(|| RepositoryError::Validation("Embedding job model is missing".into()))?,
            vector_size: row
                .try_get::<Option<i32>, _>("embedding_dimensions")?
                .ok_or_else(|| RepositoryError::Validation("Embedding job dimensions are missing".into()))?,
            collection: row
                .try_get::<Option<String>, _>("embedding_collection")?
                .ok_or_else(|| RepositoryError::Validation("Embedding job collection is missing".into()))?,
            chunk_size: row
                .try_get::<Option<i32>, _>("chunk_size")?
                .ok_or_else(|| RepositoryError::Validation("Embedding job chunk size is missing".into()))?,
            chunk_overlap: row
                .try_get::<Option<i32>, _>("chunk_overlap")?
                .ok_or_else(|| RepositoryError::Validation("Embedding job chunk overlap is missing".into()))?,
        };
        configuration.validate()?;
        Ok(Some(configuration))
    }

    /// Atomically claim one available job. `SKIP LOCKED` allows multiple
    /// application instances to run workers without processing the same job.
    pub async fn claim_next_embedding(
        &self,
    ) -> RepositoryResult<Option<ClaimedKnowledgeIngestionJob>> {
        let row = sqlx::query(
            r#"
            WITH candidate AS (
                SELECT job.id
                FROM ingestion_jobs job
                JOIN knowledge_assets asset ON asset.id = job.asset_id
                WHERE job.stage = 'embed'
                  AND job.status = 'queued'
                  AND job.available_at <= NOW()
                  AND job.requested_by IS NOT NULL
                  AND asset.status = 'embedding_pending'
                ORDER BY job.available_at, job.created_at
                FOR UPDATE OF job SKIP LOCKED
                LIMIT 1
            )
            UPDATE ingestion_jobs job
            SET status = 'running',
                attempts = job.attempts + 1,
                started_at = COALESCE(job.started_at, NOW()),
                locked_at = NOW(),
                heartbeat_at = NOW(),
                error_message = NULL
            FROM candidate
            WHERE job.id = candidate.id
            RETURNING job.id, job.asset_id, job.requested_by, job.attempts
            "#,
        )
        .fetch_optional(&*self.base.pool())
        .await?;

        row.map(|row| {
            Ok(ClaimedKnowledgeIngestionJob {
                id: row.try_get("id")?,
                asset_id: row.try_get("asset_id")?,
                requested_by: row.try_get("requested_by")?,
                attempts: row.try_get("attempts")?,
            })
        })
        .transpose()
    }

    pub async fn heartbeat(&self, job_id: Uuid) -> RepositoryResult<()> {
        sqlx::query(
            "UPDATE ingestion_jobs SET heartbeat_at = NOW() WHERE id = $1 AND status = 'running'",
        )
        .bind(job_id)
        .execute(&*self.base.pool())
        .await?;
        Ok(())
    }

    /// Persist chunks and mark the job successful only while both the job and
    /// asset remain in their expected active states.
    pub async fn complete_embedding(
        &self,
        asset_id: Uuid,
        job_id: Uuid,
        actor_id: Uuid,
        chunks: &[PersistedChunk],
    ) -> RepositoryResult<()> {
        let mut tx = self.base.pool().begin().await?;
        Self::lock_asset(&mut *tx, asset_id).await?;

        let job_status = sqlx::query_scalar::<_, String>(
            "SELECT status::text FROM ingestion_jobs WHERE id = $1 AND asset_id = $2 FOR UPDATE",
        )
        .bind(job_id)
        .bind(asset_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "IngestionJob".into(),
            id: job_id.to_string(),
        })?;
        if job_status != "running" {
            return Err(RepositoryError::Validation(
                "Embedding job is no longer active".into(),
            ));
        }

        let asset_status = sqlx::query_scalar::<_, String>(
            "SELECT status::text FROM knowledge_assets WHERE id = $1 FOR UPDATE",
        )
        .bind(asset_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "KnowledgeAsset".into(),
            id: asset_id.to_string(),
        })?;
        if asset_status != "embedding_pending" {
            return Err(RepositoryError::Validation(
                "Knowledge asset is no longer awaiting embedding completion".into(),
            ));
        }

        sqlx::query("DELETE FROM knowledge_chunks WHERE asset_id = $1")
            .bind(asset_id)
            .execute(&mut *tx)
            .await?;

        for chunk in chunks {
            sqlx::query(
                r#"
                INSERT INTO knowledge_chunks (
                    asset_id, chunk_index, text, token_count, embedding_provider,
                    embedding_model, vector_id, metadata_json
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                "#,
            )
            .bind(asset_id)
            .bind(chunk.chunk_index)
            .bind(&chunk.text)
            .bind(chunk.token_count)
            .bind(&chunk.embedding_provider)
            .bind(&chunk.embedding_model)
            .bind(&chunk.vector_id)
            .bind(&chunk.metadata)
            .execute(&mut *tx)
            .await?;
        }

        let school_id = sqlx::query_scalar::<_, Uuid>(
            "UPDATE knowledge_assets SET status = 'embedded', reviewed_by = $2 WHERE id = $1 AND status = 'embedding_pending' RETURNING school_id",
        )
        .bind(asset_id)
        .bind(actor_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            RepositoryError::Validation(
                "Knowledge asset changed state before embedding completion".into(),
            )
        })?;

        let completed = sqlx::query(
            "UPDATE ingestion_jobs SET status = 'succeeded', finished_at = NOW(), locked_at = NULL, heartbeat_at = NULL WHERE id = $1 AND status = 'running'",
        )
        .bind(job_id)
        .execute(&mut *tx)
        .await?;
        if completed.rows_affected() != 1 {
            return Err(RepositoryError::Validation(
                "Embedding job changed state before completion".into(),
            ));
        }

        Self::append_audit(
            &mut *tx,
            actor_id,
            "knowledge_asset.embedded",
            asset_id,
            school_id,
            serde_json::json!({"chunk_count": chunks.len(), "job_id": job_id}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Archive an asset and cancel every queued or running ingestion job in the
    /// same transaction, preventing a worker from reviving the archived asset.
    pub async fn archive_asset(&self, asset_id: Uuid, actor_id: Uuid) -> RepositoryResult<()> {
        let mut tx = self.base.pool().begin().await?;
        Self::lock_asset(&mut *tx, asset_id).await?;

        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM knowledge_assets WHERE id = $1 FOR UPDATE)",
        )
        .bind(asset_id)
        .fetch_one(&mut *tx)
        .await?;
        if !exists {
            return Err(RepositoryError::NotFound {
                entity: "KnowledgeAsset".into(),
                id: asset_id.to_string(),
            });
        }

        sqlx::query(
            r#"
            UPDATE ingestion_jobs
            SET status = 'cancelled',
                finished_at = NOW(),
                locked_at = NULL,
                heartbeat_at = NULL,
                error_message = COALESCE(error_message, 'Cancelled because the asset was archived')
            WHERE asset_id = $1
              AND status IN ('queued', 'running')
            "#,
        )
        .bind(asset_id)
        .execute(&mut *tx)
        .await?;

        let school_id = sqlx::query_scalar::<_, Uuid>(
            "UPDATE knowledge_assets SET status = 'archived', reviewed_by = $2, archived_at = NOW() WHERE id = $1 RETURNING school_id",
        )
        .bind(asset_id)
        .bind(actor_id)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query("UPDATE teacher_asset_selections SET enabled = FALSE WHERE asset_id = $1")
            .bind(asset_id)
            .execute(&mut *tx)
            .await?;

        Self::append_audit(
            &mut *tx,
            actor_id,
            "knowledge_asset.archived",
            asset_id,
            school_id,
            serde_json::json!({"active_ingestion_jobs_cancelled": true}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Return jobs abandoned by terminated workers to the queue.
    pub async fn recover_stale_embedding_jobs(
        &self,
        stale_after_seconds: i64,
    ) -> RepositoryResult<u64> {
        let result = sqlx::query(
            r#"
            UPDATE ingestion_jobs job
            SET status = 'queued',
                available_at = NOW(),
                locked_at = NULL,
                heartbeat_at = NULL,
                error_message = COALESCE(job.error_message, 'Recovered after stale worker lock')
            FROM knowledge_assets asset
            WHERE job.asset_id = asset.id
              AND job.stage = 'embed'
              AND job.status = 'running'
              AND asset.status = 'embedding_pending'
              AND COALESCE(job.heartbeat_at, job.locked_at, job.started_at, job.created_at)
                    < NOW() - make_interval(secs => $1)
            "#,
        )
        .bind(stale_after_seconds.max(60))
        .execute(&*self.base.pool())
        .await?;
        Ok(result.rows_affected())
    }

    /// Requeue transient failures with exponential backoff and permanently fail
    /// the asset after the configured maximum number of attempts. If another
    /// operation cancelled or completed the job, leave its state untouched.
    pub async fn record_embedding_failure(
        &self,
        job: &ClaimedKnowledgeIngestionJob,
        error: &str,
        max_attempts: i32,
    ) -> RepositoryResult<EmbeddingFailureDisposition> {
        let mut tx = self.base.pool().begin().await?;
        Self::lock_asset(&mut *tx, job.asset_id).await?;

        let current_status = sqlx::query_scalar::<_, String>(
            "SELECT status::text FROM ingestion_jobs WHERE id = $1 FOR UPDATE",
        )
        .bind(job.id)
        .fetch_optional(&mut *tx)
        .await?;

        if current_status.as_deref() != Some("running") {
            tx.commit().await?;
            return Ok(EmbeddingFailureDisposition::IgnoredInactive);
        }

        let asset_status = sqlx::query_scalar::<_, String>(
            "SELECT status::text FROM knowledge_assets WHERE id = $1 FOR UPDATE",
        )
        .bind(job.asset_id)
        .fetch_optional(&mut *tx)
        .await?;
        if asset_status.as_deref() != Some("embedding_pending") {
            tx.commit().await?;
            return Ok(EmbeddingFailureDisposition::IgnoredInactive);
        }

        if job.attempts < max_attempts.max(1) {
            let backoff_seconds = 2_i64.pow(job.attempts.clamp(1, 10) as u32);
            sqlx::query(
                r#"
                UPDATE ingestion_jobs
                SET status = 'queued',
                    available_at = NOW() + make_interval(secs => $2),
                    locked_at = NULL,
                    heartbeat_at = NULL,
                    error_message = $3
                WHERE id = $1
                "#,
            )
            .bind(job.id)
            .bind(backoff_seconds)
            .bind(error)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            return Ok(EmbeddingFailureDisposition::Requeued);
        }

        sqlx::query(
            r#"
            UPDATE ingestion_jobs
            SET status = 'failed',
                error_message = $2,
                finished_at = NOW(),
                locked_at = NULL,
                heartbeat_at = NULL
            WHERE id = $1
            "#,
        )
        .bind(job.id)
        .bind(error)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "UPDATE knowledge_assets SET status = 'failed', failure_reason = $2 WHERE id = $1 AND status = 'embedding_pending'",
        )
        .bind(job.asset_id)
        .bind(error)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(EmbeddingFailureDisposition::FailedPermanently)
    }

    async fn lock_asset(tx: &mut PgConnection, asset_id: Uuid) -> RepositoryResult<()> {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
            .bind(asset_id)
            .execute(&mut *tx)
            .await?;
        Ok(())
    }

    async fn append_audit(
        tx: &mut PgConnection,
        actor_id: Uuid,
        action: &str,
        asset_id: Uuid,
        school_id: Uuid,
        details: serde_json::Value,
    ) -> RepositoryResult<()> {
        sqlx::query(
            r#"
            INSERT INTO knowledge_audit_logs (
                actor_id, actor_role, action, target_type, target_id, school_id, details_json
            ) VALUES ($1, 'PlatformAdmin', $2, 'knowledge_asset', $3, $4, $5)
            "#,
        )
        .bind(actor_id)
        .bind(action)
        .bind(asset_id)
        .bind(school_id)
        .bind(details)
        .execute(&mut *tx)
        .await?;
        Ok(())
    }
}
