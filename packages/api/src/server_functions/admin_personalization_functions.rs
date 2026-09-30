use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use {
    crate::repositories::{
        AdminPersonalizationJobRecord, AssignmentPersonalizationJobRepository,
        AssignmentPersonalizationPolicyRepository, SchoolPersonalizationPolicy,
        TeacherPersonalizationPolicy,
    },
    crate::services::DEEPSEEK_CHAT_V1,
    uuid::Uuid,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminPersonalizationMethodDto {
    pub profile_id: String,
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminSchoolPersonalizationPolicyDto {
    pub school_id: String,
    pub school_name: String,
    pub enabled: bool,
    pub paused: bool,
    pub llm_profile_id: String,
    pub delivery_policy: String,
    pub policy_version: i32,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminTeacherPersonalizationPolicyDto {
    pub teacher_id: String,
    pub teacher_user_id: String,
    pub teacher_name: String,
    pub school_id: String,
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
pub struct AdminPersonalizationJobDto {
    pub job_id: String,
    pub school_id: String,
    pub school_name: String,
    pub teacher_user_id: String,
    pub teacher_name: String,
    pub assignment_id: String,
    pub assignment_title: String,
    pub student_reference: String,
    pub status: String,
    pub processing_stage: String,
    pub attempt_count: i32,
    pub llm_profile_id: String,
    pub llm_provider: String,
    pub model_name: String,
    pub policy_scope: String,
    pub policy_version: i32,
    pub delivery_policy: String,
    pub last_error_code: Option<String>,
    pub last_error_summary: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub heartbeat_at: Option<String>,
    pub talent_profile_present: Option<bool>,
    pub teacher_report_count: Option<i32>,
    pub performance_context_present: Option<bool>,
    pub class_material_chunk_count: Option<i32>,
    pub governed_knowledge_chunk_count: Option<i32>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub generated_content_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminPersonalizationOverviewDto {
    pub methods: Vec<AdminPersonalizationMethodDto>,
    pub schools: Vec<AdminSchoolPersonalizationPolicyDto>,
    pub recent_jobs: Vec<AdminPersonalizationJobDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetAdminSchoolPersonalizationPolicyRequest {
    pub school_id: String,
    pub enabled: bool,
    pub paused: bool,
    pub llm_profile_id: String,
    pub delivery_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetAdminTeacherPersonalizationPolicyRequest {
    pub teacher_id: String,
    pub mode: String,
    pub enabled_override: Option<bool>,
    pub paused_override: Option<bool>,
    pub llm_profile_id_override: Option<String>,
    pub delivery_policy_override: Option<String>,
}

#[cfg(feature = "server")]
async fn platform_admin_context(
) -> Result<(Uuid, std::sync::Arc<crate::rls_context::AuthorizedPool>), ServerFnError> {
    let (user, pool) = crate::server_functions::rls_helpers::extract_user_with_full_rls().await?;
    if user.role != "PlatformAdmin" {
        return Err(ServerFnError::new("Forbidden: insufficient role"));
    }
    let actor_id =
        Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid authenticated user"))?;
    Ok((actor_id, pool))
}

#[cfg(feature = "server")]
fn school_dto(policy: SchoolPersonalizationPolicy) -> AdminSchoolPersonalizationPolicyDto {
    AdminSchoolPersonalizationPolicyDto {
        school_id: policy.school_id.to_string(),
        school_name: policy.school_name,
        enabled: policy.enabled,
        paused: policy.paused,
        llm_profile_id: policy.llm_profile_id,
        delivery_policy: policy.delivery_policy,
        policy_version: policy.policy_version,
        updated_at: policy.updated_at.to_rfc3339(),
    }
}

#[cfg(feature = "server")]
fn teacher_dto(policy: TeacherPersonalizationPolicy) -> AdminTeacherPersonalizationPolicyDto {
    AdminTeacherPersonalizationPolicyDto {
        teacher_id: policy.teacher_id.to_string(),
        teacher_user_id: policy.teacher_user_id.to_string(),
        teacher_name: policy.teacher_name,
        school_id: policy.school_id.to_string(),
        mode: policy.mode,
        enabled_override: policy.enabled_override,
        paused_override: policy.paused_override,
        llm_profile_id_override: policy.llm_profile_id_override,
        delivery_policy_override: policy.delivery_policy_override,
        override_version: policy.override_version,
        effective_enabled: policy.effective_enabled,
        effective_paused: policy.effective_paused,
        effective_llm_profile_id: policy.effective_llm_profile_id,
        effective_delivery_policy: policy.effective_delivery_policy,
        effective_scope: policy.effective_scope,
        effective_version: policy.effective_version,
    }
}

#[cfg(feature = "server")]
fn job_dto(job: AdminPersonalizationJobRecord) -> AdminPersonalizationJobDto {
    AdminPersonalizationJobDto {
        job_id: job.job_id.to_string(),
        school_id: job.school_id.to_string(),
        school_name: job.school_name,
        teacher_user_id: job.teacher_user_id.to_string(),
        teacher_name: job.teacher_name,
        assignment_id: job.assignment_id.to_string(),
        assignment_title: job.assignment_title,
        student_reference: job.student_reference,
        status: job.status,
        processing_stage: job.processing_stage,
        attempt_count: job.attempt_count,
        llm_profile_id: job.llm_profile_id,
        llm_provider: job.llm_provider,
        model_name: job.model_name,
        policy_scope: job.policy_scope,
        policy_version: job.policy_version,
        delivery_policy: job.delivery_policy,
        last_error_code: job.last_error_code,
        last_error_summary: job.last_error_summary,
        created_at: job.created_at.to_rfc3339(),
        started_at: job.started_at.map(|value| value.to_rfc3339()),
        completed_at: job.completed_at.map(|value| value.to_rfc3339()),
        heartbeat_at: job.heartbeat_at.map(|value| value.to_rfc3339()),
        talent_profile_present: job.talent_profile_present,
        teacher_report_count: job.teacher_report_count,
        performance_context_present: job.performance_context_present,
        class_material_chunk_count: job.class_material_chunk_count,
        governed_knowledge_chunk_count: job.governed_knowledge_chunk_count,
        prompt_tokens: job.prompt_tokens,
        completion_tokens: job.completion_tokens,
        total_tokens: job.total_tokens,
        generated_content_changed: job.generated_content_changed,
    }
}

#[server(endpoint = "admin/personalization/overview")]
pub async fn get_admin_personalization_overview(
) -> Result<AdminPersonalizationOverviewDto, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (actor_id, pool) = platform_admin_context().await?;
        let policy_repository =
            AssignmentPersonalizationPolicyRepository::new(pool.clone());
        let job_repository = AssignmentPersonalizationJobRepository::new(pool);

        let schools = policy_repository
            .list_school_policies(actor_id)
            .await
            .map_err(|error| {
                tracing::error!(error_code = "personalization_policy_list_failed", %error);
                ServerFnError::new("Unable to load personalization policies")
            })?
            .into_iter()
            .map(school_dto)
            .collect();

        let recent_jobs = job_repository
            .list_for_platform_admin(actor_id, None, None, 200)
            .await
            .map_err(|error| {
                tracing::error!(error_code = "personalization_job_list_failed", %error);
                ServerFnError::new("Unable to load personalization jobs")
            })?
            .into_iter()
            .map(job_dto)
            .collect();

        Ok(AdminPersonalizationOverviewDto {
            methods: vec![AdminPersonalizationMethodDto {
                profile_id: DEEPSEEK_CHAT_V1.id.to_string(),
                provider: DEEPSEEK_CHAT_V1.provider.as_str().to_string(),
                model: DEEPSEEK_CHAT_V1.model.to_string(),
            }],
            schools,
            recent_jobs,
        })
    }

    #[cfg(not(feature = "server"))]
    Ok(AdminPersonalizationOverviewDto {
        methods: Vec::new(),
        schools: Vec::new(),
        recent_jobs: Vec::new(),
    })
}

#[server(endpoint = "admin/personalization/teachers")]
pub async fn list_admin_personalization_teachers(
    school_id: String,
) -> Result<Vec<AdminTeacherPersonalizationPolicyDto>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (actor_id, pool) = platform_admin_context().await?;
        let school_id =
            Uuid::parse_str(&school_id).map_err(|_| ServerFnError::new("Invalid school ID"))?;
        AssignmentPersonalizationPolicyRepository::new(pool)
            .list_teacher_policies(actor_id, school_id)
            .await
            .map(|items| items.into_iter().map(teacher_dto).collect())
            .map_err(|error| {
                tracing::error!(error_code = "personalization_teacher_policy_list_failed", %error);
                ServerFnError::new("Unable to load teacher personalization policies")
            })
    }

    #[cfg(not(feature = "server"))]
    Ok(Vec::new())
}

#[server(endpoint = "admin/personalization/school-policy")]
pub async fn set_admin_school_personalization_policy(
    request: SetAdminSchoolPersonalizationPolicyRequest,
) -> Result<AdminSchoolPersonalizationPolicyDto, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (actor_id, pool) = platform_admin_context().await?;
        let school_id = Uuid::parse_str(&request.school_id)
            .map_err(|_| ServerFnError::new("Invalid school ID"))?;
        AssignmentPersonalizationPolicyRepository::new(pool)
            .set_school_policy(
                actor_id,
                school_id,
                request.enabled,
                request.paused,
                &request.llm_profile_id,
                &request.delivery_policy,
            )
            .await
            .map(school_dto)
            .map_err(|error| {
                tracing::error!(error_code = "personalization_school_policy_update_failed", %error);
                ServerFnError::new("Unable to update school personalization policy")
            })
    }

    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server only"))
}

#[server(endpoint = "admin/personalization/teacher-policy")]
pub async fn set_admin_teacher_personalization_policy(
    request: SetAdminTeacherPersonalizationPolicyRequest,
) -> Result<AdminTeacherPersonalizationPolicyDto, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (actor_id, pool) = platform_admin_context().await?;
        let teacher_id = Uuid::parse_str(&request.teacher_id)
            .map_err(|_| ServerFnError::new("Invalid teacher ID"))?;
        AssignmentPersonalizationPolicyRepository::new(pool)
            .set_teacher_override(
                actor_id,
                teacher_id,
                &request.mode,
                request.enabled_override,
                request.paused_override,
                request.llm_profile_id_override.as_deref(),
                request.delivery_policy_override.as_deref(),
            )
            .await
            .map(teacher_dto)
            .map_err(|error| {
                tracing::error!(error_code = "personalization_teacher_policy_update_failed", %error);
                ServerFnError::new("Unable to update teacher personalization policy")
            })
    }

    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server only"))
}

#[server(endpoint = "admin/personalization/retry-job")]
pub async fn retry_admin_personalization_job(job_id: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (actor_id, pool) = platform_admin_context().await?;
        let job_id =
            Uuid::parse_str(&job_id).map_err(|_| ServerFnError::new("Invalid job ID"))?;
        AssignmentPersonalizationJobRepository::new(pool)
            .retry_for_platform_admin(actor_id, job_id)
            .await
            .map_err(|error| {
                tracing::error!(error_code = "personalization_admin_retry_failed", %error);
                ServerFnError::new("Personalization job is not eligible for retry")
            })
    }

    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server only"))
}
