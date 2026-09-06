//! Private submission attachment contract. Storage keys never cross this boundary.
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MAX_ATTACHMENT_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_SUBMISSION_FILE_BYTES: usize = 25 * 1024 * 1024;
pub const MAX_SUBMISSION_FILES: usize = 5;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentStatus {
    Pending,
    Ready,
    Submitted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubmissionAttachment {
    pub id: Uuid,
    pub filename: String,
    pub media_type: String,
    pub byte_size: i64,
    pub sha256: String,
    pub status: AttachmentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AttachmentIntent {
    pub assignment_id: Uuid,
    pub request_id: Uuid,
    pub filename: String,
    pub media_type: String,
    pub byte_size: i64,
    pub sha256: String,
}

#[post("/api/submissions/attachments/reserve")]
pub async fn reserve_submission_attachment(
    input: AttachmentIntent,
) -> Result<SubmissionAttachment, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (pool, user) = context().await?;
        crate::repositories::submission_attachment_repository::reserve(&pool, &user, input)
            .await
            .map_err(product_error)
    }
    #[cfg(not(feature = "server"))]
    {
        unreachable!()
    }
}

#[get("/api/submissions/attachments/list")]
pub async fn list_submission_attachments(
    assignment_id: Uuid,
) -> Result<Vec<SubmissionAttachment>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (pool, user) = context().await?;
        crate::repositories::submission_attachment_repository::list(&pool, &user, assignment_id)
            .await
            .map_err(product_error)
    }
    #[cfg(not(feature = "server"))]
    {
        unreachable!()
    }
}

#[post("/api/submissions/attachments/remove")]
pub async fn remove_submission_attachment(attachment_id: Uuid) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (pool, user) = context().await?;
        crate::repositories::submission_attachment_repository::remove(&pool, &user, attachment_id)
            .await
            .map_err(product_error)
    }
    #[cfg(not(feature = "server"))]
    {
        unreachable!()
    }
}

#[cfg(feature = "server")]
async fn context() -> Result<
    (
        std::sync::Arc<crate::rls_context::AuthorizedPool>,
        crate::domain::UserInfo,
    ),
    ServerFnError,
> {
    let (user, pool) = crate::server_functions::rls_helpers::extract_user().await?;
    Ok((pool, user))
}

#[cfg(feature = "server")]
fn product_error(
    error: crate::repositories::submission_attachment_repository::AttachmentError,
) -> ServerFnError {
    tracing::warn!(%error, "submission attachment request rejected");
    ServerFnError::new(error.code())
}
