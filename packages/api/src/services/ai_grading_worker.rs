//! Durable AI grading executor. Disabled unless explicitly enabled
//! via AI_GRADING_WORKER_ENABLED=true. Queued work contains no student content:
//! global scheduler transactions see only lease metadata; AI inputs and
//! persisted suggestions are read/written under a fresh Teacher AuthorizedTx.
//! This worker never publishes, accepts, or alters an academic grade.

use crate::rls_context::{AuthorizedActor, AuthorizedPool, AuthorizedTx, RlsContextError};
use crate::server_functions::ai_grading_suggestion_functions::generate_authorized_teacher_suggestion;
use dioxus::prelude::ServerFnError;
use sqlx::{PgPool, Row};
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
struct ClaimedJob {
    id: Uuid,
    school_id: Uuid,
    submission_id: Uuid,
    revision: Uuid,
    rubric_approval_id: Uuid,
    attempts: i32,
}

#[derive(Debug)]
enum ExecutionError {
    Database(sqlx::Error),
    Grading(ServerFnError),
    InvalidClaim,
}

impl From<sqlx::Error> for ExecutionError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Failure {
    Cancel(&'static str),
    Retry(&'static str, bool, i32),
}

fn classify(error: &ExecutionError) -> Failure {
    match error {
        ExecutionError::InvalidClaim => Failure::Cancel("stale_submission"),
        ExecutionError::Database(_) => Failure::Retry("database_unavailable", true, 10),
        ExecutionError::Grading(error) => {
            let value = error.to_string();
            if value.contains("grading.stale_submission") || value.contains("grading.not_found") {
                Failure::Cancel("stale_submission")
            } else if value.contains("grading.rubric_revoked")
                || value.contains("grading.rubric_conflict")
                || value.contains("grading.rubric_approval_required")
            {
                Failure::Cancel("rubric_revoked")
            } else if value.contains("grading.pdf_budget_exhausted") {
                // The operator must explicitly adjust a hard budget.
                // A bounded automatic retry would only consume more money.
                Failure::Retry("pdf_budget_exhausted", false, 5)
            } else if value.contains("grading.pdf_school_disabled") {
                Failure::Cancel("authorization_revoked")
            } else if value.contains("grading.text_required") {
                Failure::Cancel("attachment_unsupported")
            } else if value.contains("grading.invalid_suggestion")
                || value.contains("grading.invalid_rubric")
            {
                Failure::Retry("invalid_model_response", false, 5)
            } else {
                Failure::Retry("gateway_unavailable", true, 30)
            }
        }
    }
}

fn enabled(value: Option<&str>) -> bool {
    matches!(value, Some("true" | "1"))
}

async fn scoped<T, E, F>(
    raw: &PgPool,
    actor: AuthorizedActor,
    operation: F,
) -> Result<Result<T, E>, RlsContextError>
where
    F: Future<Output = Result<T, E>>,
{
    let transaction = AuthorizedTx::begin(raw, actor).await?;
    transaction.scope(operation, Result::is_ok).await
}

async fn claim(pool: &AuthorizedPool, worker: Uuid) -> Result<Option<ClaimedJob>, sqlx::Error> {
    let row = sqlx::query("SELECT * FROM public.claim_next_ai_grading_job($1)")
        .bind(worker)
        .fetch_optional(pool)
        .await?;
    row.map(|row| {
        Ok(ClaimedJob {
            id: row.try_get("job_id")?,
            school_id: row.try_get("school_id")?,
            submission_id: row.try_get("submission_id")?,
            revision: row.try_get("submission_revision")?,
            rubric_approval_id: row.try_get("rubric_approval_id")?,
            attempts: row.try_get("attempt_count")?,
        })
    })
    .transpose()
}

/// Each inference executes with the current Teacher's RLS and is rejected
/// if the claimed immutable revision or approval no longer matches.
async fn generate(
    pool: &AuthorizedPool,
    job: ClaimedJob,
    teacher_id: Uuid,
) -> Result<Uuid, ExecutionError> {
    let authorized: bool = sqlx::query_scalar(
        "SELECT EXISTS ( \
            SELECT 1 FROM public.ai_grading_rubric_approvals approval \
            JOIN public.submissions submission \
              ON submission.custom_assignment_id=approval.custom_assignment_id \
            WHERE submission.id=$1 AND submission.revision=$2 \
              AND approval.id=$3 AND approval.school_id=$4 \
              AND approval.revoked_at IS NULL \
        )",
    )
    .bind(job.submission_id)
    .bind(job.revision)
    .bind(job.rubric_approval_id)
    .bind(job.school_id)
    .fetch_one(pool)
    .await?;
    if !authorized {
        return Err(ExecutionError::InvalidClaim);
    }
    // A queued PDF job can outlive a school opt-in revocation. The queue
    // metadata itself is not proof of current permission to process a PDF.
    // Check before fetching originals/inference and again before completing
    // the lease. Text-only jobs retain their existing behavior.
    let has_original: bool = sqlx::query_scalar(
        "SELECT EXISTS ( \
            SELECT 1 FROM public.submission_attachments attachment \
            JOIN public.submissions submission \
              ON submission.custom_assignment_id=attachment.custom_assignment_id \
            WHERE submission.id=$1 AND attachment.school_id=$2 \
              AND attachment.student_id=submission.student_id \
              AND attachment.status <> 'removed' \
        )",
    )
    .bind(job.submission_id)
    .bind(job.school_id)
    .fetch_one(pool)
    .await?;
    if has_original
        && !crate::services::ai_grading_pdf_source::school_pdf_grading_enabled(pool, job.school_id)
            .await?
    {
        return Err(ExecutionError::Grading(ServerFnError::new(
            "grading.pdf_school_disabled",
        )));
    }
    let suggestion = generate_authorized_teacher_suggestion(pool, job.submission_id, teacher_id)
        .await
        .map_err(ExecutionError::Grading)?;
    if has_original
        && !crate::services::ai_grading_pdf_source::school_pdf_grading_enabled(pool, job.school_id)
            .await?
    {
        return Err(ExecutionError::Grading(ServerFnError::new(
            "grading.pdf_school_disabled",
        )));
    }
    if suggestion.submission_id != job.submission_id
        || suggestion.rubric_approval_id != job.rubric_approval_id
        || suggestion.suggestion.submission_revision != job.revision
    {
        return Err(ExecutionError::InvalidClaim);
    }
    Ok(suggestion.id)
}

async fn finish(
    raw: &PgPool,
    pool: &AuthorizedPool,
    worker: Uuid,
    job: ClaimedJob,
    suggestion_id: Uuid,
) {
    let completed = scoped(
        raw,
        AuthorizedActor::system_queue(worker),
        sqlx::query_scalar::<_, bool>("SELECT public.complete_ai_grading_job($1,$2,$3)")
            .bind(worker)
            .bind(job.id)
            .bind(suggestion_id)
            .fetch_one(pool),
    )
    .await;
    match completed {
        Ok(Ok(true)) => tracing::info!(
            job_id=%job.id,
            "Durable AI suggestion is awaiting teacher review"
        ),
        _ => tracing::warn!(
            job_id=%job.id,
            "AI grading completion was not committed; lease recovery is available"
        ),
    }
}

async fn fail(raw: &PgPool, pool: &AuthorizedPool, worker: Uuid, job: ClaimedJob, reason: Failure) {
    let result = match reason {
        Failure::Cancel(code) => scoped(
            raw,
            AuthorizedActor::system_queue(worker),
            sqlx::query_scalar::<_, bool>("SELECT public.cancel_ai_grading_job($1,$2,$3)")
                .bind(worker)
                .bind(job.id)
                .bind(code)
                .fetch_one(pool),
        )
        .await
        .map(|v| v.map(|b| b.to_string())),
        Failure::Retry(code, retryable, after) => scoped(
            raw,
            AuthorizedActor::system_queue(worker),
            sqlx::query_scalar::<_, Option<String>>(
                "SELECT public.record_ai_grading_job_failure($1,$2,$3,$4,$5)",
            )
            .bind(worker)
            .bind(job.id)
            .bind(code)
            .bind(retryable)
            .bind(after)
            .fetch_one(pool),
        )
        .await
        .map(|v| v.map(|s| s.unwrap_or_default())),
    };
    match result {
        Ok(Ok(_)) => tracing::warn!(
            job_id=%job.id,
            attempt=job.attempts,
            reason=?reason,
            "AI grading job cancelled or recorded for bounded retry"
        ),
        _ => tracing::error!(
            job_id=%job.id,
            "Could not record grading job failure; lease recovery will retry"
        ),
    }
}

/// Start the grading worker. Deployment must explicitly opt in after
/// reviewing model costs and quality; a failed gateway cannot publish grades.
pub fn start_ai_grading_worker(
    raw: Arc<PgPool>,
    pool: Arc<AuthorizedPool>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if !enabled(std::env::var("AI_GRADING_WORKER_ENABLED").ok().as_deref()) {
            tracing::info!("AI grading worker disabled; enable after review");
            return;
        }
        let worker = Uuid::new_v4();
        let poll_ms = std::env::var("AI_GRADING_WORKER_POLL_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(1500)
            .clamp(250, 10000);
        let poll = Duration::from_millis(poll_ms);
        let recovery_interval = Duration::from_secs(60);
        let mut last_recovery = Instant::now() - recovery_interval;

        loop {
            if last_recovery.elapsed() >= recovery_interval {
                let recovery = scoped(
                    raw.as_ref(),
                    AuthorizedActor::system_queue(worker),
                    sqlx::query_scalar::<_, i64>(
                        "SELECT public.recover_expired_ai_grading_jobs($1)",
                    )
                    .bind(worker)
                    .fetch_one(pool.as_ref()),
                )
                .await;
                if !matches!(recovery, Ok(Ok(_))) {
                    tracing::warn!("Bounded AI grading lease recovery failed");
                }
                last_recovery = Instant::now();
            }
            let pending = scoped(
                raw.as_ref(),
                AuthorizedActor::system_queue(worker),
                claim(pool.as_ref(), worker),
            )
            .await;
            let job = match pending {
                Ok(Ok(Some(job))) => job,
                Ok(Ok(None)) => {
                    tokio::time::sleep(poll).await;
                    continue;
                }
                _ => {
                    tracing::warn!("Bounded AI grading claim failed");
                    tokio::time::sleep(poll).await;
                    continue;
                }
            };

            let resolved = scoped(
                raw.as_ref(),
                AuthorizedActor::system_queue(worker),
                sqlx::query_scalar::<_, Option<Uuid>>(
                    "SELECT public.resolve_ai_grading_job_teacher($1,$2)",
                )
                .bind(worker)
                .bind(job.id)
                .fetch_one(pool.as_ref()),
            )
            .await;
            let teacher = match resolved {
                Ok(Ok(Some(id))) if !id.is_nil() => id,
                Ok(Ok(_)) => {
                    fail(
                        raw.as_ref(),
                        pool.as_ref(),
                        worker,
                        job,
                        Failure::Cancel("authorization_revoked"),
                    )
                    .await;
                    continue;
                }
                _ => {
                    fail(
                        raw.as_ref(),
                        pool.as_ref(),
                        worker,
                        job,
                        Failure::Retry("database_unavailable", true, 10),
                    )
                    .await;
                    continue;
                }
            };

            // The request timeout is bounded by the AI Gateway. A separate
            // system-job heartbeat keeps a valid lease across network waits.
            let (stop, mut receiver) = tokio::sync::watch::channel(false);
            let raw_heartbeat = Arc::clone(&raw);
            let pool_heartbeat = Arc::clone(&pool);
            let job_id = job.id;
            let pulse = tokio::spawn(async move {
                let mut ticker = tokio::time::interval(Duration::from_secs(30));
                ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    tokio::select! {
                        _ = ticker.tick() => {
                            let refreshed = scoped(
                                raw_heartbeat.as_ref(),
                                AuthorizedActor::system_queue(worker),
                                sqlx::query_scalar::<_, bool>(
                                    "SELECT public.heartbeat_ai_grading_job($1,$2)"
                                ).bind(worker).bind(job_id).fetch_one(pool_heartbeat.as_ref()),
                            ).await;
                            if !matches!(refreshed, Ok(Ok(true))) {
                                tracing::warn!(%job_id, "AI grading lease heartbeat failed");
                            }
                        }
                        changed = receiver.changed() => {
                            if changed.is_err() || *receiver.borrow() {
                                break;
                            }
                        }
                    }
                }
            });
            let attempt = scoped(
                raw.as_ref(),
                AuthorizedActor::new(teacher, "Teacher", Some(job.school_id))
                    .expect("validated canonical Teacher actor"),
                generate(pool.as_ref(), job, teacher),
            )
            .await;
            let _ = stop.send(true);
            let _ = pulse.await;
            match attempt {
                Ok(Ok(suggestion_id)) => {
                    finish(raw.as_ref(), pool.as_ref(), worker, job, suggestion_id).await;
                }
                Ok(Err(error)) => {
                    fail(raw.as_ref(), pool.as_ref(), worker, job, classify(&error)).await;
                }
                Err(_) => {
                    fail(
                        raw.as_ref(),
                        pool.as_ref(),
                        worker,
                        job,
                        Failure::Retry("database_unavailable", true, 10),
                    )
                    .await;
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_does_not_auto_enable_or_publish() {
        assert!(!enabled(None));
        assert!(!enabled(Some("false")));
        assert!(!enabled(Some("TRUE")));
        assert!(enabled(Some("true")));
        assert!(enabled(Some("1")));
        let code = include_str!("ai_grading_worker.rs");
        let production = code.split("#[cfg(test)]").next().unwrap_or("");
        assert!(!production.contains("UPDATE public.submissions SET grade"));
        assert!(!production.contains("UPDATE submissions SET grade"));
    }

    #[test]
    fn stale_and_invalid_model_fail_closed() {
        assert_eq!(
            classify(&ExecutionError::InvalidClaim),
            Failure::Cancel("stale_submission")
        );
        assert_eq!(
            classify(&ExecutionError::Grading(ServerFnError::new(
                "grading.rubric_revoked"
            ))),
            Failure::Cancel("rubric_revoked")
        );
        assert_eq!(
            classify(&ExecutionError::Grading(ServerFnError::new(
                "grading.invalid_suggestion"
            ))),
            Failure::Retry("invalid_model_response", false, 5)
        );
        assert_eq!(
            classify(&ExecutionError::Grading(ServerFnError::new(
                "grading.pdf_school_disabled"
            ))),
            Failure::Cancel("authorization_revoked")
        );
        assert_eq!(
            classify(&ExecutionError::Grading(ServerFnError::new(
                "grading.pdf_budget_exhausted"
            ))),
            Failure::Retry("pdf_budget_exhausted", false, 5)
        );
    }
}
