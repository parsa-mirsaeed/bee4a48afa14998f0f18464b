//! Versioned and operator-configured LLM profiles for governed personalization.
//!
//! Fixed profiles preserve their exact historical provider/model pairing.
//! Custom profiles encode the operator-selected model in their durable profile
//! identifier; the gateway alone owns its HTTPS destination and credentials.

use std::borrow::Cow;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmProviderKind {
    DeepSeek,
    GapGpt,
    Custom,
}

impl LlmProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DeepSeek => "deepseek",
            Self::GapGpt => "gapgpt",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmProfile {
    pub id: Cow<'static, str>,
    pub provider: LlmProviderKind,
    pub model: Cow<'static, str>,
}

pub const DEEPSEEK_CHAT_V1: LlmProfile = LlmProfile {
    id: Cow::Borrowed("deepseek-chat-v1"),
    provider: LlmProviderKind::DeepSeek,
    model: Cow::Borrowed("deepseek-chat"),
};

pub const DEEPSEEK_V41_FLASH_V1: LlmProfile = LlmProfile {
    id: Cow::Borrowed("deepseek-v41-flash-v1"),
    provider: LlmProviderKind::GapGpt,
    model: Cow::Borrowed("deepseek-v4.1-flash"),
};

pub const CUSTOM_PROFILE_PREFIX: &str = "custom-v1:";

/// Model syntax must also match PostgreSQL policy and job constraints. Model
/// names are deployment-owned, not accepted as arbitrary client requests.
pub fn valid_custom_model(model: &str) -> bool {
    (1..=96).contains(&model.len())
        && model.as_bytes()[0].is_ascii_alphanumeric()
        && model.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'-' | b'_' | b'.' | b'/' | b':')
        })
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LlmProfileError {
    #[error("Unsupported LLM profile: {0}")]
    Unsupported(String),
    #[error("LLM model mismatch for {profile}: expected {expected}, got {actual}")]
    ModelMismatch {
        profile: String,
        expected: String,
        actual: String,
    },
}

pub fn resolve_llm_profile(value: &str) -> Result<LlmProfile, LlmProfileError> {
    let value = value.trim();
    if let Some(model) = value.strip_prefix(CUSTOM_PROFILE_PREFIX) {
        if valid_custom_model(model) {
            return Ok(LlmProfile {
                id: Cow::Owned(format!("{CUSTOM_PROFILE_PREFIX}{model}")),
                provider: LlmProviderKind::Custom,
                model: Cow::Owned(model.to_string()),
            });
        }
        return Err(LlmProfileError::Unsupported(
            "custom-v1:<valid-model>".to_string(),
        ));
    }
    match value.to_ascii_lowercase().as_str() {
        "deepseek" | "deepseek-chat" | "deepseek-chat-v1" => Ok(DEEPSEEK_CHAT_V1),
        "deepseek-v41-flash-v1" => Ok(DEEPSEEK_V41_FLASH_V1),
        other => Err(LlmProfileError::Unsupported(other.chars().take(128).collect())),
    }
}

pub fn validate_llm_profile_override(
    profile: &LlmProfile,
    model: Option<&str>,
) -> Result<(), LlmProfileError> {
    if let Some(model) = model {
        if model != profile.model {
            return Err(LlmProfileError::ModelMismatch {
                profile: profile.id.to_string(),
                expected: profile.model.to_string(),
                actual: model.chars().take(128).collect(),
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
        assert_eq!(DEEPSEEK_V41_FLASH_V1.provider.as_str(), "gapgpt");
        assert_eq!(
            resolve_llm_profile("deepseek-chat-v1").unwrap(),
            DEEPSEEK_CHAT_V1
        );
    }

    #[test]
    fn arbitrary_models_are_rejected_for_fixed_profiles() {
        assert!(matches!(
            validate_llm_profile_override(&DEEPSEEK_CHAT_V1, Some("user-controlled-model")),
            Err(LlmProfileError::ModelMismatch { .. })
        ));
    }

    #[test]
    fn matching_model_is_accepted() {
        validate_llm_profile_override(&DEEPSEEK_CHAT_V1, Some(DEEPSEEK_CHAT_V1.model.as_ref()))
            .unwrap();
        validate_llm_profile_override(
            &DEEPSEEK_V41_FLASH_V1,
            Some(DEEPSEEK_V41_FLASH_V1.model.as_ref()),
        )
        .unwrap();
        assert_eq!(
            resolve_llm_profile(DEEPSEEK_V41_FLASH_V1.id.as_ref()).unwrap(),
            DEEPSEEK_V41_FLASH_V1
        );
    }

    #[test]
    fn models_are_exactly_pinned_per_profile() {
        for rejected in [
            "deepseek-v4.1-flash",
            "deepseek-any-other-model",
            "deepseek/another-model",
            "prefix-deepseek-chat",
            "deepseek-chat-suffix",
        ] {
            assert!(
                matches!(
                    validate_llm_profile_override(&DEEPSEEK_CHAT_V1, Some(rejected)),
                    Err(LlmProfileError::ModelMismatch { .. })
                ),
                "unexpected registered model {rejected}"
            );
        }
        assert!(validate_llm_profile_override(
            &DEEPSEEK_V41_FLASH_V1,
            Some(DEEPSEEK_CHAT_V1.model.as_ref())
        )
        .is_err());
        assert!(resolve_llm_profile("deepseek-user-model").is_err());
    }

    #[test]
    fn custom_profiles_bind_a_durable_exact_model() {
        for model in [
            "openai/gpt-4o-mini",
            "meta-llama/llama-3.3-70b-instruct:free",
            "llama-3.1-8b-instant",
        ] {
            let id = format!("{CUSTOM_PROFILE_PREFIX}{model}");
            let profile = resolve_llm_profile(&id).unwrap();
            assert_eq!(profile.provider, LlmProviderKind::Custom);
            assert_eq!(profile.id, id);
            assert_eq!(profile.model, model);
            assert!(validate_llm_profile_override(&profile, Some(model)).is_ok());
            assert!(validate_llm_profile_override(&profile, Some("different-model")).is_err());
        }
        for model in [
            "",
            " model",
            "model with spaces",
            "../model",
            "model?query=1",
            "model#fragment",
            "http://127.0.0.1",
            "a\nmodel",
        ] {
            assert!(
                resolve_llm_profile(&format!("{CUSTOM_PROFILE_PREFIX}{model}")).is_err(),
                "accepted unsafe model {model:?}"
            );
        }
        assert!(resolve_llm_profile(&format!(
            "{CUSTOM_PROFILE_PREFIX}{}",
            "a".repeat(97)
        ))
        .is_err());
    }
}
