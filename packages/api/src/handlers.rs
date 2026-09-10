use axum::{extract::Path, http::StatusCode, response::Json, Extension};
use serde_json::{json, Value};

// Auth module
pub mod auth;
pub use auth::{
    login_handler, logout_handler, AuthenticatedUser, Claims, LoginRequest, LoginResponse,
};

// Governed knowledge upload and same-logical-asset source replacement boundary.
// The legacy module remains compiled for its focused validation tests; runtime
// routing uses the version-aware handler below.
pub mod knowledge_upload;
pub mod knowledge_asset_upload;
pub use knowledge_asset_upload::{
    knowledge_upload_handler, MAX_KNOWLEDGE_PDF_BYTES, MAX_KNOWLEDGE_UPLOAD_BODY_BYTES,
};

// Platform-admin-only private source review boundary
pub mod knowledge_source;
pub use knowledge_source::knowledge_source_handler;

/// Basic health check handler
pub async fn health_check() -> Result<Json<Value>, StatusCode> {
    Ok(Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now()
    })))
}

/// Example user handler (placeholder)
pub async fn get_user(Path(id): Path<String>) -> Result<Json<Value>, StatusCode> {
    // This is a placeholder - implement actual user retrieval logic
    Ok(Json(json!({
        "id": id,
        "message": "User handler placeholder"
    })))
}

/// Example handler for testing middleware
pub async fn test_validation() -> Result<Json<Value>, StatusCode> {
    Ok(Json(json!({
        "message": "Validation test successful"
    })))
}

#[cfg(feature = "server")]
pub mod submission_attachments;
