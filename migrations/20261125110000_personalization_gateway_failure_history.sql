-- Immutable, sanitized gateway failure history. No raw prompts, bodies,
-- external credentials, URLs or student identifiers. Parent job owns retention.
CREATE TABLE public.assignment_personalization_gateway_failures (
    request_id UUID PRIMARY KEY,
    job_id UUID NOT NULL REFERENCES public.assignment_personalization_jobs(id) ON DELETE CASCADE,
    lease_owner UUID NOT NULL,
    execution_generation INTEGER NOT NULL CHECK (execution_generation > 0),
    attempt_number INTEGER NOT NULL CHECK (attempt_number > 0),
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    stage TEXT NOT NULL CHECK (stage IN (
        'queued', 'paused', 'authorizing', 'building_student_context',
        'retrieving_context', 'ai_gateway', 'provider', 'validating_response',
        'saving', 'ready', 'failed', 'cancelled'
    )),
    diagnostic_code TEXT NOT NULL CHECK (diagnostic_code IN (
        'ai_temporarily_unavailable', 'gateway_shutting_down', 'circuit_open',
        'provider_rate_limited', 'quota_exceeded', 'provider_unconfigured',
        'llm_disabled', 'invalid_provider_response', 'provider_response_too_large',
        'provider_rejected_request', 'gateway_error'
    )),
    gateway_http_status INTEGER NOT NULL CHECK (gateway_http_status BETWEEN 100 AND 599),
    gateway_elapsed_ms BIGINT NOT NULL CHECK (gateway_elapsed_ms BETWEEN 0 AND 86400000),
    retry_after_seconds INTEGER CHECK (retry_after_seconds BETWEEN 1 AND 3600),
    scheduled_retry_at TIMESTAMPTZ,
    provider_dispatch_state TEXT NOT NULL CHECK (provider_dispatch_state IN ('blocked', 'unknown', 'attempted', 'responded')),
    provider_failure_reason TEXT CHECK (provider_failure_reason IN (
        'request_preparation', 'timeout', 'connect', 'transport', 'upstream_status',
        'response_size', 'body_read', 'malformed_json', 'model_mismatch', 'invalid_choice'
    )),
    provider_transport_attempts INTEGER CHECK (provider_transport_attempts BETWEEN 0 AND 4),
    upstream_http_status INTEGER CHECK (upstream_http_status BETWEEN 100 AND 599),
    CHECK (
        (provider_dispatch_state = 'unknown' AND provider_failure_reason IS NULL AND provider_transport_attempts IS NULL AND upstream_http_status IS NULL)
        OR (provider_dispatch_state = 'blocked' AND (provider_failure_reason IS NULL OR provider_failure_reason = 'request_preparation')
            AND provider_transport_attempts IS NOT NULL AND provider_transport_attempts = 0 AND upstream_http_status IS NULL)
        OR (provider_dispatch_state = 'attempted' AND provider_failure_reason IS NOT NULL
            AND provider_failure_reason IN ('timeout','connect','transport')
            AND provider_transport_attempts IS NOT NULL AND provider_transport_attempts BETWEEN 1 AND 4 AND upstream_http_status IS NULL)
        OR (provider_dispatch_state = 'responded' AND provider_failure_reason IS NOT NULL
            AND provider_transport_attempts IS NOT NULL AND provider_transport_attempts BETWEEN 1 AND 4
            AND upstream_http_status IS NOT NULL AND (
                (provider_failure_reason = 'upstream_status' AND upstream_http_status NOT BETWEEN 200 AND 299)
                OR (provider_failure_reason IN ('response_size','body_read','malformed_json','model_mismatch','invalid_choice')
                    AND upstream_http_status BETWEEN 200 AND 299)
            ))
    )
);
CREATE INDEX assignment_personalization_gateway_failures_job_time_idx
    ON public.assignment_personalization_gateway_failures (job_id, recorded_at DESC, request_id);
CREATE INDEX assignment_personalization_gateway_failures_observed_idx
    ON public.assignment_personalization_gateway_failures (job_id, recorded_at, request_id)
    WHERE provider_dispatch_state IN ('attempted', 'responded');
ALTER TABLE public.assignment_personalization_gateway_failures ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.assignment_personalization_gateway_failures FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE public.assignment_personalization_gateway_failures FROM PUBLIC;

CREATE POLICY personalization_gateway_failure_admin_select
ON public.assignment_personalization_gateway_failures FOR SELECT
USING (public.get_role() = 'PlatformAdmin');

-- Only the active authorized original teacher transaction can append a
-- record for its matching job/lease/attempt. No update or delete policy.
CREATE POLICY personalization_gateway_failure_active_insert
ON public.assignment_personalization_gateway_failures FOR INSERT
WITH CHECK (
    public.get_role() = 'Teacher'
    AND EXISTS (
        SELECT 1 FROM public.assignment_personalization_jobs j
        WHERE j.id = job_id
          AND j.requested_by = public.get_user_id()
          AND j.school_id = public.get_school_id()
          AND j.status = 'running'
          AND j.lease_owner = assignment_personalization_gateway_failures.lease_owner
          AND j.attempt_count = attempt_number
          AND j.execution_generation = assignment_personalization_gateway_failures.execution_generation
          AND j.processing_stage = stage
    )
);
COMMENT ON TABLE public.assignment_personalization_gateway_failures IS
    'Append-only, parent-job-retained allowlisted gateway failures; admin read only. External status only when observed; no billing inference.';
