//! Pinned Supabase Storage v1.60.4 may return HTTP 400 with a structured
//! NoSuchBucket code (rather than HTTP 404) for a missing bucket.
//! Only the exact machine code authorizes first-use bucket creation.
use reqwest::Response;
use serde_json::Value;

const MAX_ERROR_BODY: usize = 4096;

fn missing_bucket_code(status: u16, bytes: &[u8]) -> bool {
    if status == 404 {
        return true;
    }
    status == 400
        && serde_json::from_slice::<Value>(bytes)
            .ok()
            .and_then(|body| body.get("code").and_then(Value::as_str).map(str::to_owned))
            .as_deref()
            == Some("NoSuchBucket")
}

/// Consumes a failed bucket-lookup response without buffering unbounded HTML,
/// invalid JSON, or potentially sensitive provider error responses.
pub async fn missing_bucket_response(mut response: Response) -> bool {
    let status = response.status().as_u16();
    if status == 404 {
        return true;
    }
    if status != 400
        || response
            .content_length()
            .is_some_and(|n| n > MAX_ERROR_BODY as u64)
    {
        return false;
    }
    let mut bytes = Vec::new();
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) if bytes.len().saturating_add(chunk.len()) <= MAX_ERROR_BODY => {
                bytes.extend_from_slice(&chunk);
            }
            Ok(None) => break,
            _ => return false,
        }
    }
    missing_bucket_code(status, &bytes)
}

/// Fail closed for an existing public/misconfigured bucket. Both fields are
/// server-asserted; client-supplied object metadata is never trusted.
pub fn matches_private_bucket(body: &Value, limit: usize, mime: &[&str]) -> bool {
    if body.get("public").and_then(Value::as_bool) != Some(false) {
        return false;
    }
    // Older manually provisioned private buckets can have NULL server-side
    // limits. Application handlers independently enforce the strict size and
    // MIME contract. Reject contradictory *configured* limits, but do not
    // disable an otherwise private pre-existing bucket solely for omitted
    // optional Storage metadata.
    if let Some(size) = body.get("file_size_limit").filter(|v| !v.is_null()) {
        if size.as_u64() != Some(limit as u64) {
            return false;
        }
    }
    if let Some(types) = body.get("allowed_mime_types").filter(|v| !v.is_null()) {
        let Some(allowed) = types.as_array() else {
            return false;
        };
        if allowed.len() != mime.len()
            || !mime
                .iter()
                .all(|expected| allowed.iter().any(|v| v.as_str() == Some(expected)))
        {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn exact_missing_bucket_error_is_required() {
        assert!(missing_bucket_code(404, b""));
        assert!(missing_bucket_code(
            400,
            br#"{"code":"NoSuchBucket","message":"Bucket not found"}"#
        ));
        for body in [
            br#"{"code":"InvalidToken"}"#.as_slice(),
            br#"{"message":"Bucket not found"}"#,
            br#"{"code":"noSuchBucket"}"#,
            b"<html>400</html>",
            b"",
        ] {
            assert!(!missing_bucket_code(400, body));
        }
        for status in [200, 401, 403, 409, 429, 500, 503] {
            assert!(!missing_bucket_code(status, br#"{"code":"NoSuchBucket"}"#));
        }
    }

    #[test]
    fn existing_bucket_must_be_private_with_exact_limits_and_types() {
        let valid = json!({
            "public":false,"file_size_limit":10485760,
            "allowed_mime_types":["image/png","application/pdf","image/jpeg"]
        });
        let types = ["application/pdf", "image/jpeg", "image/png"];
        assert!(matches_private_bucket(&valid, 10 * 1024 * 1024, &types));
        assert!(!matches_private_bucket(&valid, 20 * 1024 * 1024, &types));
        assert!(!matches_private_bucket(
            &valid,
            10 * 1024 * 1024,
            &["application/pdf"]
        ));
        assert!(!matches_private_bucket(
            &json!({"public":true,"file_size_limit":10485760,
            "allowed_mime_types":types}),
            10 * 1024 * 1024,
            &types
        ));
        assert!(matches_private_bucket(
            &json!({"public":false}),
            10 * 1024 * 1024,
            &types
        ));
        assert!(matches_private_bucket(
            &json!({"public":false,"file_size_limit":null,"allowed_mime_types":null}),
            10 * 1024 * 1024,
            &types
        ));
        assert!(!matches_private_bucket(
            &json!({"public":false,"allowed_mime_types":"invalid"}),
            10 * 1024 * 1024,
            &types
        ));
    }
}
