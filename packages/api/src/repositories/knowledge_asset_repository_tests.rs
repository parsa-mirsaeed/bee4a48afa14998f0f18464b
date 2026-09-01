use super::knowledge_asset_repository::KnowledgeAssetStatus;
#[cfg(feature = "server")]
use super::KnowledgeAssetRepository;
#[cfg(feature = "server")]
use crate::repositories::{Repository, RepositoryError};

#[test]
fn status_strings_round_trip() {
    let cases = [
        (KnowledgeAssetStatus::Submitted, "submitted"),
        (KnowledgeAssetStatus::OcrPending, "ocr_pending"),
        (KnowledgeAssetStatus::OcrReady, "ocr_ready"),
        (KnowledgeAssetStatus::EmbeddingPending, "embedding_pending"),
        (KnowledgeAssetStatus::Embedded, "embedded"),
        (KnowledgeAssetStatus::Published, "published"),
        (KnowledgeAssetStatus::Archived, "archived"),
        (KnowledgeAssetStatus::Failed, "failed"),
    ];

    for (status, expected) in cases {
        assert_eq!(status.as_str(), expected);
        assert_eq!(KnowledgeAssetStatus::parse(expected).unwrap(), status);
    }
}

#[test]
fn unknown_status_is_rejected() {
    let error = KnowledgeAssetStatus::parse("processing").unwrap_err();
    assert!(error.to_string().contains("Unknown knowledge asset status"));
}

#[test]
fn serde_uses_snake_case_lifecycle_values() {
    let json = serde_json::to_string(&KnowledgeAssetStatus::EmbeddingPending).unwrap();
    assert_eq!(json, "\"embedding_pending\"");

    let decoded: KnowledgeAssetStatus = serde_json::from_str("\"ocr_ready\"").unwrap();
    assert_eq!(decoded, KnowledgeAssetStatus::OcrReady);
}

#[test]
fn only_review_stage_assets_accept_verified_ocr() {
    for status in [
        KnowledgeAssetStatus::Submitted,
        KnowledgeAssetStatus::OcrPending,
        KnowledgeAssetStatus::OcrReady,
        KnowledgeAssetStatus::Failed,
    ] {
        assert!(
            status.accepts_verified_ocr(),
            "{status:?} should accept OCR"
        );
    }

    for status in [
        KnowledgeAssetStatus::EmbeddingPending,
        KnowledgeAssetStatus::Embedded,
        KnowledgeAssetStatus::Published,
        KnowledgeAssetStatus::Archived,
    ] {
        assert!(
            !status.accepts_verified_ocr(),
            "{status:?} must reject a direct OCR write"
        );
    }
}

