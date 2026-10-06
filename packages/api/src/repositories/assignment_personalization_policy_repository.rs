use crate::repositories::{BaseRepository, Repository, RepositoryError, RepositoryResult};
use crate::services::{
    llm_profile::resolve_llm_profile, llm_service::normalize_assignment_specialization_instructions,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

pub const DELIVERY_REQUIRE_PERSONALIZED: &str = "require_personalized";
pub const DELIVERY_ALLOW_ORIGINAL_FALLBACK: &str = "allow_original_fallback";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SchoolPersonalizationPolicy {
    pub school_id: Uuid,
    pub school_name: String,
    pub enabled: bool,
    pub paused: bool,
    pub llm_profile_id: String,
    pub delivery_policy: String,
    pub specialization_instructions: String,
    pub policy_version: i32,
    pub updated_by: Option<Uuid>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TeacherPersonalizationPolicy {
    pub teacher_id: Uuid,
    pub teacher_user_id: Uuid,
    pub teacher_name: String,
    pub school_id: Uuid,
    pub mode: String,
    pub enabled_override: Option<bool>,
    pub paused_override: Option<bool>,
    pub llm_profile_id_override: Option<String>,
    pub delivery_policy_override: Option<String>,
    pub override_version: i32,
    pub effective_enabled: bool,
    pub effective_paused: bool,
    pub effective_llm_profile_id: String,
    pub effective_delivery_policy: String,
    pub effective_scope: String,
    pub effective_version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EffectivePersonalizationPolicy {
    pub enabled: bool,
    pub paused: bool,
    pub llm_profile_id: String,
    pub delivery_policy: String,
    pub policy_scope: String,
    pub policy_version: i32,
}

#[derive(Clone)]
pub struct AssignmentPersonalizationPolicyRepository {
    base: BaseRepository,
}

impl AssignmentPersonalizationPolicyRepository {
    pub fn new<T>(pool: T) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    async fn require_platform_admin(&self, actor_id: Uuid) -> RepositoryResult<()> {
        let allowed = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT public.get_role() = 'PlatformAdmin'
               AND public.get_user_id() = $1
            "#,
        )
        .bind(actor_id)
        .fetch_one(&*self.base.pool())
        .await?;
        if !allowed {
            return Err(RepositoryError::Unauthorized);
        }
        Ok(())
    }

    fn validate_policy_values(
        profile_id: &str,
        delivery_policy: &str,
    ) -> RepositoryResult<&'static str> {
        let profile = resolve_llm_profile(profile_id)
            .map_err(|error| RepositoryError::Validation(error.to_string()))?;
        if !matches!(
            delivery_policy,
            DELIVERY_REQUIRE_PERSONALIZED | DELIVERY_ALLOW_ORIGINAL_FALLBACK
        ) {
            return Err(RepositoryError::Validation(
                "Unsupported assignment personalization delivery policy".to_string(),
            ));
        }
        Ok(profile.id)
    }

    fn normalize_specialization_instructions(value: &str) -> RepositoryResult<String> {
        normalize_assignment_specialization_instructions(value)
            .map_err(|error| RepositoryError::Validation(error.to_string()))
    }

    pub async fn list_school_policies(
        &self,
        actor_id: Uuid,
    ) -> RepositoryResult<Vec<SchoolPersonalizationPolicy>> {
        self.require_platform_admin(actor_id).await?;
        let rows = sqlx::query(
            r#"
            SELECT
                school.id AS school_id,
                school.name AS school_name,
                COALESCE(policy.enabled, TRUE) AS enabled,
                COALESCE(policy.paused, FALSE) AS paused,
                COALESCE(policy.llm_profile_id, 'deepseek-chat-v1') AS llm_profile_id,
                COALESCE(policy.delivery_policy, 'require_personalized') AS delivery_policy,
                COALESCE(
                    NULLIF(BTRIM(policy.specialization_instructions), ''),
                    'Adapt difficulty, scope, format, and scaffolding to the learner profile while preserving the original learning objective, required knowledge, and grading intent.'
                ) AS specialization_instructions,
                COALESCE(policy.policy_version, 1) AS policy_version,
                policy.updated_by,
                COALESCE(policy.updated_at, school.created_at) AS updated_at
            FROM schools AS school
            LEFT JOIN assignment_personalization_school_policies AS policy
              ON policy.school_id = school.id
            ORDER BY school.name, school.id
            "#,
        )
        .fetch_all(&*self.base.pool())
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(SchoolPersonalizationPolicy {
                    school_id: row.try_get("school_id")?,
                    school_name: row.try_get("school_name")?,
                    enabled: row.try_get("enabled")?,
                    paused: row.try_get("paused")?,
                    llm_profile_id: row.try_get("llm_profile_id")?,
                    delivery_policy: row.try_get("delivery_policy")?,
                    specialization_instructions: row.try_get("specialization_instructions")?,
                    policy_version: row.try_get("policy_version")?,
                    updated_by: row.try_get("updated_by")?,
                    updated_at: row.try_get("updated_at")?,
                })
            })
            .collect()
    }

    pub async fn list_teacher_policies(
        &self,
        actor_id: Uuid,
        school_id: Uuid,
    ) -> RepositoryResult<Vec<TeacherPersonalizationPolicy>> {
        self.require_platform_admin(actor_id).await?;
        let rows = sqlx::query(
            r#"
            SELECT *
            FROM public.list_assignment_personalization_teacher_policies_for_admin($1)
            "#,
        )
        .bind(school_id)
        .fetch_all(&*self.base.pool())
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(TeacherPersonalizationPolicy {
                    teacher_id: row.try_get("teacher_id")?,
                    teacher_user_id: row.try_get("teacher_user_id")?,
                    teacher_name: row.try_get("teacher_name")?,
                    school_id: row.try_get("school_id")?,
                    mode: row.try_get("mode")?,
                    enabled_override: row.try_get("enabled_override")?,
                    paused_override: row.try_get("paused_override")?,
                    llm_profile_id_override: row.try_get("llm_profile_id_override")?,
                    delivery_policy_override: row.try_get("delivery_policy_override")?,
                    override_version: row.try_get("override_version")?,
                    effective_enabled: row.try_get("effective_enabled")?,
                    effective_paused: row.try_get("effective_paused")?,
                    effective_llm_profile_id: row.try_get("effective_llm_profile_id")?,
                    effective_delivery_policy: row.try_get("effective_delivery_policy")?,
                    effective_scope: row.try_get("effective_scope")?,
                    effective_version: row.try_get("effective_version")?,
                })
            })
            .collect()
    }

    pub async fn effective_for_teacher_user(
        &self,
        school_id: Uuid,
        teacher_user_id: Uuid,
    ) -> RepositoryResult<EffectivePersonalizationPolicy> {
        let row = sqlx::query(
            r#"
            SELECT enabled, paused, llm_profile_id, delivery_policy, policy_scope, policy_version
            FROM public.resolve_assignment_personalization_policy($1, $2)
            "#,
        )
        .bind(school_id)
        .bind(teacher_user_id)
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(EffectivePersonalizationPolicy {
            enabled: row.try_get("enabled")?,
            paused: row.try_get("paused")?,
            llm_profile_id: row.try_get("llm_profile_id")?,
            delivery_policy: row.try_get("delivery_policy")?,
            policy_scope: row.try_get("policy_scope")?,
            policy_version: row.try_get("policy_version")?,
        })
    }

    pub async fn set_school_policy(
        &self,
        actor_id: Uuid,
        school_id: Uuid,
        enabled: bool,
        paused: bool,
        llm_profile_id: &str,
        delivery_policy: &str,
        specialization_instructions: &str,
    ) -> RepositoryResult<SchoolPersonalizationPolicy> {
        self.require_platform_admin(actor_id).await?;
        let canonical_profile_id = Self::validate_policy_values(llm_profile_id, delivery_policy)?;
        let specialization_instructions =
            Self::normalize_specialization_instructions(specialization_instructions)?;

        sqlx::query(
            r#"
            INSERT INTO assignment_personalization_school_policies (
                school_id,
                enabled,
                paused,
                llm_profile_id,
                delivery_policy,
                specialization_instructions,
                policy_version,
                updated_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, 1, $7)
            ON CONFLICT (school_id)
            DO UPDATE SET
                enabled = EXCLUDED.enabled,
                paused = EXCLUDED.paused,
                llm_profile_id = EXCLUDED.llm_profile_id,
                delivery_policy = EXCLUDED.delivery_policy,
                specialization_instructions = EXCLUDED.specialization_instructions,
                policy_version = assignment_personalization_school_policies.policy_version + 1,
                updated_by = EXCLUDED.updated_by
            "#,
        )
        .bind(school_id)
        .bind(enabled)
        .bind(paused)
        .bind(canonical_profile_id)
        .bind(delivery_policy)
        .bind(&specialization_instructions)
        .bind(actor_id)
        .execute(&*self.base.pool())
        .await?;

        self.list_school_policies(actor_id)
            .await?
            .into_iter()
            .find(|policy| policy.school_id == school_id)
            .ok_or_else(|| RepositoryError::NotFound {
                entity: "SchoolPersonalizationPolicy".to_string(),
                id: school_id.to_string(),
            })
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn set_teacher_override(
        &self,
        actor_id: Uuid,
        teacher_id: Uuid,
        mode: &str,
        enabled_override: Option<bool>,
        paused_override: Option<bool>,
        llm_profile_id_override: Option<&str>,
        delivery_policy_override: Option<&str>,
    ) -> RepositoryResult<TeacherPersonalizationPolicy> {
        self.require_platform_admin(actor_id).await?;
        if !matches!(mode, "inherit" | "override") {
            return Err(RepositoryError::Validation(
                "Teacher personalization policy mode must be inherit or override".to_string(),
            ));
        }
        if mode == "inherit"
            && (enabled_override.is_some()
                || paused_override.is_some()
                || llm_profile_id_override.is_some()
                || delivery_policy_override.is_some())
        {
            return Err(RepositoryError::Validation(
                "Inherited teacher personalization policy cannot contain override values"
                    .to_string(),
            ));
        }
        let canonical_profile_override = if let Some(profile) = llm_profile_id_override {
            let delivery = delivery_policy_override.unwrap_or(DELIVERY_REQUIRE_PERSONALIZED);
            Some(Self::validate_policy_values(profile, delivery)?)
        } else {
            if let Some(delivery) = delivery_policy_override {
                if !matches!(
                    delivery,
                    DELIVERY_REQUIRE_PERSONALIZED | DELIVERY_ALLOW_ORIGINAL_FALLBACK
                ) {
                    return Err(RepositoryError::Validation(
                        "Unsupported assignment personalization delivery policy".to_string(),
                    ));
                }
            }
            None
        };

        let canonical_school = sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT public.assignment_personalization_teacher_school_for_admin($1)",
        )
        .bind(teacher_id)
        .fetch_one(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "Teacher".to_string(),
            id: teacher_id.to_string(),
        })?;

        sqlx::query(
            r#"
            INSERT INTO assignment_personalization_teacher_overrides (
                teacher_id,
                school_id,
                mode,
                enabled_override,
                paused_override,
                llm_profile_id_override,
                delivery_policy_override,
                override_version,
                updated_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 1, $8)
            ON CONFLICT (teacher_id)
            DO UPDATE SET
                school_id = EXCLUDED.school_id,
                mode = EXCLUDED.mode,
                enabled_override = EXCLUDED.enabled_override,
                paused_override = EXCLUDED.paused_override,
                llm_profile_id_override = EXCLUDED.llm_profile_id_override,
                delivery_policy_override = EXCLUDED.delivery_policy_override,
                override_version = assignment_personalization_teacher_overrides.override_version + 1,
                updated_by = EXCLUDED.updated_by
            "#,
        )
        .bind(teacher_id)
        .bind(canonical_school)
        .bind(mode)
        .bind(enabled_override)
        .bind(paused_override)
        .bind(canonical_profile_override)
        .bind(delivery_policy_override)
        .bind(actor_id)
        .execute(&*self.base.pool())
        .await?;

        self.list_teacher_policies(actor_id, canonical_school)
            .await?
            .into_iter()
            .find(|policy| policy.teacher_id == teacher_id)
            .ok_or_else(|| RepositoryError::NotFound {
                entity: "TeacherPersonalizationPolicy".to_string(),
                id: teacher_id.to_string(),
            })
    }
}

impl Repository for AssignmentPersonalizationPolicyRepository {
    fn pool(&self) -> Arc<crate::rls_context::AuthorizedPool> {
        self.base.pool()
    }
}
