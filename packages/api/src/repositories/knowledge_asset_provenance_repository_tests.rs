#[cfg(feature = "server")]
use super::{KnowledgeAssetRepository, Repository};

#[cfg(feature = "server")]
#[tokio::test]
async fn replacement_source_reverification_rebinds_stale_ocr_with_new_revision() {
    use crate::rls_context::{AuthorizedActor, AuthorizedTx};
    use sha2::{Digest, Sha256};
    use sqlx::{postgres::PgPoolOptions, Row};
    use uuid::Uuid;

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL is required for knowledge provenance tests");
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .expect("connect knowledge provenance database");

    let school_id = Uuid::new_v4();
    let manager_id = Uuid::new_v4();
    let admin_id = Uuid::new_v4();
    let asset_id = Uuid::new_v4();
    let source_one = Uuid::new_v4();
    let replacement_object = Uuid::new_v4();
    let suffix = Uuid::new_v4().simple().to_string();
    let source_one_bytes = b"%PDF-1.4\ninitial governed source\n%%EOF\n".to_vec();
    let source_two_bytes = b"%PDF-1.4\nreplacement governed source\n%%EOF\n".to_vec();
    let source_one_sha = format!("{:x}", Sha256::digest(&source_one_bytes));
    let source_two_sha = format!("{:x}", Sha256::digest(&source_two_bytes));

    let manager_role: Uuid =
        sqlx::query_scalar("SELECT id FROM roles WHERE name::text = 'SchoolManager' LIMIT 1")
            .fetch_one(&pool)
            .await
            .expect("SchoolManager role fixture");
    let admin_role: Uuid =
        sqlx::query_scalar("SELECT id FROM roles WHERE name::text = 'PlatformAdmin' LIMIT 1")
            .fetch_one(&pool)
            .await
            .expect("PlatformAdmin role fixture");

    sqlx::query("INSERT INTO schools (id, name) VALUES ($1, $2)")
        .bind(school_id)
        .bind(format!("Knowledge provenance school {suffix}"))
        .execute(&pool)
        .await
        .expect("insert provenance school");
    sqlx::query(
        r#"
        INSERT INTO users (id, name, email, role_id, school_id, is_active, metadata)
        VALUES
            ($1, 'Knowledge provenance manager', $2, $3, $4, TRUE, '{}'::jsonb),
            ($5, 'Knowledge provenance admin', $6, $7, $4, TRUE, '{}'::jsonb)
        "#,
    )
    .bind(manager_id)
    .bind(format!(
        "knowledge-provenance-manager-{suffix}@example.test"
    ))
    .bind(manager_role)
    .bind(school_id)
    .bind(admin_id)
    .bind(format!("knowledge-provenance-admin-{suffix}@example.test"))
    .bind(admin_role)
    .execute(&pool)
    .await
    .expect("insert provenance actors");
    sqlx::query(
        r#"
        INSERT INTO knowledge_assets (
            id, school_id, title, source_type, status, language, created_by
        ) VALUES ($1, $2, 'Source replacement OCR fixture', 'pdf', 'submitted', 'en', $3)
        "#,
    )
    .bind(asset_id)
    .bind(school_id)
    .bind(manager_id)
    .execute(&pool)
    .await
    .expect("insert provenance asset");

    let mut bootstrap = pool.begin().await.expect("begin provenance bootstrap");
    sqlx::query("SELECT set_config('app.user_id', $1, true)")
        .bind(admin_id.to_string())
        .execute(&mut *bootstrap)
        .await
        .expect("set bootstrap admin actor");
    sqlx::query("SELECT set_config('app.user_role', 'PlatformAdmin', true)")
        .execute(&mut *bootstrap)
        .await
        .expect("set bootstrap admin role");
    sqlx::query("SELECT set_config('app.school_id', $1, true)")
        .bind(school_id.to_string())
        .execute(&mut *bootstrap)
        .await
        .expect("set bootstrap school");
    sqlx::query("SELECT set_config('app.elevated_operation', 'false', true)")
        .execute(&mut *bootstrap)
        .await
        .expect("set bootstrap non-elevated context");
    sqlx::query(
        r#"
        INSERT INTO knowledge_source_files (
            id, asset_id, original_file_url, original_filename, mime_type,
            file_size_bytes, sha256, is_scanned_pdf
        ) VALUES ($1, $2, $3, 'initial.pdf', 'application/pdf', $4, $5, FALSE)
        "#,
    )
    .bind(source_one)
    .bind(asset_id)
    .bind(format!(
        "storage://edutalent-knowledge-sources/{school_id}/{source_one}.pdf"
    ))
    .bind(i64::try_from(source_one_bytes.len()).expect("initial bytes fit i64"))
    .bind(&source_one_sha)
    .execute(&mut *bootstrap)
    .await
    .expect("insert initial governed source");
    sqlx::query_scalar::<_, Uuid>("SELECT record_knowledge_source_review($1, $2, $3)")
        .bind(asset_id)
        .bind(source_one)
        .bind(&source_one_bytes)
        .fetch_one(&mut *bootstrap)
        .await
        .expect("review initial governed source");
    let initial_ocr_revision = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO knowledge_ocr_texts (
            asset_id, raw_text, clean_text, ocr_provider, ocr_verified_by,
            text_sha256, revision
        ) VALUES ($1, 'initial OCR', 'initial OCR', 'manual', $2, $3, $4)
        "#,
    )
    .bind(asset_id)
    .bind(admin_id)
    .bind(format!("{:x}", Sha256::digest(b"initial OCR")))
    .bind(initial_ocr_revision)
    .execute(&mut *bootstrap)
    .await
    .expect("insert initial verified OCR");
    sqlx::query("UPDATE knowledge_assets SET status = 'ocr_ready' WHERE id = $1")
        .bind(asset_id)
        .execute(&mut *bootstrap)
        .await
        .expect("advance initial asset to OCR ready");
    bootstrap
        .commit()
        .await
        .expect("commit provenance bootstrap");

    let expected_asset_revision: i64 =
        sqlx::query_scalar("SELECT asset_revision FROM knowledge_assets WHERE id = $1")
            .bind(asset_id)
            .fetch_one(&pool)
            .await
            .expect("read asset revision before replacement");

    let manager_actor = AuthorizedActor::new(manager_id, "SchoolManager", Some(school_id))
        .expect("valid SchoolManager actor");
    let manager_tx = AuthorizedTx::begin(&pool, manager_actor)
        .await
        .expect("begin manager source replacement transaction");
    let replacement_source = manager_tx
        .scope(
            async {
                let repository = KnowledgeAssetRepository::new(());
                let row = sqlx::query(
                    r#"
                    SELECT source_file_id
                    FROM manager_replace_knowledge_source_revision(
                        $1, $2, $3, 'replacement.pdf', 'application/pdf', $4, $5, NULL, FALSE
                    )
                    "#,
                )
                .bind(asset_id)
                .bind(expected_asset_revision)
                .bind(format!(
                    "storage://edutalent-knowledge-sources/{school_id}/{replacement_object}.pdf"
                ))
                .bind(i64::try_from(source_two_bytes.len()).expect("replacement bytes fit i64"))
                .bind(&source_two_sha)
                .fetch_one(&*repository.pool())
                .await
                .expect("replace governed source");
                row.try_get::<Uuid, _>("source_file_id")
                    .expect("decode replacement source id")
            },
            |_| true,
        )
        .await
        .expect("finish manager source replacement transaction");

    let stale_row = sqlx::query(
        r#"
        SELECT asset.status::text AS status,
               asset.current_source_file_id,
               ocr.source_file_id,
               ocr.revision
        FROM knowledge_assets AS asset
        JOIN knowledge_ocr_texts AS ocr ON ocr.asset_id = asset.id
        WHERE asset.id = $1
        "#,
    )
    .bind(asset_id)
    .fetch_one(&pool)
    .await
    .expect("read stale OCR after source replacement");
    assert_eq!(
        stale_row.try_get::<String, _>("status").unwrap(),
        "ocr_pending"
    );
    assert_eq!(
        stale_row
            .try_get::<Uuid, _>("current_source_file_id")
            .unwrap(),
        replacement_source
    );
    assert_eq!(
        stale_row
            .try_get::<Option<Uuid>, _>("source_file_id")
            .unwrap(),
        Some(source_one)
    );
    assert_eq!(
        stale_row.try_get::<Uuid, _>("revision").unwrap(),
        initial_ocr_revision
    );

    let current_ocr_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
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
    .fetch_one(&pool)
    .await
    .expect("count current OCR after source replacement");
    assert_eq!(current_ocr_count, 0);

    let admin_actor =
        AuthorizedActor::new(admin_id, "PlatformAdmin", None).expect("valid PlatformAdmin actor");
    let admin_tx = AuthorizedTx::begin(&pool, admin_actor)
        .await
        .expect("begin replacement OCR review transaction");
    admin_tx
        .scope(
            async {
                let repository = KnowledgeAssetRepository::new(());
                sqlx::query_scalar::<_, Uuid>("SELECT record_knowledge_source_review($1, $2, $3)")
                    .bind(asset_id)
                    .bind(replacement_source)
                    .bind(&source_two_bytes)
                    .fetch_one(&*repository.pool())
                    .await
                    .expect("review replacement governed source");

                repository
                    .attach_verified_ocr_for_source(
                        asset_id,
                        "replacement OCR",
                        "replacement OCR",
                        "manual",
                        admin_id,
                        &format!("{:x}", Sha256::digest(b"replacement OCR")),
                        replacement_source,
                        &source_two_sha,
                        None,
                    )
                    .await
                    .expect("reverify OCR against replacement source");
            },
            |_| true,
        )
        .await
        .expect("finish replacement OCR review transaction");

    let rebound = sqlx::query(
        r#"
        SELECT asset.status::text AS status,
               ocr.source_file_id,
               lower(ocr.source_sha256) AS source_sha256,
               ocr.revision,
               ocr.clean_text
        FROM knowledge_assets AS asset
        JOIN knowledge_ocr_texts AS ocr ON ocr.asset_id = asset.id
        WHERE asset.id = $1
        "#,
    )
    .bind(asset_id)
    .fetch_one(&pool)
    .await
    .expect("read rebound OCR");
    let rebound_revision: Uuid = rebound
        .try_get("revision")
        .expect("decode rebound revision");
    assert_eq!(rebound.try_get::<String, _>("status").unwrap(), "ocr_ready");
    assert_eq!(
        rebound
            .try_get::<Option<Uuid>, _>("source_file_id")
            .unwrap(),
        Some(replacement_source)
    );
    assert_eq!(
        rebound
            .try_get::<Option<String>, _>("source_sha256")
            .unwrap(),
        Some(source_two_sha)
    );
    assert_ne!(rebound_revision, initial_ocr_revision);
    assert_eq!(
        rebound.try_get::<String, _>("clean_text").unwrap(),
        "replacement OCR"
    );

    let old_provenance: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_ocr_revision_provenance WHERE asset_id = $1 AND revision = $2",
    )
    .bind(asset_id)
    .bind(initial_ocr_revision)
    .fetch_one(&pool)
    .await
    .expect("count preserved old OCR provenance");
    assert_eq!(old_provenance, 1);
}
