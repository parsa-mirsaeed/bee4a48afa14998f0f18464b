//! Private originals only; no OCR/AI call and no client-controlled origin.
use crate::app_state::AppState;
use crate::repositories::submission_attachment_repository::{AttachmentError, Result};
use crate::server_functions::submission_attachment_functions::MAX_ATTACHMENT_BYTES;
use reqwest::{Client, RequestBuilder};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::time::Duration;

pub const BUCKET: &str = "edutalent-submission-originals";
fn client() -> Result<Client> {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|_| AttachmentError::Storage)
}
fn auth(request: RequestBuilder, state: &AppState) -> RequestBuilder {
    request
        .bearer_auth(&state.supabase_config.secret_key)
        .header("apikey", &state.supabase_config.secret_key)
}
fn url(state: &AppState, path: &str) -> String {
    format!(
        "{}/storage/v1/{path}",
        state.supabase_config.url.trim_end_matches('/')
    )
}

pub async fn verify_private_bucket(state: &AppState) -> Result<()> {
    let c = client()?;
    let endpoint = url(state, &format!("bucket/{BUCKET}"));
    let response = auth(c.get(&endpoint), state)
        .send()
        .await
        .map_err(|_| AttachmentError::Storage)?;
    if response.status().as_u16() == 404 {
        let created=auth(c.post(url(state,"bucket")),state).json(&json!({"id":BUCKET,"name":BUCKET,"public":false,"fileSizeLimit":MAX_ATTACHMENT_BYTES,"allowedMimeTypes":["application/pdf","image/jpeg","image/png"]})).send().await.map_err(|_| AttachmentError::Storage)?;
        if !created.status().is_success() && !matches!(created.status().as_u16(), 400 | 409) {
            return Err(AttachmentError::Storage);
        }
    } else if !response.status().is_success() {
        return Err(AttachmentError::Storage);
    }
    // Always read back privacy, including the create/concurrent-create paths.
    let check = auth(c.get(endpoint), state)
        .send()
        .await
        .map_err(|_| AttachmentError::Storage)?;
    if !check.status().is_success() {
        return Err(AttachmentError::Storage);
    }
    let data: Value = check.json().await.map_err(|_| AttachmentError::Storage)?;
    if data.get("public").and_then(Value::as_bool) != Some(false) {
        return Err(AttachmentError::Storage);
    }
    Ok(())
}