#[cfg(feature = "server")]
#[tokio::test]
async fn verified_ocr_cannot_revive_terminal_assets() {
    use crate::rls_context::{AuthorizedActor, AuthorizedTx};
    use sha2::{Digest, Sha256};
    use sqlx::postgres::PgPoolOptions;
    use uuid::Uuid;

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL is required for knowledge lifecycle tests");
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .expect("connect knowledge lifecycle database");
    let suffix = Uuid::new_v4().simple().to_string();
    let school_id = Uuid::new_v4();
    let actor_id = Uuid::new_v4();
    let submitted_asset = Uuid::new_v4();
    let ocr_ready_asset = Uuid::new_v4();
    let failed_asset = Uuid::new_v4();
    let archived_asset = Uuid::new_v4();
    let published_asset = Uuid::new_v4();
    let verified_hash = "a".repeat(64);
    let rejected_hash = "b".repeat(64);

    let platform_admin_role = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM roles WHERE name::text = 'PlatformAdmin' LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .expect("PlatformAdmin role fixture");
    sqlx::query("INSERT INTO schools (id, name) VALUES ($1, $2)")
        .bind(school_id)
        .bind(format!("Knowledge lifecycle school {suffix}"))
        .execute(&pool)
        .await
        .expect("insert school");
    sqlx::query(
        "INSERT INTO users (id, name, email, role_id, school_id, is_active, metadata) VALUES ($1, $2, $3, $4, $5, TRUE, '{}'::jsonb)",
    )
    .bind(actor_id)
    .bind("Knowledge lifecycle admin")
    .bind(format!("knowledge-lifecycle-{suffix}@example.test"))
    .bind(platform_admin_role)
    .bind(school_id)
    .execute(&pool)
    .await
    .expect("insert PlatformAdmin");
    for (asset_id, status) in [
        (submitted_asset, "submitted"),
        (ocr_ready_asset, "ocr_ready"),
        (failed_asset, "failed"),
        (archived_asset, "archived"),
        (published_asset, "published"),
    ] {
        sqlx::query(
            "INSERT INTO knowledge_assets (id, school_id, title, source_type, status, language, created_by, published_at) VALUES ($1, $2, $3, 'pdf', $4::knowledge_asset_status, 'en', $5, CASE WHEN $4 = 'published' THEN NOW() ELSE NULL END)",
        )
        .bind(asset_id)
        .bind(school_id)
        .bind(format!("Knowledge lifecycle asset {status} {suffix}"))
        .bind(status)
        .bind(actor_id)
        .execute(&pool)
        .await
        .expect("insert knowledge asset");
    }

    // Review-stage fixtures must follow the real governed pipeline: immutable
    // source revision -> protected byte review -> verified OCR. Do not fabricate
    // audit evidence or let OCR establish source provenance itself.
    let mut governed_sources = Vec::new();
    for asset_id in [submitted_asset, ocr_ready_asset, failed_asset] {
        let source_id = Uuid::new_v4();
        let source_bytes = format!("%PDF-lifecycle-fixture-{asset_id}").into_bytes();
        let source_sha = format!("{:x}", Sha256::digest(&source_bytes));
        sqlx::query(
            r#"
            INSERT INTO knowledge_source_files (
                id, asset_id, original_file_url, original_filename, mime_type,
                file_size_bytes, sha256, is_scanned_pdf
            ) VALUES ($1, $2, $3, 'fixture.pdf', 'application/pdf', $4, $5, FALSE)
            "#,
        )
        .bind(source_id)
        .bind(asset_id)
        .bind(format!(
            "storage://edutalent-knowledge-sources/{school_id}/{source_id}.pdf"
        ))
        .bind(i64::try_from(source_bytes.len()).expect("fixture size fits i64"))
        .bind(&source_sha)
        .execute(&pool)
        .await
        .expect("insert governed source revision");
        governed_sources.push((asset_id, source_id, source_bytes));
    }

    let mut review_tx = pool
        .begin()
        .await
        .expect("begin source review fixture transaction");
    sqlx::query("SELECT set_config('app.user_id', $1, true)")
        .bind(actor_id.to_string())
        .execute(&mut *review_tx)
        .await
        .expect("set review actor context");
    sqlx::query("SELECT set_config('app.user_role', 'PlatformAdmin', true)")
        .execute(&mut *review_tx)
        .await
        .expect("set review role context");
    sqlx::query("SELECT set_config('app.school_id', $1, true)")
        .bind(school_id.to_string())
        .execute(&mut *review_tx)
        .await
        .expect("set review school context");
    for (asset_id, source_id, source_bytes) in &governed_sources {
        sqlx::query_scalar::<_, Uuid>("SELECT record_knowledge_source_review($1, $2, $3)")
            .bind(asset_id)
            .bind(source_id)
            .bind(source_bytes)
            .fetch_one(&mut *review_tx)
            .await
            .expect("record trusted source review evidence");
    }
    review_tx
        .commit()
        .await
        .expect("commit source review fixture transaction");

    sqlx::query(
        "INSERT INTO knowledge_ocr_texts (asset_id, raw_text, clean_text, ocr_provider, ocr_verified_by, text_sha256) VALUES ($1, 'old text', 'old text', 'manual', $2, $3)",
    )
    .bind(ocr_ready_asset)
    .bind(actor_id)
    .bind(&rejected_hash)
    .execute(&pool)
    .await
    .expect("insert existing verified OCR");

    let actor = AuthorizedActor::new(actor_id, "PlatformAdmin", None).expect("valid actor");
    let transaction = AuthorizedTx::begin(&pool, actor.clone())
        .await
        .expect("begin RLS transaction");
    let outcome = transaction
        .scope(
            async {
                let repository = KnowledgeAssetRepository::new(());
                repository
                    .attach_verified_ocr(
                        submitted_asset,
                        "verified source",
                        "verified source",
                        "manual",
                        actor_id,
                        &verified_hash,
                        None,
                    )
                    .await
                    .expect("submitted asset accepts verified OCR");
                let existing_revision: Uuid = sqlx::query_scalar(
                    "SELECT revision FROM knowledge_ocr_texts WHERE asset_id = $1",
                )
                .bind(ocr_ready_asset)
                .fetch_one(&*repository.pool())
                .await
                .expect("read the current OCR revision");
                repository
                    .attach_verified_ocr(
                        ocr_ready_asset,
                        "corrected text",
                        "corrected text",
                        "manual",
                        actor_id,
                        &verified_hash,
                        Some(existing_revision),
                    )
                    .await
                    .expect("OCR-ready asset accepts a governed correction");
                repository
                    .attach_verified_ocr(
                        failed_asset,
                        "recovered text",
                        "recovered text",
                        "manual",
                        actor_id,
                        &verified_hash,
                        None,
                    )
                    .await
                    .expect("failed asset accepts a governed OCR recovery");
            },
            |_| true,
        )
        .await;
    outcome.expect("finish lifecycle authorization transaction");

    // A rejected nested write deliberately rolls back its own outer request
    // transaction. Run each direct stale/terminal invocation independently so
    // it cannot roll back the successful OCR verification above.
    for asset_id in [archived_asset, published_asset] {
        let transaction = AuthorizedTx::begin(&pool, actor.clone())
            .await
            .expect("begin terminal lifecycle transaction");
        let outcome = transaction
            .scope(
                async {
                    KnowledgeAssetRepository::new(())
                        .attach_verified_ocr(
                            asset_id,
                            "replacement",
                            "replacement",
                            "manual",
                            actor_id,
                            &rejected_hash,
                            None,
                        )
                        .await
                },
                |result| result.is_ok(),
            )
            .await
            .expect("finish terminal lifecycle transaction");
        assert!(matches!(outcome, Err(RepositoryError::Validation(_))));
    }

    let transaction = AuthorizedTx::begin(&pool, actor.clone())
        .await
        .expect("begin stale OCR revision transaction");
    let outcome = transaction
        .scope(
            async {
                KnowledgeAssetRepository::new(())
                    .attach_verified_ocr(
                        ocr_ready_asset,
                        "stale text",
                        "stale text",
                        "manual",
                        actor_id,
                        &rejected_hash,
                        Some(Uuid::new_v4()),
                    )
                    .await
            },
            |result| result.is_ok(),
        )
        .await
        .expect("finish stale OCR revision transaction");
    assert!(matches!(outcome, Err(RepositoryError::Validation(_))));

    let transaction = AuthorizedTx::begin(&pool, actor)
        .await
        .expect("begin persistence verification transaction");
    let outcome = transaction
        .scope(
            async {
                let repository = KnowledgeAssetRepository::new(());
                for (asset_id, expected_status) in [
                    (submitted_asset, KnowledgeAssetStatus::OcrReady),
                    (ocr_ready_asset, KnowledgeAssetStatus::OcrReady),
                    (failed_asset, KnowledgeAssetStatus::OcrReady),
                    (archived_asset, KnowledgeAssetStatus::Archived),
                    (published_asset, KnowledgeAssetStatus::Published),
                ] {
                    let asset = repository
                        .find_by_id(asset_id)
                        .await
                        .expect("read persisted lifecycle state in authorized scope");
                    assert_eq!(asset.status, expected_status);
                }
                let terminal_ocr_rows: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM knowledge_ocr_texts WHERE asset_id = ANY($1)",
                )
                .bind(vec![archived_asset, published_asset])
                .fetch_one(&*repository.pool())
                .await
                .expect("count terminal OCR rows in authorized scope");
                assert_eq!(terminal_ocr_rows, 0);
                let corrected_text: String = sqlx::query_scalar(
                    "SELECT clean_text FROM knowledge_ocr_texts WHERE asset_id = $1",
                )
                .bind(ocr_ready_asset)
                .fetch_one(&*repository.pool())
                .await
                .expect("read corrected verified OCR in authorized scope");
                assert_eq!(corrected_text, "corrected text");
            },
            |_| true,
        )
        .await;
    outcome.expect("finish persistence verification transaction");
}
