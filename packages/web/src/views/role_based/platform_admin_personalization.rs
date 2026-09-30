use crate::i18n::{platform_admin_translation, use_locale, Locale};
use crate::views::role_based::components::DashboardSection;
use api::server_functions::admin_personalization_functions::{
    get_admin_personalization_overview, list_admin_personalization_teachers,
    retry_admin_personalization_job, set_admin_school_personalization_policy,
    set_admin_teacher_personalization_policy, AdminPersonalizationJobDto,
    AdminPersonalizationMethodDto, AdminSchoolPersonalizationPolicyDto,
    AdminTeacherPersonalizationPolicyDto, SetAdminSchoolPersonalizationPolicyRequest,
    SetAdminTeacherPersonalizationPolicyRequest,
};
use dioxus::prelude::*;

fn t(key: &'static str, locale: Locale) -> String {
    platform_admin_translation(key, locale)
        .unwrap_or(key)
        .to_string()
}

fn yes_no(value: bool, locale: Locale) -> String {
    t(
        if value {
            "platform_admin.personalization.yes"
        } else {
            "platform_admin.personalization.no"
        },
        locale,
    )
}

fn stage_label(stage: &str, locale: Locale) -> String {
    let key = match stage {
        "queued" => "platform_admin.personalization.stage.queued",
        "paused" => "platform_admin.personalization.stage.paused",
        "authorizing" => "platform_admin.personalization.stage.authorizing",
        "building_student_context" => {
            "platform_admin.personalization.stage.building_student_context"
        }
        "retrieving_context" => "platform_admin.personalization.stage.retrieving_context",
        "ai_gateway" => "platform_admin.personalization.stage.ai_gateway",
        "provider" => "platform_admin.personalization.stage.provider",
        "validating_response" => "platform_admin.personalization.stage.validating_response",
        "saving" => "platform_admin.personalization.stage.saving",
        "ready" | "succeeded" => "platform_admin.personalization.stage.ready",
        "failed" => "platform_admin.personalization.stage.failed",
        "cancelled" => "platform_admin.personalization.stage.cancelled",
        _ => "platform_admin.personalization.stage.queued",
    };
    t(key, locale)
}

fn delivery_label(value: &str, locale: Locale) -> String {
    match value {
        "allow_original_fallback" => t("platform_admin.personalization.allow_fallback", locale),
        _ => t(
            "platform_admin.personalization.require_personalized",
            locale,
        ),
    }
}

fn failure_label(code: Option<&str>, locale: Locale) -> String {
    let key = match code {
        Some("gateway_unavailable") => {
            "platform_admin.personalization.failure.gateway_unavailable"
        }
        Some("provider_unconfigured") => {
            "platform_admin.personalization.failure.provider_unconfigured"
        }
        Some("rate_limited") => "platform_admin.personalization.failure.rate_limited",
        Some("invalid_gateway_response") => {
            "platform_admin.personalization.failure.invalid_gateway_response"
        }
        Some("processing_unavailable") => {
            "platform_admin.personalization.failure.processing_unavailable"
        }
        Some("content_rejected") => "platform_admin.personalization.failure.content_rejected",
        Some("authorization_revoked") => {
            "platform_admin.personalization.failure.authorization_revoked"
        }
        Some("policy_disabled") => "platform_admin.personalization.failure.policy_disabled",
        _ => "platform_admin.personalization.failure.other",
    };
    t(key, locale)
}

