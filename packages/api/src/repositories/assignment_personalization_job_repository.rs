use crate::repositories::{BaseRepository, Repository, RepositoryError, RepositoryResult};
use chrono::{DateTime, Utc};
use sqlx::Row;
use uuid::Uuid;

pub const ASSIGNMENT_PERSONALIZATION_MODEL: &str = "deepseek-chat";
pub const ASSIGNMENT_PERSONALIZATION_PROFILE: &str = "assignment_personalization_v1";
pub const ASSIGNMENT_PERSONALIZATION_PROFILE_VERSION: i32 = 1;
pub const ASSIGNMENT_PERSONALIZATION_LLM_PROFILE: &str = "deepseek-chat-v1";
pub const ASSIGNMENT_PERSONALIZATION_LLM_PROVIDER: &str = "deepseek";

#[derive(Debug, Clone)]
pub struct ClaimedAssignmentPersonalizationJob {
    pub id: Uuid,
    pub school_id: Uuid,
    pub assignment_id: Uuid,
    pub student_id: Uuid,
    pub requested_by: Uuid,
    pub attempt_count: i32,
    pub model_name: String,
    pub profile_name: String,
    pub profile_version: i32,
    pub llm_profile_id: String,
    pub llm_provider: String,
    pub policy_scope: String,
    pub policy_version: i32,
    pub delivery_policy: String,
    pub specialization_instructions: String,
    pub lease_owner: Uuid,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PersonalizationExecutionDiagnostics {
    pub talent_profile_present: bool,
    pub teacher_report_count: i32,
    pub performance_context_present: bool,
    pub class_material_chunk_count: i32,
    pub governed_knowledge_chunk_count: i32,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub generated_content_changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonalizationFailureKind {
    GatewayUnavailable,
    ConfigurationUnavailable,
    RateLimited,
    InvalidGatewayResponse,
    ProcessingUnavailable,
    ContentRejected,
}

impl PersonalizationFailureKind {
    pub fn code(self) -> &'static str {
        match self {
            Self::GatewayUnavailable => "gateway_unavailable",
            Self::ConfigurationUnavailable => "provider_unconfigured",
            Self::RateLimited => "rate_limited",
            Self::InvalidGatewayResponse => "invalid_gateway_response",
            Self::ProcessingUnavailable => "processing_unavailable",
            Self::ContentRejected => "content_rejected",
        }
    }

    pub fn safe_summary(self) -> &'static str {
        match self {
            Self::GatewayUnavailable => "AI gateway is temporarily unavailable",
            Self::ConfigurationUnavailable => "AI personalization provider is not configured",
            Self::RateLimited => "AI personalization is temporarily rate limited",
            Self::InvalidGatewayResponse => "AI gateway returned an invalid response",
            Self::ProcessingUnavailable => "Personalization processing is temporarily unavailable",
            Self::ContentRejected => "Personalization input was rejected by safety limits",
        }
    }