pub async fn read(state: &AppState, key: &str) -> Result<Vec<u8>> {
    verify_private_bucket(state).await?;
    let mut response = auth(
        client()?.get(url(state, &format!("object/{BUCKET}/{key}"))),
        state,
    )
    .send()
    .await
    .map_err(|_| AttachmentError::Storage)?;
    if !response.status().is_success()
        || response
            .content_length()
            .is_some_and(|n| n > MAX_ATTACHMENT_BYTES as u64)
    {
        return Err(AttachmentError::Storage);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| AttachmentError::Storage)?
    {
        if bytes.len().saturating_add(chunk.len()) > MAX_ATTACHMENT_BYTES {
            return Err(AttachmentError::Storage);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

pub async fn put_verified(
    state: &AppState,
    key: &str,
    media_type: &str,
    bytes: Vec<u8>,
    expected: &str,
) -> Result<()> {
    verify_private_bucket(state).await?;
    // No upsert. A lost response is reconciled by reading and comparing the
    // immutable object's bytes, never by accepting a conflict as proof.
    let size = bytes.len();
    let response = auth(
        client()?.post(url(state, &format!("object/{BUCKET}/{key}"))),
        state,
    )
    .header("Content-Type", media_type)
    .header("x-upsert", "false")
    .body(bytes)
    .send()
    .await;
    match response {
        Ok(r) if r.status().is_success() || matches!(r.status().as_u16(), 400 | 409) => {}
        _ => return Err(AttachmentError::Storage),
    }
    let stored = read(state, key).await?;
    if stored.len() != size || hash(&stored) != expected {
        return Err(AttachmentError::Storage);
    }
    Ok(())
}

pub async fn delete(state: &AppState, key: &str) -> Result<()> {
    let response = auth(
        client()?.delete(url(state, &format!("object/{BUCKET}"))),
        state,
    )
    .json(&json!({"prefixes":[key]}))
    .send()
    .await
    .map_err(|_| AttachmentError::Storage)?;
    if response.status().is_success() || response.status().as_u16() == 404 {
        Ok(())
    } else {
        Err(AttachmentError::Storage)
    }
}
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn validate_bytes(media_type: &str, bytes: &[u8]) -> Result<()> {
    if bytes.is_empty() || bytes.len() > MAX_ATTACHMENT_BYTES {
        return Err(AttachmentError::Limit);
    }
    match media_type {
        "application/pdf" => {
            // Bounded framing/truncation checks; do not decompress arbitrary PDF
            // streams in the gateway. Originals are forced downloads, not served
            // as executable inline content. This is not malware sanitization.
            if !(bytes.starts_with(b"%PDF-1.") || bytes.starts_with(b"%PDF-2.0")) {
                return Err(AttachmentError::Invalid);
            }
            let end = bytes
                .iter()
                .rposition(|b| !b.is_ascii_whitespace())
                .ok_or(AttachmentError::Invalid)?
                + 1;
            if !bytes[..end].ends_with(b"%%EOF") {
                return Err(AttachmentError::Invalid);
            }
            let tail = &bytes[end.saturating_sub(1024)..end - 5];
            let marker = tail
                .windows(9)
                .rposition(|w| w == b"startxref")
                .ok_or(AttachmentError::Invalid)?;
            let offset = std::str::from_utf8(&tail[marker + 9..])
                .ok()
                .and_then(|s| s.trim().parse::<usize>().ok())
                .ok_or(AttachmentError::Invalid)?;
            let crossref = bytes.get(offset..).ok_or(AttachmentError::Invalid)?;
            let stream_header = &crossref[..crossref.len().min(100)];
            if !crossref.starts_with(b"xref")
                && !(stream_header.windows(3).any(|w| w == b"obj")
                    && crossref.windows(5).any(|w| w == b"/XRef"))
            {
                return Err(AttachmentError::Invalid);
            }
        }
        "image/png" | "image/jpeg" => {
            let format = if media_type == "image/png" {
                image::ImageFormat::Png
            } else {
                image::ImageFormat::Jpeg
            };
            if image::guess_format(bytes).ok() != Some(format) {
                return Err(AttachmentError::Invalid);
            }
            if format == image::ImageFormat::Jpeg && !bytes.ends_with(&[0xff, 0xd9]) {
                return Err(AttachmentError::Invalid);
            }
            if format == image::ImageFormat::Png
                && !bytes.ends_with(&[0, 0, 0, 0, b'I', b'E', b'N', b'D', 174, 66, 96, 130])
            {
                return Err(AttachmentError::Invalid);
            }
            let mut reader = image::ImageReader::with_format(std::io::Cursor::new(bytes), format);
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(8192);
            limits.max_image_height = Some(8192);
            limits.max_alloc = Some(64 * 1024 * 1024);
            reader.limits(limits);
            reader.decode().map_err(|_| AttachmentError::Invalid)?;
        }
        _ => return Err(AttachmentError::Invalid),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_empty_truncated_and_mismatched_originals() {
        assert!(validate_bytes("application/pdf", b"%PDF-1.4\n%%EOF").is_err());
        assert!(validate_bytes("image/jpeg", &[255, 216, 255, 217]).is_err());
        assert!(validate_bytes("image/png", b"\x89PNG\r\n\x1a\n").is_err());
        assert!(validate_bytes("text/html", b"<html>").is_err());
        assert!(validate_bytes("image/png", b"").is_err());
    }
    #[test]
    fn validates_complete_bounded_images_without_mutating_originals() {
        for format in [image::ImageFormat::Png, image::ImageFormat::Jpeg] {
            let mut out = std::io::Cursor::new(Vec::new());
            image::DynamicImage::new_rgb8(2, 2)
                .write_to(&mut out, format)
                .unwrap();
            let bytes = out.into_inner();
            let mime = if format == image::ImageFormat::Png {
                "image/png"
            } else {
                "image/jpeg"
            };
            assert!(validate_bytes(mime, &bytes).is_ok());
            assert!(validate_bytes(mime, &bytes[..bytes.len() / 2]).is_err());
            assert!(validate_bytes("application/pdf", &bytes).is_err());
        }
    }
}