#[component]
pub fn PlatformPersonalizationSection() -> Element {
    let locale = use_locale().current();
    let mut selected_school = use_signal(|| None::<(String, String)>);
    let mut overview =
        use_resource(move || async move { get_admin_personalization_overview().await });
    let mut teachers = use_resource(move || {
        let selected = selected_school();
        async move {
            match selected {
                Some((school_id, _)) => list_admin_personalization_teachers(school_id).await,
                None => Ok(Vec::new()),
            }
        }
    });

    rsx! {
        DashboardSection {
            title: t("platform_admin.personalization.title", locale),
            description: Some(t("platform_admin.personalization.description", locale)),
            children: rsx! {
                div { class: "space-y-6",
                    div { class: "flex justify-end",
                        button {
                            r#type: "button",
                            class: "rounded-lg border border-gray-300 px-3 py-2 text-sm font-medium dark:border-gray-700",
                            onclick: move |_| {
                                overview.restart();
                                if selected_school().is_some() {
                                    teachers.restart();
                                }
                            },
                            {t("platform_admin.personalization.refresh", locale)}
                        }
                    }

                    match overview.read().as_ref() {
                        None => rsx! {
                            p { class: "text-gray-500", role: "status",
                                {t("platform_admin.personalization.loading", locale)}
                            }
                        },
                        Some(Err(_)) => rsx! {
                            div { class: "rounded-lg bg-red-50 p-4 text-sm text-red-700 dark:bg-red-900/20 dark:text-red-200", role: "alert",
                                p { {t("platform_admin.personalization.load_error", locale)} }
                                button {
                                    r#type: "button",
                                    class: "mt-3 font-semibold underline",
                                    onclick: move |_| overview.restart(),
                                    {t("platform_admin.personalization.refresh", locale)}
                                }
                            }
                        },
                        Some(Ok(data)) => rsx! {
                            CapabilityPanel { capability: data.capability.clone() }

                            section { class: "space-y-3",
                                h2 { class: "text-lg font-semibold text-gray-900 dark:text-white",
                                    {t("platform_admin.personalization.schools", locale)}
                                }
                                div { class: "grid grid-cols-1 gap-4 xl:grid-cols-2",
                                    for policy in data.schools.iter() {
                                        SchoolPolicyCard {
                                            key: "{policy.school_id}-{policy.policy_version}",
                                            policy: policy.clone(),
                                            methods: data.methods.clone(),
                                            on_saved: move |_| {
                                                overview.restart();
                                                if selected_school().is_some() {
                                                    teachers.restart();
                                                }
                                            },
                                            on_manage_teachers: move |selection| {
                                                selected_school.set(Some(selection));
                                                teachers.restart();
                                            },
                                        }
                                    }
                                }
                            }

                            if let Some((_, school_name)) = selected_school() {
                                section { class: "space-y-3",
                                    div { class: "flex flex-wrap items-center justify-between gap-2",
                                        h2 { class: "text-lg font-semibold text-gray-900 dark:text-white",
                                            {format!("{} · {}", t("platform_admin.personalization.teachers", locale), school_name)}
                                        }
                                        button {
                                            r#type: "button",
                                            class: "rounded-lg border border-gray-300 px-3 py-1.5 text-sm dark:border-gray-700",
                                            onclick: move |_| selected_school.set(None),
                                            "×"
                                        }
                                    }
                                    match teachers.read().as_ref() {
                                        None => rsx! {
                                            p { class: "text-sm text-gray-500", {t("platform_admin.personalization.loading", locale)} }
                                        },
                                        Some(Err(_)) => rsx! {
                                            p { class: "text-sm text-red-600", role: "alert", {t("platform_admin.personalization.load_error", locale)} }
                                        },
                                        Some(Ok(items)) if items.is_empty() => rsx! {
                                            div { class: "et-ui-card p-5 text-sm text-gray-500",
                                                {t("platform_admin.personalization.no_teachers", locale)}
                                            }
                                        },
                                        Some(Ok(items)) => rsx! {
                                            div { class: "grid grid-cols-1 gap-4 xl:grid-cols-2",
                                                for item in items.iter() {
                                                    TeacherPolicyCard {
                                                        key: "{item.teacher_id}-{item.override_version}-{item.effective_version}-{item.effective_enabled}-{item.effective_paused}-{item.effective_llm_profile_id}-{item.effective_delivery_policy}",
                                                        policy: item.clone(),
                                                        methods: data.methods.clone(),
                                                        on_saved: move |_| {
                                                            teachers.restart();
                                                            overview.restart();
                                                        },
                                                    }
                                                }
                                            }
                                        },
                                    }
                                }
                            }

                            section { class: "space-y-3",
                                h2 { class: "text-lg font-semibold text-gray-900 dark:text-white",
                                    {t("platform_admin.personalization.jobs", locale)}
                                }
                                if data.recent_jobs.is_empty() {
                                    div { class: "et-ui-card p-6 text-sm text-gray-500",
                                        {t("platform_admin.personalization.jobs_empty", locale)}
                                    }
                                } else {
                                    div { class: "space-y-3",
                                        for job in data.recent_jobs.iter() {
                                            PersonalizationJobCard {
                                                key: "{job.job_id}",
                                                job: job.clone(),
                                                on_retried: move |_| overview.restart(),
                                            }
                                        }
                                    }
                                }
                            }
                        },
                    }
                }
            }
        }
    }
}

