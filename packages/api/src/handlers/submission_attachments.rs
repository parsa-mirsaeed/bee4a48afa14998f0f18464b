use crate::repositories::submission_attachment_repository::{self as repository, AttachmentError};
use crate::services::submission_attachment_storage as storage;
use crate::{app_state::AppState, domain::UserInfo, rls_context::AuthorizedPool};
use axum::{
    body::{Body, Bytes},
    extract::Query,
    http::{HeaderMap, StatusCode},
    response::{Json, Response},
    Extension,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct AttachmentQuery {
    pub id: Uuid,
}
type Rejection = (StatusCode, Json<Value>);
fn reject(error: AttachmentError) -> Rejection {
    tracing::warn!(%error,"private submission attachment request failed");
    let status = match error {
        AttachmentError::Forbidden => StatusCode::FORBIDDEN,
        AttachmentError::Invalid => StatusCode::UNSUPPORTED_MEDIA_TYPE,
        AttachmentError::Limit => StatusCode::PAYLOAD_TOO_LARGE,
        AttachmentError::Conflict => StatusCode::CONFLICT,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    };
    (status, Json(json!({"error":error.code()})))
}

pub async fn upload_submission_attachment(
    Extension(state): Extension<AppState>,
    Extension(user): Extension<UserInfo>,
    Extension(pool): Extension<Arc<AuthorizedPool>>,
    Query(query): Query<AttachmentQuery>,
    headers: HeaderMap,
    bytes: Bytes,
) -> Result<Json<Value>, Rejection> {
    let row = sqlx::query("SELECT custom_assignment_id FROM submission_attachments WHERE id=$1")
        .bind(query.id)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| reject(e.into()))?
        .ok_or_else(|| reject(AttachmentError::Forbidden))?;
    let assignment: Uuid = row.get("custom_assignment_id");
    let (_, school) = repository::authorize_student(&pool, &user, assignment, true)
        .await
        .map_err(reject)?;
    let row = sqlx::query("SELECT * FROM submission_attachments WHERE id=$1 FOR UPDATE")
        .bind(query.id)
        .fetch_one(&*pool)
        .await
        .map_err(|e| reject(e.into()))?;
    let attachment = repository::from_row(&row).map_err(reject)?;
    if attachment.status
        == crate::server_functions::submission_attachment_functions::AttachmentStatus::Submitted
    {
        return Err(reject(AttachmentError::Conflict));
    }
    if headers.get("content-type").and_then(|v| v.to_str().ok())
        != Some(attachment.media_type.as_str())
        || bytes.len() as i64 != attachment.byte_size
        || storage::hash(&bytes) != attachment.sha256
    {
        return Err(reject(AttachmentError::Invalid));
    }
    let media = attachment.media_type.clone();
    let original = bytes.to_vec();
    let original = tokio::task::spawn_blocking(move || {
        storage::validate_bytes(&media, &original)?;
        Ok::<_, AttachmentError>(original)
    })
    .await
    .map_err(|_| reject(AttachmentError::Invalid))?
    .map_err(reject)?;
    storage::put_verified(
        &state,
        &repository::object_key(school, query.id),
        &attachment.media_type,
        original,
        &attachment.sha256,
    )
    .await
    .map_err(reject)?;
    let changed=sqlx::query("UPDATE submission_attachments SET status='ready',verified_at=COALESCE(verified_at,NOW()) WHERE id=$1 AND status IN ('pending','ready')")
        .bind(query.id).execute(&*pool).await.map_err(|e|reject(e.into()))?;
    if changed.rows_affected() != 1 {
        return Err(reject(AttachmentError::Conflict));
    }
    Ok(Json(json!({"id":query.id,"status":"ready"})))
}

pub async fn download_submission_attachment(
    Extension(state): Extension<AppState>,
    Extension(user): Extension<UserInfo>,
    Extension(pool): Extension<Arc<AuthorizedPool>>,
    Query(query): Query<AttachmentQuery>,
) -> Result<Response, Rejection> {
    if !matches!(user.role.as_str(), "Student" | "Teacher") {
        return Err(reject(AttachmentError::Forbidden));
    }
    let row = sqlx::query(
        "SELECT * FROM submission_attachments WHERE id=$1 AND status IN ('ready','submitted')",
    )
    .bind(query.id)
    .fetch_optional(&*pool)
    .await
    .map_err(|e| reject(e.into()))?
    .ok_or_else(|| reject(AttachmentError::Forbidden))?;
    let school: Uuid = row.get("school_id");
    let attachment = repository::from_row(&row).map_err(reject)?;
    let bytes = storage::read(&state, &repository::object_key(school, query.id))
        .await
        .map_err(reject)?;
    if bytes.len() as i64 != attachment.byte_size || storage::hash(&bytes) != attachment.sha256 {
        return Err(reject(AttachmentError::Storage));
    }
    let filename = match attachment.media_type.as_str() {
        "application/pdf" => "attachment; filename=homework.pdf",
        "image/jpeg" => "attachment; filename=homework.jpg",
        "image/png" => "attachment; filename=homework.png",
        _ => return Err(reject(AttachmentError::Invalid)),
    };
    Response::builder()
        .header("content-type", attachment.media_type)
        .header("content-disposition", filename)
        .header("cache-control", "private, no-store, max-age=0")
        .header("x-content-type-options", "nosniff")
        .header("content-security-policy", "sandbox; default-src 'none'")
        .body(Body::from(bytes))
        .map_err(|_| reject(AttachmentError::Storage))
}
