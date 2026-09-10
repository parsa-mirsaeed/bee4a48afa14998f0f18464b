//! Retriable deletion of abandoned/removed intents. Submitted originals are
//! excluded by both the scheduler and the immutable-original trigger.
use crate::repositories::submission_attachment_repository::object_key;
use crate::services::submission_attachment_storage;
use crate::{
    app_state::AppState,
    rls_context::{AuthorizedActor, AuthorizedTx},
};
use sqlx::Row;
use uuid::Uuid;

pub fn start_submission_attachment_cleanup(state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let worker = Uuid::new_v4();
        loop {
            let result = cleanup_one(&state, worker).await;
            let delay = match result {
                Ok(true) => 1,
                Ok(false) => 10,
                Err(error) => {
                    tracing::warn!(%error,"submission attachment cleanup will retry");
                    60
                }
            };
            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
        }
    })
}
async fn cleanup_one(
    state: &AppState,
    worker: Uuid,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let tx = AuthorizedTx::begin(
        &state.services.raw_pool,
        AuthorizedActor::system_queue(worker),
    )
    .await?;
    let claim=tx.scope(async {
        sqlx::query("SELECT attachment_id,school_id FROM edutalent_internal.claim_submission_attachment_cleanup()")
            .fetch_optional(&*state.services.pool).await
    },Result::is_ok).await??;
    let Some(row) = claim else { return Ok(false) };
    let id: Uuid = row.try_get("attachment_id")?;
    let school: Uuid = row.try_get("school_id")?;
    // Claim/tombstone was committed before remote deletion. Failures retain it
    // for retry after five minutes; a crash cannot lose cleanup ownership.
    submission_attachment_storage::delete(state, &object_key(school, id)).await?;
    let tx = AuthorizedTx::begin(
        &state.services.raw_pool,
        AuthorizedActor::system_job(worker, school),
    )
    .await?;
    tx.scope(async {
        // Recheck tombstones daily: a timed-out remote PUT may complete after a
        // DELETE. Repeated idempotent deletion prevents such a late object from
        // surviving indefinitely. Metadata tombstones remain for retry identity.
        sqlx::query("UPDATE submission_attachments SET cleaned_at=NOW(),cleanup_after=NOW()+INTERVAL '1 day' WHERE id=$1 AND status='removed'")
            .bind(id).execute(&*state.services.pool).await
    },Result::is_ok).await??;
    Ok(true)
}