    pub fn retryable(self) -> bool {
        !matches!(self, Self::ContentRejected | Self::ConfigurationUnavailable)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonalizationFailureDisposition {
    Requeued,
    FailedPermanently,
    IgnoredInactive,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PersonalizationQueueSummary {
    pub queued: i64,
    pub running: i64,
    pub succeeded: i64,
    pub failed: i64,
    pub cancelled: i64,
    pub total: i64,
    pub max_attempt_count: i32,
    pub last_completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdminPersonalizationScopeSummary {
    pub school_id: Option<Uuid>,
    pub teacher_user_id: Option<Uuid>,
    pub assignment_id: Option<Uuid>,
    pub queued: i64,
    pub running: i64,
    pub succeeded: i64,
    pub failed: i64,
    pub cancelled: i64,
    pub total: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminPersonalizationJobRecord {
    pub job_id: Uuid,
    pub school_id: Uuid,
    pub school_name: String,
    pub teacher_user_id: Uuid,
    pub teacher_name: String,
    pub assignment_id: Uuid,
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
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub heartbeat_at: Option<DateTime<Utc>>,
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

#[derive(Clone)]
pub struct AssignmentPersonalizationJobRepository {
    base: BaseRepository,
}

impl AssignmentPersonalizationJobRepository {
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

    pub async fn summaries_for_platform_admin(
        &self,
        actor_id: Uuid,
    ) -> RepositoryResult<Vec<AdminPersonalizationScopeSummary>> {
        self.require_platform_admin(actor_id).await?;
        let rows = sqlx::query(
            r#"
            SELECT
                school_id,
                requested_by AS teacher_user_id,
                assignment_id,
                COUNT(*) FILTER (WHERE status = 'queued') AS queued,
                COUNT(*) FILTER (WHERE status = 'running') AS running,
                COUNT(*) FILTER (WHERE status = 'succeeded') AS succeeded,
                COUNT(*) FILTER (WHERE status = 'failed') AS failed,
                COUNT(*) FILTER (WHERE status = 'cancelled') AS cancelled,
                COUNT(*) AS total
            FROM assignment_personalization_jobs
            GROUP BY GROUPING SETS (
                (),
                (school_id),
                (school_id, requested_by),
                (school_id, requested_by, assignment_id)
            )
            ORDER BY school_id NULLS FIRST, teacher_user_id NULLS FIRST, assignment_id NULLS FIRST
            "#,
        )
        .fetch_all(&*self.base.pool())
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(AdminPersonalizationScopeSummary {
                    school_id: row.try_get("school_id")?,
                    teacher_user_id: row.try_get("teacher_user_id")?,
                    assignment_id: row.try_get("assignment_id")?,
                    queued: row.try_get("queued")?,
                    running: row.try_get("running")?,
                    succeeded: row.try_get("succeeded")?,
                    failed: row.try_get("failed")?,
                    cancelled: row.try_get("cancelled")?,
                    total: row.try_get("total")?,
                })
            })
            .collect()
    }

    pub async fn retry_failed_scope_for_platform_admin(
        &self,
        actor_id: Uuid,
        school_id: Option<Uuid>,
        teacher_user_id: Option<Uuid>,
        assignment_id: Option<Uuid>,
        limit: i64,
    ) -> RepositoryResult<u64> {
        self.require_platform_admin(actor_id).await?;
        let retried = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT public.retry_assignment_personalization_jobs_admin(
                NULL,
                $1,
                $2,
                $3,
                $4
            )
            "#,
        )
        .bind(school_id)
        .bind(teacher_user_id)
        .bind(assignment_id)
        .bind(limit.clamp(1, 100) as i32)
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(retried.max(0) as u64)
    }

    pub async fn list_for_platform_admin(
        &self,
        actor_id: Uuid,
        school_id: Option<Uuid>,
        teacher_user_id: Option<Uuid>,
        limit: i64,
    ) -> RepositoryResult<Vec<AdminPersonalizationJobRecord>> {
        self.require_platform_admin(actor_id).await?;
        let rows = sqlx::query(
            r#"
            SELECT *
            FROM public.list_assignment_personalization_jobs_for_admin($1, $2, $3)
            "#,
        )
        .bind(school_id)
        .bind(teacher_user_id)
        .bind(limit.clamp(1, 500) as i32)
        .fetch_all(&*self.base.pool())
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(AdminPersonalizationJobRecord {
                    job_id: row.try_get("job_id")?,
                    school_id: row.try_get("school_id")?,
                    school_name: row.try_get("school_name")?,
                    teacher_user_id: row.try_get("teacher_user_id")?,
                    teacher_name: row.try_get("teacher_name")?,
                    assignment_id: row.try_get("assignment_id")?,
                    assignment_title: row.try_get("assignment_title")?,
                    student_reference: row.try_get("student_reference")?,
                    status: row.try_get("status")?,
                    processing_stage: row.try_get("processing_stage")?,
                    attempt_count: row.try_get("attempt_count")?,
                    llm_profile_id: row.try_get("llm_profile_id")?,
                    llm_provider: row.try_get("llm_provider")?,
                    model_name: row.try_get("model_name")?,
                    policy_scope: row.try_get("policy_scope")?,
                    policy_version: row.try_get("policy_version")?,
                    delivery_policy: row.try_get("delivery_policy")?,
                    last_error_code: row.try_get("last_error_code")?,
                    last_error_summary: row.try_get("last_error_summary")?,
                    created_at: row.try_get("created_at")?,
                    started_at: row.try_get("started_at")?,
                    completed_at: row.try_get("completed_at")?,
                    heartbeat_at: row.try_get("heartbeat_at")?,
                    talent_profile_present: row.try_get("talent_profile_present")?,
                    teacher_report_count: row.try_get("teacher_report_count")?,
                    performance_context_present: row.try_get("performance_context_present")?,
                    class_material_chunk_count: row.try_get("class_material_chunk_count")?,
                    governed_knowledge_chunk_count: row
                        .try_get("governed_knowledge_chunk_count")?,
                    prompt_tokens: row.try_get("prompt_tokens")?,
                    completion_tokens: row.try_get("completion_tokens")?,
                    total_tokens: row.try_get("total_tokens")?,
                    generated_content_changed: row.try_get("generated_content_changed")?,
                })
            })
            .collect()
    }

    pub async fn retry_for_platform_admin(
        &self,
        actor_id: Uuid,
        job_id: Uuid,
    ) -> RepositoryResult<()> {
        self.require_platform_admin(actor_id).await?;
        let retried = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT public.retry_assignment_personalization_jobs_admin(
                $1,
                NULL,
                NULL,
                NULL,
                1
            )
            "#,
        )
        .bind(job_id)
        .fetch_one(&*self.base.pool())
        .await?;

        if retried != 1 {
            return Err(RepositoryError::Validation(
                "Personalization job is not eligible for admin retry".to_string(),
            ));
        }
        Ok(())
    }

