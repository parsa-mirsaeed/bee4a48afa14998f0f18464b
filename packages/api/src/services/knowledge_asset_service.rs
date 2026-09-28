// PR-03: protected database access is transaction-scoped through AuthorizedPool.
//! Application service for reviewed knowledge ingestion and filtered retrieval.

use crate::repositories::{
    EmbeddingJobConfiguration, KnowledgeAssetRepository, KnowledgeIngestionJobRepository,
    PersistedChunk, RepositoryError,
};
use crate::rls_context::AuthorizedPool;
use crate::services::embedding_service::{
    chunk_document, ChunkMetadata, EmbeddingClient, EmbeddingConfig, EmbeddingError,
};
use crate::services::knowledge_vector_store_service::{
    KnowledgeSearchResult, KnowledgeVectorPoint, KnowledgeVectorStoreService,
};
use crate::services::vector_store_service::VectorStoreError;
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum KnowledgeAssetError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Embedding error: {0}")]
    Embedding(#[from] EmbeddingError),
    #[error("Vector store error: {0}")]
    VectorStore(#[from] VectorStoreError),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("No verified OCR text is available")]
    EmptyText,
    #[error("At least one published and enabled knowledge asset is required")]
    NoEnabledAssets,
    #[error("Embedding job configuration does not match the active deployment: {0}")]
    EmbeddingConfigurationMismatch(String),
}

#[derive(Clone)]
pub struct KnowledgeAssetService {
    pool: Arc<AuthorizedPool>,
    repository: KnowledgeAssetRepository,
    embedding_client: EmbeddingClient,
    embedding_config: EmbeddingConfig,
    vector_store: KnowledgeVectorStoreService,
}

impl KnowledgeAssetService {
    pub async fn new(pool: Arc<AuthorizedPool>) -> Result<Self, KnowledgeAssetError> {
        let embedding_config = EmbeddingConfig::from_env()?;
        let embedding_client = EmbeddingClient::with_config(embedding_config.clone())?;
        let vector_store = KnowledgeVectorStoreService::new().await?;
        Ok(Self {
            repository: KnowledgeAssetRepository::new(Arc::clone(&pool)),
            pool,
            embedding_client,
            embedding_config,
            vector_store,
        })
    }

    pub async fn attach_verified_ocr(
        &self,
        asset_id: Uuid,
        raw_text: &str,
        provider: &str,
        verified_by: Uuid,
        expected_revision: Option<Uuid>,
    ) -> Result<(), KnowledgeAssetError> {
        let clean_text = normalize_persian_text(raw_text);
        if clean_text.is_empty() {
            return Err(KnowledgeAssetError::EmptyText);
        }
        let text_sha256 = sha256_hex(clean_text.as_bytes());
        self.repository
            .attach_verified_ocr(
                asset_id,
                raw_text,
                &clean_text,
                provider,
                verified_by,
                &text_sha256,
                expected_revision,
            )
            .await?;
        Ok(())
    }

    pub async fn process_embedding_job(
        &self,
        job_id: Uuid,
        asset_id: Uuid,
        actor_id: Uuid,
    ) -> Result<usize, KnowledgeAssetError> {
        let job_configuration = KnowledgeIngestionJobRepository::new(Arc::clone(&self.pool))
            .get_embedding_job_configuration(job_id)
            .await?;
        let (chunk_size, chunk_overlap) = match job_configuration.as_ref() {
            Some(configuration) => {
                self.verify_job_configuration(configuration)?;
                (
                    configuration.chunk_size as usize,
                    configuration.chunk_overlap as usize,
                )
            }
            None => (
                std::env::var("KNOWLEDGE_CHUNK_SIZE")
                    .ok()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(1800usize),
                std::env::var("KNOWLEDGE_CHUNK_OVERLAP")
                    .ok()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(200usize),
            ),
        };

        let source = self.repository.get_for_embedding_current(asset_id).await?;
        if source.clean_text.trim().is_empty() {
            return Err(KnowledgeAssetError::EmptyText);
        }
        self.embed_asset_inner(
            asset_id,
            actor_id,
            job_id,
            source,
            chunk_size,
            chunk_overlap,
        )
        .await
    }

    fn verify_job_configuration(
        &self,
        configuration: &EmbeddingJobConfiguration,
    ) -> Result<(), KnowledgeAssetError> {
        let active = &self.embedding_config;
        let matches_active = configuration.profile_id == active.profile.id
            && configuration.provider == active.profile.provider.as_str()
            && configuration.model == active.model
            && configuration.vector_size as u64 == active.vector_size
            && configuration.collection == active.collection_name;
        if matches_active {
            return Ok(());
        }

        Err(KnowledgeAssetError::EmbeddingConfigurationMismatch(format!(
            "queued profile {} targets {} / {} dimensions, active profile is {} targeting {} / {} dimensions",
            configuration.profile_id,
            configuration.collection,
            configuration.vector_size,
            active.profile.id,
            active.collection_name,
            active.vector_size,
        )))
    }

    async fn embed_asset_inner(
        &self,
        asset_id: Uuid,
        actor_id: Uuid,
        job_id: Uuid,
        source: crate::repositories::AssetForEmbedding,
        chunk_size: usize,
        chunk_overlap: usize,
    ) -> Result<usize, KnowledgeAssetError> {
        let chunks = chunk_document(
            &source.clean_text,
            chunk_size,
            chunk_overlap,
            ChunkMetadata::default(),
        );
        if chunks.is_empty() {
            return Err(KnowledgeAssetError::EmptyText);
        }

        let school_id = source.asset.school_id;
        let mut embeddings = Vec::with_capacity(chunks.len());
        let batch_size = self.embedding_client.recommended_batch_size();
        for batch in chunks.chunks(batch_size) {
            embeddings.extend(
                self.embedding_client
                    .embed_batch_for_school(
                        school_id,
                        batch.iter().map(|chunk| chunk.text.clone()).collect(),
                    )
                    .await?,
            );
        }

        let metadata_base = serde_json::json!({
            "school_id": source.asset.school_id,
            "asset_id": source.asset.id,
            "language": source.asset.language,
            "subject": source.asset.subject,
            "grade": source.asset.grade,
            "template_type": source.asset.template_type,
            "published": false,
            "tags": source.asset.tags,
            "embedding_profile": self.embedding_config.profile.id,
            "embedding_collection": self.embedding_config.collection_name,
            "embedding_dimensions": self.embedding_config.vector_size,
            "chunk_size": chunk_size,
            "chunk_overlap": chunk_overlap,
        });

        let vector_points = chunks
            .iter()
            .zip(embeddings.iter())
            .map(|(chunk, embedding)| KnowledgeVectorPoint {
                asset_id: source.asset.id.to_string(),
                school_id: source.asset.school_id.to_string(),
                title: source.asset.title.clone(),
                language: source.asset.language.clone(),
                subject: source.asset.subject.clone(),
                grade: source.asset.grade.clone(),
                template_type: source.asset.template_type.clone(),
                chunk_index: chunk.chunk_index,
                text: chunk.text.clone(),
                embedding: embedding.clone(),
            })
            .collect();
        self.vector_store
            .replace_asset_points(&asset_id.to_string(), vector_points)
            .await?;

        let persisted = chunks
            .into_iter()
            .map(|chunk| PersistedChunk {
                chunk_index: chunk.chunk_index as i32,
                token_count: estimate_token_count(&chunk.text) as i32,
                vector_id: format!("knowledge:{}:{}", asset_id, chunk.chunk_index),
                text: chunk.text,
                embedding_provider: self.embedding_config.profile.provider.as_str().to_string(),
                embedding_model: self.embedding_config.model.clone(),
                metadata: metadata_base.clone(),
            })
            .collect::<Vec<_>>();

        KnowledgeIngestionJobRepository::new(Arc::clone(&self.pool))
            .complete_embedding(asset_id, job_id, actor_id, &persisted)
            .await?;
        Ok(persisted.len())
    }

    pub async fn publish_asset(
        &self,
        asset_id: Uuid,
        actor_id: Uuid,
    ) -> Result<(), KnowledgeAssetError> {
        self.repository.publish(asset_id, actor_id).await?;
        if let Err(error) = self
            .vector_store
            .set_published(&asset_id.to_string(), true)
            .await
        {
            tracing::error!(asset_id = %asset_id, error = %error, "Failed to publish Qdrant payload");
            sqlx::query(
                "UPDATE knowledge_assets SET status = 'embedded', published_at = NULL WHERE id = $1",
            )
            .bind(asset_id)
            .execute(&*self.pool)
            .await?;
            return Err(error.into());
        }
        Ok(())
    }

    pub async fn archive_asset(
        &self,
        asset_id: Uuid,
        actor_id: Uuid,
    ) -> Result<(), KnowledgeAssetError> {
        self.vector_store
            .set_published(&asset_id.to_string(), false)
            .await?;
        KnowledgeIngestionJobRepository::new(Arc::clone(&self.pool))
            .archive_asset(asset_id, actor_id)
            .await?;
        Ok(())
    }

    pub async fn search_for_teacher(
        &self,
        teacher_user_id: Uuid,
        query: &str,
        requested_asset_ids: &[Uuid],
        context_scope: &str,
        context_key: &str,
        top_k: usize,
    ) -> Result<Vec<KnowledgeSearchResult>, KnowledgeAssetError> {
        let authorized = self
            .repository
            .authorized_enabled_asset_ids(
                teacher_user_id,
                requested_asset_ids,
                context_scope,
                context_key,
            )
            .await?;
        if authorized.is_empty() {
            return Err(KnowledgeAssetError::NoEnabledAssets);
        }
        let school_id: Uuid = sqlx::query("SELECT school_id FROM users WHERE id = $1")
            .bind(teacher_user_id)
            .fetch_one(&*self.pool)
            .await?
            .try_get("school_id")?;
        let query_embedding = self
            .embedding_client
            .embed_query_for_school(school_id, query)
            .await?;
        let authorized_strings = authorized.iter().map(Uuid::to_string).collect::<Vec<_>>();
        let results = self
            .vector_store
            .search(
                query_embedding,
                &school_id.to_string(),
                &authorized_strings,
                top_k.clamp(1, 50),
            )
            .await?;
        self.repository
            .append_query_audit(teacher_user_id, school_id, &authorized, results.len())
            .await?;
        Ok(results)
    }
}

pub fn normalize_persian_text(input: &str) -> String {
    let normalized = input
        .replace('ي', "ی")
        .replace('ى', "ی")
        .replace('ك', "ک")
        .replace('\u{0640}', "")
        .replace("\r\n", "\n")
        .replace('\r', "\n");

    normalized
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn estimate_token_count(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_arabic_codepoints_used_in_persian() {
        assert_eq!(normalize_persian_text("كتاب  يكي"), "کتاب یکی");
    }

    #[test]
    fn collapses_whitespace_without_merging_paragraphs() {
        assert_eq!(normalize_persian_text("a   b\n\n c"), "a b\nc");
    }
}