#[component]
fn CapabilityPanel(
    capability: api::server_functions::admin_personalization_functions::AdminPersonalizationCapabilityDto,
) -> Element {
    let locale = use_locale().current();
    let gateway_state = if capability.gateway_reachable {
        t("platform_admin.personalization.reachable", locale)
    } else {
        t("platform_admin.personalization.unreachable", locale)
    };
    let gateway_class = if capability.gateway_reachable {
        "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-200"
    } else {
        "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-200"
    };

    rsx! {
        section { class: "et-ui-card p-5",
            div { class: "flex flex-wrap items-center justify-between gap-3",
                h2 { class: "text-lg font-semibold text-gray-900 dark:text-white",
                    {t("platform_admin.personalization.capability", locale)}
                }
                span { class: "rounded-full px-2.5 py-1 text-xs font-semibold {gateway_class}",
                    {format!("{}: {}", t("platform_admin.personalization.gateway", locale), gateway_state)}
                }
            }
            dl { class: "mt-4 grid grid-cols-1 gap-3 text-sm sm:grid-cols-2 xl:grid-cols-6",
                CompactField { label: t("platform_admin.personalization.mode", locale), value: capability.llm_mode }
                CompactField { label: t("platform_admin.personalization.profile", locale), value: capability.llm_profile }
                CompactField { label: t("platform_admin.personalization.provider", locale), value: capability.provider }
                CompactField { label: t("platform_admin.personalization.model", locale), value: capability.model }
                CompactField { label: t("platform_admin.personalization.configured", locale), value: yes_no(capability.configured, locale) }
                CompactField { label: t("platform_admin.personalization.circuit", locale), value: capability.circuit }
            }
        }
    }
}

#[component]
fn SchoolPolicyCard(
    policy: AdminSchoolPersonalizationPolicyDto,
    methods: Vec<AdminPersonalizationMethodDto>,
    on_saved: EventHandler,
    on_manage_teachers: EventHandler<(String, String)>,
) -> Element {
    let locale = use_locale().current();
    let initial_enabled = policy.enabled;
    let initial_paused = policy.paused;
    let initial_profile = policy.llm_profile_id.clone();
    let initial_delivery = policy.delivery_policy.clone();
    let school_id = policy.school_id.clone();
    let school_name = policy.school_name.clone();

    let mut enabled = use_signal(move || initial_enabled);
    let mut paused = use_signal(move || initial_paused);
    let mut profile = use_signal(move || initial_profile);
    let mut delivery = use_signal(move || initial_delivery);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| false);

    rsx! {
        article { class: "et-ui-card p-5",
            div { class: "flex flex-wrap items-start justify-between gap-3",
                div {
                    h3 { class: "font-semibold text-gray-900 dark:text-white", "{policy.school_name}" }
                    p { class: "mt-1 text-xs text-gray-500",
                        {format!("{} · v{}", t("platform_admin.personalization.effective", locale), policy.policy_version)}
                    }
                }
                button {
                    r#type: "button",
                    class: "rounded-lg border border-gray-300 px-3 py-1.5 text-sm font-medium dark:border-gray-700",
                    onclick: move |_| on_manage_teachers.call((school_id.clone(), school_name.clone())),
                    {t("platform_admin.personalization.manage_teachers", locale)}
                }
            }

            div { class: "mt-4 grid grid-cols-1 gap-3 sm:grid-cols-2",
                label { class: "flex items-center gap-2 text-sm",
                    input {
                        r#type: "checkbox",
                        checked: enabled(),
                        disabled: busy(),
                        onchange: move |event| enabled.set(event.checked()),
                    }
                    span { {t("platform_admin.personalization.enabled", locale)} }
                }
                label { class: "flex items-center gap-2 text-sm",
                    input {
                        r#type: "checkbox",
                        checked: paused(),
                        disabled: busy(),
                        onchange: move |event| paused.set(event.checked()),
                    }
                    span { {t("platform_admin.personalization.paused", locale)} }
                }
                label { class: "text-sm",
                    span { class: "mb-1 block font-medium", {t("platform_admin.personalization.profile", locale)} }
                    select {
                        class: "w-full rounded-lg border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900",
                        value: "{profile}",
                        disabled: busy(),
                        onchange: move |event| profile.set(event.value()),
                        for method in methods.iter() {
                            option { value: "{method.profile_id}", "{method.profile_id} · {method.model}" }
                        }
                    }
                }
                label { class: "text-sm",
                    span { class: "mb-1 block font-medium", {t("platform_admin.personalization.delivery", locale)} }
                    select {
                        class: "w-full rounded-lg border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900",
                        value: "{delivery}",
                        disabled: busy(),
                        onchange: move |event| delivery.set(event.value()),
                        option { value: "require_personalized", {t("platform_admin.personalization.require_personalized", locale)} }
                        option { value: "allow_original_fallback", {t("platform_admin.personalization.allow_fallback", locale)} }
                    }
                }
            }
            if error() {
                p { class: "mt-3 text-sm text-red-600", role: "alert",
                    {t("platform_admin.personalization.save_error", locale)}
                }
            }
            button {
                r#type: "button",
                class: "mt-4 rounded-lg bg-primary px-4 py-2 text-sm font-semibold text-white disabled:opacity-50",
                disabled: busy(),
                onclick: move |_| {
                    if busy() {
                        return;
                    }
                    busy.set(true);
                    error.set(false);
                    let request = SetAdminSchoolPersonalizationPolicyRequest {
                        school_id: policy.school_id.clone(),
                        enabled: enabled(),
                        paused: paused(),
                        llm_profile_id: profile(),
                        delivery_policy: delivery(),
                    };
                    spawn(async move {
                        match set_admin_school_personalization_policy(request).await {
                            Ok(_) => on_saved.call(()),
                            Err(_) => error.set(true),
                        }
                        busy.set(false);
                    });
                },
                if busy() {
                    {t("platform_admin.personalization.saving", locale)}
                } else {
                    {t("platform_admin.personalization.save", locale)}
                }
            }
        }
    }
}

