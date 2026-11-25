//! Internal application-to-AI-gateway wire types.
//!
//! Provider credentials and destination URLs are intentionally absent. The
//! application may select only the configured model/profile; the gateway owns
//! every external destination and secret.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GatewayEmbeddingRequest {
    pub model: String,
    pub input: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GatewayEmbeddingResponse {
    pub object: String,
    pub data: Vec<GatewayEmbeddingData>,
    pub model: String,
    #[serde(default)]
    pub usage: Option<GatewayUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GatewayEmbeddingData {
    pub object: String,
    pub embedding: Vec<f32>,
    pub index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct GatewayUsage {
    #[serde(default)]
    pub prompt_tokens: Option<u32>,
    #[serde(default)]
    pub completion_tokens: Option<u32>,
    #[serde(default)]
    pub total_tokens: Option<u32>,
}

/// Internal native-PDF grading request. Identity fields are asserted by the
/// upstream authorized server: the gateway must independently verify its
/// bearer token and school header, never trust a client-supplied destination.
/// No external file URLs or arbitrary provider credentials are accepted.
/// Intentionally no Debug implementation: a request contains student PDF bytes.
#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GatewayNativePdfRequest {
    pub model: String,
    pub submission_id: Uuid,
    pub submission_revision: Uuid,
    pub attachment_id: Uuid,
    pub pdf_sha256: String,
    /// Base64 encoding of the exact immutable original PDF, not extracted text.
    pub pdf_base64: String,
    pub system_instructions: String,
    pub teacher_rubric_prompt: String,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GatewayChatRequest {
    pub model: String,
    pub messages: Vec<GatewayChatMessage>,
    pub max_tokens: u32,
    pub temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<GatewayResponseFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GatewayChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GatewayResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GatewayChatResponse {
    pub model: String,
    pub choices: Vec<GatewayChatChoice>,
    #[serde(default)]
    pub usage: Option<GatewayUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GatewayChatChoice {
    pub index: usize,
    pub message: GatewayChatMessage,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GatewayErrorEnvelope {
    pub error: GatewayErrorBody,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GatewayErrorBody {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_seconds: Option<u64>,
}

/** Internal gateway-only, bounded diagnostic header. Never copy provider headers. */
pub const PROVIDER_EVIDENCE_HEADER: &str = "x-edutalent-provider-evidence";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderDispatch {
    Blocked,
    Attempted,
    Responded,
}

impl ProviderDispatch {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blocked => "blocked",
            Self::Attempted => "attempted",
            Self::Responded => "responded",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderFailureReason {
    RequestPreparation,
    Timeout,
    Connect,
    Transport,
    UpstreamStatus,
    ResponseSize,
    BodyRead,
    MalformedJson,
    ModelMismatch,
    InvalidChoice,
}

impl ProviderFailureReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RequestPreparation => "request_preparation",
            Self::Timeout => "timeout",
            Self::Connect => "connect",
            Self::Transport => "transport",
            Self::UpstreamStatus => "upstream_status",
            Self::ResponseSize => "response_size",
            Self::BodyRead => "body_read",
            Self::MalformedJson => "malformed_json",
            Self::ModelMismatch => "model_mismatch",
            Self::InvalidChoice => "invalid_choice",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderFailureEvidence {
    pub dispatch: ProviderDispatch,
    pub reason: ProviderFailureReason,
    /// Invocations of the provider HTTP client's send, not billing evidence.
    pub transport_attempts: u8,
    pub upstream_http_status: Option<u16>,
}

impl ProviderFailureEvidence {
    pub fn is_valid(self) -> bool {
        use ProviderDispatch as D;
        use ProviderFailureReason as R;
        match (self.dispatch, self.reason) {
            (D::Blocked, R::RequestPreparation) => {
                self.transport_attempts == 0 && self.upstream_http_status.is_none()
            }
            (D::Attempted, R::Timeout | R::Connect | R::Transport) => {
                (1..=4).contains(&self.transport_attempts) && self.upstream_http_status.is_none()
            }
            (D::Responded, R::UpstreamStatus) => {
                (1..=4).contains(&self.transport_attempts)
                    && self.upstream_http_status.is_some_and(|status| {
                        (100..=599).contains(&status) && !(200..=299).contains(&status)
                    })
            }
            (D::Responded, R::ResponseSize | R::BodyRead | R::MalformedJson | R::ModelMismatch | R::InvalidChoice) => {
                (1..=4).contains(&self.transport_attempts)
                    && self.upstream_http_status.is_some_and(|status| (200..=299).contains(&status))
            }
            _ => false,
        }
    }

    /// Reject unknown fields, values, oversized metadata and contradictory
    /// combinations. Do not include the raw header in errors or logs.
    pub fn parse(value: &str) -> Option<Self> {
        if value.len() > 256 {
            return None;
        }
        serde_json::from_str::<Self>(value).ok().filter(|evidence| evidence.is_valid())
    }
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;

    #[test]
    fn only_bounded_coherent_provider_evidence_is_accepted() {
        let good = r#"{"dispatch":"responded","reason":"model_mismatch","transport_attempts":1,"upstream_http_status":200}"#;
        assert!(ProviderFailureEvidence::parse(good).is_some());
        for bad in [
            good.replace("200", "401"),
            good.replace("model_mismatch", "provider-secret-sentinel"),
            good.replace("\"transport_attempts\":1", "\"transport_attempts\":5"),
            good.replace("responded", "blocked"),
            good.replace("200", "null"),
            format!("{}{}", &good[..good.len() - 1], r#","raw":"secret"}"#),
        ] {
            assert!(ProviderFailureEvidence::parse(&bad).is_none());
        }
    }
}