    /// Explicit teacher retry remains durable. The target is re-authorized in
    /// SQL and a failed/cancelled job is returned to the queue without creating
    /// another idempotency identity. Already-personalized work is a no-op.
    pub async fn requeue_for_teacher(
        &self,
        requested_by: Uuid,
        assignment_id: Uuid,
        student_id: Uuid,
    ) -> RepositoryResult<Uuid> {
        let row = sqlx::query(
            r#"
            WITH target AS (
                SELECT
                    custom_assignment.id AS custom_assignment_id,
                    custom_assignment.prompt_ctx,
                    teacher.school_id,
                    assignment.class_section_id
                FROM assignments assignment
                JOIN teachers teacher ON teacher.id = assignment.teacher_id
                JOIN users teacher_user ON teacher_user.id = teacher.user_id
                JOIN roles teacher_role ON teacher_role.id = teacher_user.role_id
                JOIN class_sections class_section ON class_section.id = assignment.class_section_id
                JOIN teaching_assignments teaching_assignment
                  ON teaching_assignment.teacher_id = teacher.id
                 AND teaching_assignment.class_section_id = assignment.class_section_id
                JOIN students student ON student.id = $2
                JOIN users student_user ON student_user.id = student.user_id
                JOIN enrollments enrollment
                  ON enrollment.student_id = student.id
                 AND enrollment.class_section_id = assignment.class_section_id
                JOIN custom_assignments custom_assignment
                  ON custom_assignment.assignment_id = assignment.id
                 AND custom_assignment.student_id = student.id
                WHERE assignment.id = $1
                  AND assignment.status = 'Published'::assignment_status
                  AND teacher.user_id = $3
                  AND teacher_user.is_active = TRUE
                  AND teacher_role.name::text = 'Teacher'
                  AND teacher.school_id = class_section.school_id
                  AND student.school_id = teacher.school_id
                  AND student_user.school_id = teacher.school_id
                  AND student_user.is_active = TRUE
            ),
            effective AS (
                SELECT target.*, policy.*
                FROM target
                CROSS JOIN LATERAL public.resolve_assignment_personalization_policy_v2(
                    target.school_id,
                    $3
                ) AS policy
            ),
            upserted AS (
                INSERT INTO assignment_personalization_jobs (
                    school_id,
                    assignment_id,
                    student_id,
                    class_section_id,
                    requested_by,
                    status,
                    attempt_count,
                    available_at,
                    lease_owner,
                    heartbeat_at,
                    last_error_code,
                    last_error_summary,
                    completed_at,
                    idempotency_key,
                    model_name,
                    profile_name,
                    profile_version,
                    llm_profile_id,
                    llm_provider,
                    policy_scope,
                    policy_version,
                    delivery_policy,
                    specialization_instructions,
                    processing_stage
                )
                SELECT
                    effective.school_id,
                    $1,
                    $2,
                    effective.class_section_id,
                    $3,
                    CASE WHEN effective.enabled THEN 'queued' ELSE 'cancelled' END,
                    0,
                    NOW(),
                    NULL,
                    NULL,
                    CASE WHEN effective.enabled THEN NULL ELSE 'policy_disabled' END,
                    CASE
                        WHEN effective.enabled THEN NULL
                        ELSE 'Assignment personalization is disabled by policy'
                    END,
                    CASE WHEN effective.enabled THEN NULL ELSE NOW() END,
                    concat($1::text, ':', $2::text, ':assignment_personalization_v1:1'),
                    $4,
                    $5,
                    $6,
                    effective.llm_profile_id,
                    $7,
                    effective.policy_scope,
                    effective.policy_version,
                    effective.delivery_policy,
                    effective.specialization_instructions,
                    CASE
                        WHEN NOT effective.enabled THEN 'cancelled'
                        WHEN effective.paused THEN 'paused'
                        ELSE 'queued'
                    END
                FROM effective
                WHERE effective.prompt_ctx IS NULL
                ON CONFLICT (assignment_id, student_id, profile_name, profile_version)
                DO UPDATE SET
                    status = CASE
                        WHEN assignment_personalization_jobs.status = 'succeeded'
                            THEN assignment_personalization_jobs.status
                        ELSE EXCLUDED.status
                    END,
                    attempt_count = CASE
                        WHEN assignment_personalization_jobs.status = 'succeeded'
                            THEN assignment_personalization_jobs.attempt_count
                        ELSE 0
                    END,
                    available_at = CASE
                        WHEN assignment_personalization_jobs.status = 'succeeded'
                            THEN assignment_personalization_jobs.available_at
                        ELSE NOW()
                    END,
                    lease_owner = CASE
                        WHEN assignment_personalization_jobs.status = 'succeeded'
                            THEN assignment_personalization_jobs.lease_owner
                        ELSE NULL
                    END,
                    heartbeat_at = CASE
                        WHEN assignment_personalization_jobs.status = 'succeeded'
                            THEN assignment_personalization_jobs.heartbeat_at
                        ELSE NULL
                    END,
                    last_error_code = CASE
                        WHEN assignment_personalization_jobs.status = 'succeeded'
                            THEN assignment_personalization_jobs.last_error_code
                        ELSE EXCLUDED.last_error_code
                    END,
                    last_error_summary = CASE
                        WHEN assignment_personalization_jobs.status = 'succeeded'
                            THEN assignment_personalization_jobs.last_error_summary
                        ELSE EXCLUDED.last_error_summary
                    END,
                    completed_at = CASE
                        WHEN assignment_personalization_jobs.status = 'succeeded'
                            THEN assignment_personalization_jobs.completed_at
                        ELSE EXCLUDED.completed_at
                    END,
                    processing_stage = CASE
                        WHEN assignment_personalization_jobs.status = 'succeeded'
                            THEN assignment_personalization_jobs.processing_stage
                        ELSE EXCLUDED.processing_stage
                    END
                RETURNING id
            )
            SELECT effective.custom_assignment_id
            FROM effective
            LEFT JOIN upserted ON TRUE
            LIMIT 1
            "#,
        )
        .bind(assignment_id)
        .bind(student_id)
        .bind(requested_by)
        .bind(ASSIGNMENT_PERSONALIZATION_MODEL)
        .bind(ASSIGNMENT_PERSONALIZATION_PROFILE)
        .bind(ASSIGNMENT_PERSONALIZATION_PROFILE_VERSION)
        .bind(ASSIGNMENT_PERSONALIZATION_LLM_PROVIDER)
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or(RepositoryError::Unauthorized)?;

        Ok(row.try_get("custom_assignment_id")?)
    }