#[component]
fn TeacherPolicyCard(
    policy: AdminTeacherPersonalizationPolicyDto,
    methods: Vec<AdminPersonalizationMethodDto>,
    on_saved: EventHandler,
) -> Element {
    let locale = use_locale().current();
    let initial_mode = policy.mode.clone();
    let initial_enabled = policy.enabled_override.unwrap_or(policy.effective_enabled);
    let initial_paused = policy.paused_override.unwrap_or(policy.effective_paused);
    let initial_profile = policy
        .llm_profile_id_override
        .clone()
        .unwrap_or_else(|| policy.effective_llm_profile_id.clone());
    let initial_delivery = policy
        .delivery_policy_override
        .clone()
        .unwrap_or_else(|| policy.effective_delivery_policy.clone());

    let mut mode = use_signal(move || initial_mode);
    let mut enabled = use_signal(move || initial_enabled);
    let mut paused = use_signal(move || initial_paused);
    let mut profile = use_signal(move || initial_profile);
    let mut delivery = use_signal(move || initial_delivery);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| false);

    rsx! {
        article { class: "et-ui-card p-5",
            h3 { class: "font-semibold text-gray-900 dark:text-white", "{policy.teacher_name}" }
            p { class: "mt-1 text-xs text-gray-500",
                {format!(
                    "{}: {} · {} · {}",
                    t("platform_admin.personalization.effective", locale),
                    policy.effective_scope,
                    policy.effective_llm_profile_id,
                    delivery_label(&policy.effective_delivery_policy, locale),
                )}
            }

            label { class: "mt-4 block text-sm",
                span { class: "mb-1 block font-medium", {t("platform_admin.personalization.teachers", locale)} }
                select {
                    class: "w-full rounded-lg border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900",
                    value: "{mode}",
                    disabled: busy(),
                    onchange: move |event| mode.set(event.value()),
                    option { value: "inherit", {t("platform_admin.personalization.inherit", locale)} }
                    option { value: "override", {t("platform_admin.personalization.override", locale)} }
                }
            }

            if mode() == "override" {
                div { class: "mt-3 grid grid-cols-1 gap-3 sm:grid-cols-2",
                    label { class: "flex items-center gap-2 text-sm",
                        input {
                            r#type: "checkbox",
                            checked: enabled(),
                            disabled: busy(),
                            onchange: move |event| enabled.set(event.checked()),
                        }
                        span { {t("platform_admin.personalization.enabled", locale)} }
                    }
                    label { class: "flex items-center gap-2 text-sm",
                        input {
                            r#type: "checkbox",
                            checked: paused(),
                            disabled: busy(),
                            onchange: move |event| paused.set(event.checked()),
                        }
                        span { {t("platform_admin.personalization.paused", locale)} }
                    }
                    label { class: "text-sm",
                        span { class: "mb-1 block font-medium", {t("platform_admin.personalization.profile", locale)} }
                        select {
                            class: "w-full rounded-lg border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900",
                            value: "{profile}",
                            disabled: busy(),
                            onchange: move |event| profile.set(event.value()),
                            for method in methods.iter() {
                                option { value: "{method.profile_id}", "{method.profile_id} · {method.model}" }
                            }
                        }
                    }
                    label { class: "text-sm",
                        span { class: "mb-1 block font-medium", {t("platform_admin.personalization.delivery", locale)} }
                        select {
                            class: "w-full rounded-lg border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900",
                            value: "{delivery}",
                            disabled: busy(),
                            onchange: move |event| delivery.set(event.value()),
                            option { value: "require_personalized", {t("platform_admin.personalization.require_personalized", locale)} }
                            option { value: "allow_original_fallback", {t("platform_admin.personalization.allow_fallback", locale)} }
                        }
                    }
                }
            }

            if error() {
                p { class: "mt-3 text-sm text-red-600", role: "alert",
                    {t("platform_admin.personalization.save_error", locale)}
                }
            }
            button {
                r#type: "button",
                class: "mt-4 rounded-lg bg-primary px-4 py-2 text-sm font-semibold text-white disabled:opacity-50",
                disabled: busy(),
                onclick: move |_| {
                    if busy() {
                        return;
                    }
                    busy.set(true);
                    error.set(false);
                    let override_mode = mode() == "override";
                    let request = SetAdminTeacherPersonalizationPolicyRequest {
                        teacher_id: policy.teacher_id.clone(),
                        mode: mode(),
                        enabled_override: override_mode.then(|| enabled()),
                        paused_override: override_mode.then(|| paused()),
                        llm_profile_id_override: override_mode.then(|| profile()),
                        delivery_policy_override: override_mode.then(|| delivery()),
                    };
                    spawn(async move {
                        match set_admin_teacher_personalization_policy(request).await {
                            Ok(_) => on_saved.call(()),
                            Err(_) => error.set(true),
                        }
                        busy.set(false);
                    });
                },
                if busy() {
                    {t("platform_admin.personalization.saving", locale)}
                } else {
                    {t("platform_admin.personalization.save", locale)}
                }
            }
        }
    }
}

