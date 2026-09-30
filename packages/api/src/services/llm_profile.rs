//! Versioned LLM profiles used by governed assignment personalization.
//!
//! A profile binds the only provider/model pair application code is allowed to
//! request through the internal AI Gateway. Provider URLs and credentials remain
//! gateway-owned deployment configuration and are deliberately absent here.

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmProviderKind {
    DeepSeek,
}

impl LlmProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DeepSeek => "deepseek",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LlmProfile {
    pub id: &'static str,
    pub provider: LlmProviderKind,
    pub model: &'static str,
}

pub const DEEPSEEK_CHAT_V1: LlmProfile = LlmProfile {
    id: "deepseek-chat-v1",
    provider: LlmProviderKind::DeepSeek,
    model: "deepseek-chat",
};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LlmProfileError {
    #[error("Unsupported LLM profile: {0}")]
    Unsupported(String),
    #[error("LLM model mismatch for {profile}: expected {expected}, got {actual}")]
    ModelMismatch {
        profile: &'static str,
        expected: &'static str,
        actual: String,
    },
}

pub fn resolve_llm_profile(value: &str) -> Result<LlmProfile, LlmProfileError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "deepseek" | "deepseek-chat" | "deepseek-chat-v1" => Ok(DEEPSEEK_CHAT_V1),
        other => Err(LlmProfileError::Unsupported(other.to_string())),
    }
}

pub fn validate_llm_profile_override(
    profile: LlmProfile,
    model: Option<&str>,
) -> Result<(), LlmProfileError> {
    if let Some(model) = model.filter(|value| !value.trim().is_empty()) {
        if model != profile.model {
            return Err(LlmProfileError::ModelMismatch {
                profile: profile.id,
                expected: profile.model,
                actual: model.to_string(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_resolve_to_registered_profile() {
        assert_eq!(resolve_llm_profile("deepseek").unwrap(), DEEPSEEK_CHAT_V1);
        assert_eq!(
            resolve_llm_profile("deepseek-chat-v1").unwrap(),
            DEEPSEEK_CHAT_V1
        );
    }

    #[test]
    fn arbitrary_models_are_rejected() {
        assert!(matches!(
            validate_llm_profile_override(DEEPSEEK_CHAT_V1, Some("user-controlled-model")),
            Err(LlmProfileError::ModelMismatch { .. })
        ));
    }

    #[test]
    fn matching_model_is_accepted() {
        validate_llm_profile_override(DEEPSEEK_CHAT_V1, Some(DEEPSEEK_CHAT_V1.model)).unwrap();
    }
}