    /// Validate the exact claimed assignment/student relationship under the
    /// original Teacher actor. This is called both before and after the provider
    /// request; if enrollment or authorization changes mid-flight, the outer
    /// transaction rolls back generated content.
    pub async fn authorize_claimed_job(
        &self,
        job: &ClaimedAssignmentPersonalizationJob,
    ) -> RepositoryResult<()> {
        let authorized = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM assignment_personalization_jobs queue
                JOIN assignments assignment ON assignment.id = queue.assignment_id
                JOIN teachers teacher ON teacher.id = assignment.teacher_id
                JOIN users teacher_user ON teacher_user.id = teacher.user_id
                JOIN roles teacher_role ON teacher_role.id = teacher_user.role_id
                JOIN class_sections class_section ON class_section.id = assignment.class_section_id
                JOIN teaching_assignments teaching_assignment
                  ON teaching_assignment.teacher_id = teacher.id
                 AND teaching_assignment.class_section_id = assignment.class_section_id
                JOIN students student ON student.id = queue.student_id
                JOIN users student_user ON student_user.id = student.user_id
                JOIN enrollments enrollment
                  ON enrollment.student_id = student.id
                 AND enrollment.class_section_id = assignment.class_section_id
                JOIN custom_assignments custom_assignment
                  ON custom_assignment.assignment_id = assignment.id
                 AND custom_assignment.student_id = student.id
                WHERE queue.id = $1
                  AND queue.status = 'running'
                  AND queue.lease_owner = $2
                  AND queue.requested_by = $3
                  AND queue.school_id = $4
                  AND queue.assignment_id = $5
                  AND queue.student_id = $6
                  AND assignment.status = 'Published'::assignment_status
                  AND teacher.user_id = queue.requested_by
                  AND teacher_user.is_active = TRUE
                  AND teacher_role.name::text = 'Teacher'
                  AND teacher.school_id = queue.school_id
                  AND class_section.school_id = queue.school_id
                  AND student.school_id = queue.school_id
                  AND student_user.school_id = queue.school_id
                  AND student_user.is_active = TRUE
            )
            "#,
        )
        .bind(job.id)
        .bind(job.lease_owner)
        .bind(job.requested_by)
        .bind(job.school_id)
        .bind(job.assignment_id)
        .bind(job.student_id)
        .fetch_one(&*self.base.pool())
        .await?;

        if !authorized {
            return Err(RepositoryError::Unauthorized);
        }
        Ok(())
    }

    pub async fn heartbeat(&self, job_id: Uuid, lease_owner: Uuid) -> RepositoryResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE assignment_personalization_jobs
            SET heartbeat_at = NOW()
            WHERE id = $1
              AND status = 'running'
              AND lease_owner = $2
            "#,
        )
        .bind(job_id)
        .bind(lease_owner)
        .execute(&*self.base.pool())
        .await?;
        if result.rows_affected() != 1 {
            return Err(RepositoryError::Validation(
                "Personalization job lease is no longer active".into(),
            ));
        }
        Ok(())
    }

    pub async fn update_stage(
        &self,
        job_id: Uuid,
        lease_owner: Uuid,
        stage: &str,
    ) -> RepositoryResult<()> {
        if !matches!(
            stage,
            "authorizing"
                | "building_student_context"
                | "retrieving_context"
                | "ai_gateway"
                | "provider"
                | "validating_response"
                | "saving"
        ) {
            return Err(RepositoryError::Validation(
                "Unsupported assignment personalization processing stage".to_string(),
            ));
        }
        let result = sqlx::query(
            r#"
            UPDATE assignment_personalization_jobs
            SET processing_stage = $3,
                heartbeat_at = NOW()
            WHERE id = $1
              AND status = 'running'
              AND lease_owner = $2
            "#,
        )
        .bind(job_id)
        .bind(lease_owner)
        .bind(stage)
        .execute(&*self.base.pool())
        .await?;
        if result.rows_affected() != 1 {
            return Err(RepositoryError::Validation(
                "Personalization job lease is no longer active".into(),
            ));
        }
        Ok(())
    }

    pub async fn complete_with_diagnostics(
        &self,
        job_id: Uuid,
        lease_owner: Uuid,
        diagnostics: &PersonalizationExecutionDiagnostics,
    ) -> RepositoryResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE assignment_personalization_jobs
            SET status = 'succeeded',
                completed_at = NOW(),
                lease_owner = NULL,
                heartbeat_at = NULL,
                last_error_code = NULL,
                last_error_summary = NULL,
                processing_stage = 'ready',
                talent_profile_present = $3,
                teacher_report_count = $4,
                performance_context_present = $5,
                class_material_chunk_count = $6,
                governed_knowledge_chunk_count = $7,
                prompt_tokens = $8,
                completion_tokens = $9,
                total_tokens = $10,
                generated_content_changed = $11
            WHERE id = $1
              AND status = 'running'
              AND lease_owner = $2
            "#,
        )
        .bind(job_id)
        .bind(lease_owner)
        .bind(diagnostics.talent_profile_present)
        .bind(diagnostics.teacher_report_count)
        .bind(diagnostics.performance_context_present)
        .bind(diagnostics.class_material_chunk_count)
        .bind(diagnostics.governed_knowledge_chunk_count)
        .bind(diagnostics.prompt_tokens)
        .bind(diagnostics.completion_tokens)
        .bind(diagnostics.total_tokens)
        .bind(diagnostics.generated_content_changed)
        .execute(&*self.base.pool())
        .await?;
        if result.rows_affected() != 1 {
            return Err(RepositoryError::Validation(
                "Personalization job lease changed before completion".into(),
            ));
        }
        Ok(())
    }

    pub async fn complete(&self, job_id: Uuid, lease_owner: Uuid) -> RepositoryResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE assignment_personalization_jobs
            SET status = 'succeeded',
                completed_at = NOW(),
                lease_owner = NULL,
                heartbeat_at = NULL,
                last_error_code = NULL,
                last_error_summary = NULL,
                processing_stage = 'ready'
            WHERE id = $1
              AND status = 'running'
              AND lease_owner = $2
            "#,
        )
        .bind(job_id)
        .bind(lease_owner)
        .execute(&*self.base.pool())
        .await?;
        if result.rows_affected() != 1 {
            return Err(RepositoryError::Validation(
                "Personalization job lease changed before completion".into(),
            ));
        }
        Ok(())
    }

    pub async fn cancel_claimed_job(
        &self,
        job_id: Uuid,
        lease_owner: Uuid,
    ) -> RepositoryResult<()> {
        sqlx::query(
            r#"
            UPDATE assignment_personalization_jobs
            SET status = 'cancelled',
                completed_at = NOW(),
                lease_owner = NULL,
                heartbeat_at = NULL,
                last_error_code = 'authorization_revoked',
                last_error_summary = 'Assignment personalization authorization is no longer valid',
                processing_stage = 'cancelled'
            WHERE id = $1
              AND status = 'running'
              AND lease_owner = $2
            "#,
        )
        .bind(job_id)
        .bind(lease_owner)
        .execute(&*self.base.pool())
        .await?;
        Ok(())
    }

    pub async fn record_failure(
        &self,
        job: &ClaimedAssignmentPersonalizationJob,
        kind: PersonalizationFailureKind,
        retry_after_seconds: u64,
        max_attempts: i32,
    ) -> RepositoryResult<PersonalizationFailureDisposition> {
        let active = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM assignment_personalization_jobs
                WHERE id = $1
                  AND status = 'running'
                  AND lease_owner = $2
            )
            "#,
        )
        .bind(job.id)
        .bind(job.lease_owner)
        .fetch_one(&*self.base.pool())
        .await?;
        if !active {
            return Ok(PersonalizationFailureDisposition::IgnoredInactive);
        }

        let max_attempts = max_attempts.clamp(1, 10);
        let should_retry = kind.retryable() && job.attempt_count < max_attempts;
        if should_retry {
            let exponential = 2_i64.pow(job.attempt_count.clamp(1, 10) as u32);
            let delay_seconds = exponential.max(retry_after_seconds.min(3_600) as i64);
            sqlx::query(
                r#"
                UPDATE assignment_personalization_jobs
                SET status = 'queued',
                    available_at = NOW() + make_interval(secs => $3::DOUBLE PRECISION),
                    lease_owner = NULL,
                    heartbeat_at = NULL,
                    last_error_code = $4,
                    last_error_summary = $5,
                    processing_stage = 'queued'
                WHERE id = $1
                  AND status = 'running'
                  AND lease_owner = $2
                "#,
            )
            .bind(job.id)
            .bind(job.lease_owner)
            .bind(delay_seconds)
            .bind(kind.code())
            .bind(kind.safe_summary())
            .execute(&*self.base.pool())
            .await?;
            return Ok(PersonalizationFailureDisposition::Requeued);
        }

        sqlx::query(
            r#"
            UPDATE assignment_personalization_jobs
            SET status = 'failed',
                completed_at = NOW(),
                lease_owner = NULL,
                heartbeat_at = NULL,
                last_error_code = $3,
                last_error_summary = $4,
                processing_stage = 'failed'
            WHERE id = $1
              AND status = 'running'
              AND lease_owner = $2
            "#,
        )
        .bind(job.id)
        .bind(job.lease_owner)
        .bind(kind.code())
        .bind(kind.safe_summary())
        .execute(&*self.base.pool())
        .await?;
        Ok(PersonalizationFailureDisposition::FailedPermanently)
    }

    /// Safe teacher-facing metrics. No prompts, generated content, provider
    /// payloads, secrets, or raw error bodies are returned.
    pub async fn summary_for_teacher(
        &self,
        requested_by: Uuid,
    ) -> RepositoryResult<PersonalizationQueueSummary> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE status = 'queued') AS queued,
                COUNT(*) FILTER (WHERE status = 'running') AS running,
                COUNT(*) FILTER (WHERE status = 'succeeded') AS succeeded,
                COUNT(*) FILTER (WHERE status = 'failed') AS failed,
                COUNT(*) FILTER (WHERE status = 'cancelled') AS cancelled,
                COUNT(*) AS total,
                COALESCE(MAX(attempt_count), 0) AS max_attempt_count,
                MAX(completed_at) AS last_completed_at
            FROM assignment_personalization_jobs
            WHERE requested_by = $1
              AND school_id = (SELECT school_id FROM users WHERE id = $1)
            "#,
        )
        .bind(requested_by)
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(PersonalizationQueueSummary {
            queued: row.try_get("queued")?,
            running: row.try_get("running")?,
            succeeded: row.try_get("succeeded")?,
            failed: row.try_get("failed")?,
            cancelled: row.try_get("cancelled")?,
            total: row.try_get("total")?,
            max_attempt_count: row.try_get("max_attempt_count")?,
            last_completed_at: row.try_get("last_completed_at")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_failure_messages_are_fixed_and_non_sensitive() {
        for kind in [
            PersonalizationFailureKind::GatewayUnavailable,
            PersonalizationFailureKind::ConfigurationUnavailable,
            PersonalizationFailureKind::RateLimited,
            PersonalizationFailureKind::InvalidGatewayResponse,
            PersonalizationFailureKind::ProcessingUnavailable,
            PersonalizationFailureKind::ContentRejected,
        ] {
            assert!(kind.code().len() <= 64);
            assert!(kind
                .code()
                .chars()
                .all(|character| character.is_ascii_lowercase() || character == '_'));
            assert!(kind.safe_summary().len() <= 160);
            let lowered = kind.safe_summary().to_ascii_lowercase();
            for forbidden in [
                "authorization: bearer",
                "api_key",
                "password",
                "postgresql://",
            ] {
                assert!(!lowered.contains(forbidden));
            }
        }
        assert!(!PersonalizationFailureKind::ContentRejected.retryable());
        assert!(!PersonalizationFailureKind::ConfigurationUnavailable.retryable());
        assert!(PersonalizationFailureKind::RateLimited.retryable());
    }
}
