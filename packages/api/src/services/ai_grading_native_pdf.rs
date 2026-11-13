//! Server-only native PDF grading input boundary.
//!
//! This is a *binary document* contract, not text extraction. No OCR,
//! PDF-to-text transformation, or provider request is performed here.
//! Dispatch stays disabled until a separately reviewed provider-native
//! document adapter has its own exact-head end-to-end security proof.

use crate::services::submission_attachment_storage::{hash, validate_bytes};
use crate::server_functions::submission_attachment_functions::MAX_ATTACHMENT_BYTES;
use uuid::Uuid;

/// No student document bytes, filenames, base64 or storage keys are included
/// in error messages or Debug formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativePdfInputError {
    MissingIdentity,
    InvalidDigest,
    InvalidOriginal,
    UnsupportedMediaType,
}

/// Authenticated source identity is supplied by the authorized repository,
/// never by an LLM response or a client-controlled PDF URL.
pub struct NativePdfGradingInput {
    school_id: Uuid,
    submission_id: Uuid,
    submission_revision: Uuid,
    attachment_id: Uuid,
    bytes: Vec<u8>,
}

impl NativePdfGradingInput {
    pub fn from_private_original(
        school_id: Uuid,
        submission_id: Uuid,
        submission_revision: Uuid,
        attachment_id: Uuid,
        media_type: &str,
        expected_sha256: &str,
        original: Vec<u8>,
    ) -> Result<Self, NativePdfInputError> {
        if [school_id, submission_id, submission_revision, attachment_id]
            .iter()
            .any(Uuid::is_nil)
        {
            return Err(NativePdfInputError::MissingIdentity);
        }
        if media_type != "application/pdf" {
            return Err(NativePdfInputError::UnsupportedMediaType);
        }
        if expected_sha256.len() != 64
            || !expected_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(NativePdfInputError::InvalidDigest);
        }
        if original.is_empty() || original.len() > MAX_ATTACHMENT_BYTES {
            return Err(NativePdfInputError::InvalidOriginal);
        }
        // Reject mutated storage originals before any future outbound request.
        // Exact PDF framing and MIME are checked without extracting text.
        if hash(&original) != expected_sha256.to_ascii_lowercase()
            || validate_bytes("application/pdf", &original).is_err()
        {
            return Err(NativePdfInputError::InvalidOriginal);
        }
        Ok(Self {
            school_id,
            submission_id,
            submission_revision,
            attachment_id,
            bytes: original,
        })
    }

    pub fn school_id(&self) -> Uuid { self.school_id }
    pub fn submission_id(&self) -> Uuid { self.submission_id }
    pub fn submission_revision(&self) -> Uuid { self.submission_revision }
    pub fn attachment_id(&self) -> Uuid { self.attachment_id }
    pub fn original_pdf_bytes(&self) -> &[u8] { &self.bytes }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pdf() -> Vec<u8> {
        // Synthetic, structurally framed single-object PDF; no student data.
        let mut raw = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\n".to_vec();
        let xref = raw.len();
        raw.extend_from_slice(format!("xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Size 1 >>\nstartxref\n{xref}\n%%EOF").as_bytes());
        raw
    }

    fn make(media_type: &str, digest: &str, bytes: Vec<u8>) -> Result<NativePdfGradingInput, NativePdfInputError> {
        NativePdfGradingInput::from_private_original(
            Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(),
            media_type, digest, bytes,
        )
    }

    #[test]
    fn original_bytes_are_preserved_without_text_extraction() {
        let bytes = pdf();
        let input = make("application/pdf", &hash(&bytes), bytes.clone()).unwrap();
        assert_eq!(input.original_pdf_bytes(), bytes.as_slice());
        assert!(!input.school_id().is_nil());
        assert!(!input.submission_revision().is_nil());
    }

    #[test]
    fn rejects_hash_mismatch_truncated_original_and_non_pdf_media() {
        let bytes = pdf();
        assert!(matches!(make("application/pdf", &"0".repeat(64), bytes.clone()), Err(NativePdfInputError::InvalidOriginal)));
        assert!(matches!(make("application/pdf", &hash(&bytes), bytes[..bytes.len()-5].to_vec()), Err(NativePdfInputError::InvalidOriginal)));
        assert!(matches!(make("image/png", &hash(&bytes), bytes.clone()), Err(NativePdfInputError::UnsupportedMediaType)));
        assert!(matches!(make("application/pdf", "not-a-digest", bytes), Err(NativePdfInputError::InvalidDigest)));
    }

    #[test]
    fn rejects_missing_binding_and_does_not_expose_document_in_debug() {
        let bytes = pdf();
        let error = NativePdfGradingInput::from_private_original(
            Uuid::nil(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(),
            "application/pdf", &hash(&bytes), bytes,
        ).err().unwrap();
        assert_eq!(error, NativePdfInputError::MissingIdentity);
        assert_eq!(format!("{error:?}"), "MissingIdentity");
    }
}