#[component]
fn PersonalizationJobCard(job: AdminPersonalizationJobDto, on_retried: EventHandler) -> Element {
    let locale = use_locale().current();
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| false);
    let retryable = matches!(job.status.as_str(), "failed" | "cancelled");
    let prompt_tokens = job
        .prompt_tokens
        .map(|value| value.to_string())
        .unwrap_or_else(|| "—".to_string());
    let completion_tokens = job
        .completion_tokens
        .map(|value| value.to_string())
        .unwrap_or_else(|| "—".to_string());
    let total_tokens = job
        .total_tokens
        .map(|value| value.to_string())
        .unwrap_or_else(|| "—".to_string());

    rsx! {
        article { class: "et-ui-card p-5",
            div { class: "flex flex-wrap items-start justify-between gap-3",
                div {
                    h3 { class: "font-semibold text-gray-900 dark:text-white", "{job.assignment_title}" }
                    p { class: "mt-1 text-sm text-gray-500", "{job.school_name} · {job.teacher_name}" }
                    p { class: "mt-1 text-xs text-gray-500", dir: "ltr",
                        {format!("{}: {}", t("platform_admin.personalization.student_ref", locale), job.student_reference)}
                    }
                }
                div { class: "flex flex-wrap gap-2",
                    span { class: "rounded-full bg-gray-100 px-2.5 py-1 text-xs font-semibold dark:bg-gray-800",
                        {format!("{}: {}", t("platform_admin.personalization.status", locale), stage_label(&job.status, locale))}
                    }
                    span { class: "rounded-full bg-blue-50 px-2.5 py-1 text-xs font-semibold text-blue-800 dark:bg-blue-900/20 dark:text-blue-200",
                        {format!("{}: {}", t("platform_admin.personalization.stage", locale), stage_label(&job.processing_stage, locale))}
                    }
                }
            }

            dl { class: "mt-4 grid grid-cols-1 gap-3 text-sm sm:grid-cols-2 xl:grid-cols-4",
                CompactField {
                    label: t("platform_admin.personalization.profile", locale),
                    value: format!("{} · {} · {}", job.llm_profile_id, job.llm_provider, job.model_name),
                }
                CompactField {
                    label: t("platform_admin.personalization.delivery", locale),
                    value: format!("{} v{} · {}", job.policy_scope, job.policy_version, delivery_label(&job.delivery_policy, locale)),
                }
                CompactField {
                    label: t("platform_admin.personalization.attempts", locale),
                    value: job.attempt_count.to_string(),
                }
                CompactField {
                    label: t("platform_admin.personalization.tokens", locale),
                    value: format!("{prompt_tokens} / {completion_tokens} / {total_tokens}"),
                }
            }

            details { class: "mt-4 rounded-lg border border-gray-200 p-3 text-sm dark:border-gray-700",
                summary { class: "cursor-pointer font-medium",
                    {t("platform_admin.personalization.context", locale)}
                }
                dl { class: "mt-3 grid grid-cols-1 gap-2 sm:grid-cols-2 xl:grid-cols-3",
                    CompactField {
                        label: t("platform_admin.personalization.talent", locale),
                        value: job.talent_profile_present.map(|value| yes_no(value, locale)).unwrap_or_else(|| "—".to_string()),
                    }
                    CompactField {
                        label: t("platform_admin.personalization.teacher_reports", locale),
                        value: job.teacher_report_count.map(|value| value.to_string()).unwrap_or_else(|| "—".to_string()),
                    }
                    CompactField {
                        label: t("platform_admin.personalization.performance", locale),
                        value: job.performance_context_present.map(|value| yes_no(value, locale)).unwrap_or_else(|| "—".to_string()),
                    }
                    CompactField {
                        label: t("platform_admin.personalization.class_chunks", locale),
                        value: job.class_material_chunk_count.map(|value| value.to_string()).unwrap_or_else(|| "—".to_string()),
                    }
                    CompactField {
                        label: t("platform_admin.personalization.knowledge_chunks", locale),
                        value: job.governed_knowledge_chunk_count.map(|value| value.to_string()).unwrap_or_else(|| "—".to_string()),
                    }
                    CompactField {
                        label: t("platform_admin.personalization.changed", locale),
                        value: job.generated_content_changed.map(|value| yes_no(value, locale)).unwrap_or_else(|| "—".to_string()),
                    }
                }
            }

            if job.last_error_code.is_some() {
                div { class: "mt-4 rounded-lg bg-red-50 p-3 text-sm text-red-800 dark:bg-red-900/20 dark:text-red-200",
                    p { class: "font-medium", {t("platform_admin.personalization.error", locale)} }
                    p { class: "mt-1", "{failure_label(job.last_error_code.as_deref(), locale)}" }
                }
            }

            if retryable {
                button {
                    r#type: "button",
                    class: "mt-4 rounded-lg border border-gray-300 px-3 py-2 text-sm font-semibold disabled:opacity-50 dark:border-gray-700",
                    disabled: busy(),
                    onclick: move |_| {
                        if busy() {
                            return;
                        }
                        busy.set(true);
                        error.set(false);
                        let job_id = job.job_id.clone();
                        spawn(async move {
                            match retry_admin_personalization_job(job_id).await {
                                Ok(()) => on_retried.call(()),
                                Err(_) => error.set(true),
                            }
                            busy.set(false);
                        });
                    },
                    {t("platform_admin.personalization.retry", locale)}
                }
            }
            if error() {
                p { class: "mt-2 text-sm text-red-600", role: "alert",
                    {t("platform_admin.personalization.retry_error", locale)}
                }
            }
        }
    }
}

#[component]
fn CompactField(label: String, value: String) -> Element {
    rsx! {
        div { class: "rounded-lg bg-gray-50 px-3 py-2 dark:bg-gray-900/40",
            dt { class: "text-xs font-medium text-gray-500", "{label}" }
            dd { class: "mt-1 break-words font-medium text-gray-900 dark:text-white", dir: "auto", "{value}" }
        }
    }
}
