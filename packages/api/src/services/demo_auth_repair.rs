use crate::app_state::AppState;
use crate::session_security::resolve_active_session;
use anyhow::{anyhow, bail, ensure, Context, Result};
use reqwest::{Method, RequestBuilder};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

const ENABLE_ENV: &str = "EDUTALENT_DEMO_AUTH_REPAIR";
const ENABLE_VALUE: &str = "reconcile-five-example-test-users-v1";
const STALE_EMAILS_ENV: &str = "EDUTALENT_DEMO_AUTH_REPAIR_STALE_EMAILS";
const MIN_PASSWORD_LEN: usize = 20;
const LIST_PAGE_SIZE: usize = 100;
const MAX_LIST_PAGES: usize = 100;

#[derive(Clone, Copy, Debug)]
struct DemoAccountSpec {
    role: &'static str,
    email_env: &'static str,
    password_env: &'static str,
}

const ACCOUNT_SPECS: [DemoAccountSpec; 5] = [
    DemoAccountSpec {
        role: "PlatformAdmin",
        email_env: "EDUTALENT_DEMO_AUTH_ADMIN_EMAIL",
        password_env: "EDUTALENT_DEMO_AUTH_ADMIN_PASSWORD",
    },
    DemoAccountSpec {
        role: "SchoolManager",
        email_env: "EDUTALENT_DEMO_AUTH_MANAGER_EMAIL",
        password_env: "EDUTALENT_DEMO_AUTH_MANAGER_PASSWORD",
    },
    DemoAccountSpec {
        role: "Teacher",
        email_env: "EDUTALENT_DEMO_AUTH_TEACHER_EMAIL",
        password_env: "EDUTALENT_DEMO_AUTH_TEACHER_PASSWORD",
    },
    DemoAccountSpec {
        role: "Student",
        email_env: "EDUTALENT_DEMO_AUTH_STUDENT_EMAIL",
        password_env: "EDUTALENT_DEMO_AUTH_STUDENT_PASSWORD",
    },
    DemoAccountSpec {
        role: "Parent",
        email_env: "EDUTALENT_DEMO_AUTH_PARENT_EMAIL",
        password_env: "EDUTALENT_DEMO_AUTH_PARENT_PASSWORD",
    },
];

#[derive(Clone, Debug)]
struct DemoCredential {
    role: &'static str,
    email: String,
    password: String,
}

#[derive(Clone, Debug)]
struct CanonicalAccount {
    id: String,
    email: String,
    role: String,
    password: String,
}

#[derive(Clone, Debug, Deserialize)]
struct RestUser {
    id: String,
    email: String,
    role_id: Value,
    is_active: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct RestRole {
    id: Value,
    name: String,
}

#[derive(Clone, Debug, Deserialize)]
struct AdminUser {
    id: String,
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AdminUserList {
    users: Vec<AdminUser>,
}

#[derive(Debug, Deserialize)]
struct PasswordGrantResponse {
    access_token: String,
}

/// Reconcile only the explicitly configured five `@example.test` demo users.
///
/// This maintenance path is disabled unless `EDUTALENT_DEMO_AUTH_REPAIR` is set
/// to the exact activation value. It has no public route, never logs credentials,
/// preserves the canonical application UUIDs, and verifies the normal session
/// resolver before startup continues.
pub async fn run_demo_auth_repair_if_enabled(state: &AppState) -> Result<()> {
    if std::env::var(ENABLE_ENV).ok().as_deref() != Some(ENABLE_VALUE) {
        return Ok(());
    }

    tracing::warn!("One-time demo authentication reconciliation is enabled");

    let credentials = load_credentials()?;
    let stale_emails = load_stale_emails()?;
    let roles = fetch_roles(state).await?;

    let mut canonical = Vec::with_capacity(credentials.len());
    for credential in credentials {
        canonical.push(resolve_canonical_account(state, credential, &roles).await?);
    }

    let canonical_ids: HashSet<String> = canonical.iter().map(|item| item.id.clone()).collect();
    let canonical_emails: HashSet<String> = canonical
        .iter()
        .map(|item| item.email.to_ascii_lowercase())
        .collect();

    let auth_users = list_auth_users(state).await?;

    // Remove only stale aliases explicitly supplied by the operator and wrong-ID
    // duplicates that currently own one of the five canonical demo addresses.
    for auth_user in &auth_users {
        let email = auth_user
            .email
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .to_ascii_lowercase();
        let stale_alias = stale_emails.contains(&email);
        let wrong_canonical_owner =
            canonical_emails.contains(&email) && !canonical_ids.contains(&auth_user.id);

        if stale_alias || wrong_canonical_owner {
            delete_auth_user(state, &auth_user.id).await?;
            tracing::info!(
                auth_user_id = %auth_user.id,
                email = %email,
                reason = if stale_alias { "stale-demo-alias" } else { "wrong-canonical-owner" },
                "Removed stale demo Supabase Auth identity"
            );
        }
    }

    // If canonical UUIDs are present but cross-wired to another canonical demo
    // email, release those unique emails first so each final update can be atomic.
    let auth_users = list_auth_users(state).await?;
    for account in &canonical {
        let Some(existing) = auth_users.iter().find(|user| user.id == account.id) else {
            continue;
        };
        let current_email = existing
            .email
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .to_ascii_lowercase();
        if current_email != account.email.to_ascii_lowercase()
            && canonical_emails.contains(&current_email)
        {
            let temporary_email = format!("edutalent-repair-{}@example.test", account.id);
            update_auth_user(state, &account.id, &temporary_email, None, false, None).await?;
            tracing::info!(
                auth_user_id = %account.id,
                role = %account.role,
                "Temporarily released a cross-wired demo email"
            );
        }
    }

    for account in &canonical {
        reconcile_account(state, account).await?;
    }

    for account in &canonical {
        verify_account(state, account).await?;
    }

    tracing::warn!(
        repaired_accounts = canonical.len(),
        "One-time demo authentication reconciliation completed; disable the repair environment flag"
    );
    Ok(())
}

fn load_credentials() -> Result<Vec<DemoCredential>> {
    let mut credentials = Vec::with_capacity(ACCOUNT_SPECS.len());
    let mut seen_emails = HashSet::new();

    for spec in ACCOUNT_SPECS {
        let email = required_env(spec.email_env)?.trim().to_ascii_lowercase();
        validate_demo_email(&email)?;
        ensure!(
            seen_emails.insert(email.clone()),
            "duplicate demo auth repair email: {email}"
        );

        let password = required_env(spec.password_env)?;
        validate_demo_password(&password)?;
        credentials.push(DemoCredential {
            role: spec.role,
            email,
            password,
        });
    }

    Ok(credentials)
}

fn load_stale_emails() -> Result<HashSet<String>> {
    let raw = std::env::var(STALE_EMAILS_ENV).unwrap_or_default();
    let mut emails = HashSet::new();
    for value in raw
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let email = value.to_ascii_lowercase();
        validate_demo_email(&email)?;
        emails.insert(email);
    }
    Ok(emails)
}

fn required_env(name: &str) -> Result<String> {
    let value = std::env::var(name)
        .with_context(|| format!("{name} is required when demo auth repair is enabled"))?;
    ensure!(!value.trim().is_empty(), "{name} must not be empty");
    Ok(value)
}

fn validate_demo_email(email: &str) -> Result<()> {
    ensure!(
        email.ends_with("@example.test") && email.len() > "@example.test".len(),
        "demo auth repair is restricted to @example.test identities"
    );
    Ok(())
}

fn validate_demo_password(password: &str) -> Result<()> {
    ensure!(
        password.len() >= MIN_PASSWORD_LEN,
        "demo auth repair passwords must be at least {MIN_PASSWORD_LEN} characters"
    );
    ensure!(
        password != "e2e-password",
        "the local E2E password is forbidden for the internet-facing demo"
    );
    Ok(())
}

fn validate_canonical_user_id(value: &str, role: &str) -> Result<()> {
    let parsed_id = Uuid::parse_str(value)
        .with_context(|| format!("canonical {role} user ID is not a UUID"))?;
    ensure!(
        !parsed_id.is_nil(),
        "canonical {role} user ID must not be the nil UUID"
    );
    Ok(())
}

async fn fetch_roles(state: &AppState) -> Result<HashMap<String, Value>> {
    let url = format!(
        "{}/rest/v1/roles",
        state.supabase_config.url.trim_end_matches('/')
    );
    let response = elevated_request(state, Method::GET, &url)
        .query(&[("select", "id,name")])
        .send()
        .await
        .context("failed to query canonical roles")?;
    let status = response.status();
    ensure!(
        status.is_success(),
        "canonical role query failed with HTTP {status}"
    );
    let roles = response
        .json::<Vec<RestRole>>()
        .await
        .context("failed to decode canonical roles")?;

    Ok(roles.into_iter().map(|role| (role.name, role.id)).collect())
}

async fn resolve_canonical_account(
    state: &AppState,
    credential: DemoCredential,
    roles: &HashMap<String, Value>,
) -> Result<CanonicalAccount> {
    let url = format!(
        "{}/rest/v1/users",
        state.supabase_config.url.trim_end_matches('/')
    );
    let email_filter = format!("eq.{}", credential.email);
    let response = elevated_request(state, Method::GET, &url)
        .query(&[
            ("select", "id,email,role_id,is_active"),
            ("email", email_filter.as_str()),
        ])
        .send()
        .await
        .with_context(|| {
            format!(
                "failed to query canonical demo account for {}",
                credential.role
            )
        })?;
    let status = response.status();
    ensure!(
        status.is_success(),
        "canonical demo account query for {} failed with HTTP {status}",
        credential.role
    );
    let users = response.json::<Vec<RestUser>>().await.with_context(|| {
        format!(
            "failed to decode canonical demo account for {}",
            credential.role
        )
    })?;
    ensure!(
        users.len() == 1,
        "expected exactly one canonical {} account for {}, found {}",
        credential.role,
        credential.email,
        users.len()
    );

    let user = users.into_iter().next().expect("length checked");
    ensure!(
        user.is_active,
        "canonical {} account is inactive",
        credential.role
    );
    let expected_role_id = roles
        .get(credential.role)
        .ok_or_else(|| anyhow!("canonical role {} does not exist", credential.role))?;
    ensure!(
        &user.role_id == expected_role_id,
        "canonical account {} does not have expected role {}",
        credential.email,
        credential.role
    );

    validate_canonical_user_id(&user.id, credential.role)?;
    ensure!(
        user.email.eq_ignore_ascii_case(&credential.email),
        "canonical email mismatch for {}",
        credential.role
    );

    Ok(CanonicalAccount {
        id: user.id,
        email: credential.email,
        role: credential.role.to_string(),
        password: credential.password,
    })
}

async fn list_auth_users(state: &AppState) -> Result<Vec<AdminUser>> {
    let mut all = Vec::new();
    for page in 1..=MAX_LIST_PAGES {
        let url = format!(
            "{}/auth/v1/admin/users",
            state.supabase_config.url.trim_end_matches('/')
        );
        let response = elevated_request(state, Method::GET, &url)
            .query(&[("page", page), ("per_page", LIST_PAGE_SIZE)])
            .send()
            .await
            .context("failed to list Supabase Auth users")?;
        let status = response.status();
        ensure!(
            status.is_success(),
            "Supabase Auth user listing failed with HTTP {status}"
        );
        let page_body = response
            .json::<AdminUserList>()
            .await
            .context("failed to decode Supabase Auth user listing")?;
        let count = page_body.users.len();
        all.extend(page_body.users);
        if count < LIST_PAGE_SIZE {
            return Ok(all);
        }
    }
    bail!("Supabase Auth user listing exceeded the safety pagination limit")
}

async fn reconcile_account(state: &AppState, account: &CanonicalAccount) -> Result<()> {
    let users = list_auth_users(state).await?;
    if users.iter().any(|user| user.id == account.id) {
        update_auth_user(
            state,
            &account.id,
            &account.email,
            Some(&account.password),
            true,
            Some(&account.role),
        )
        .await?;
        tracing::info!(
            auth_user_id = %account.id,
            email = %account.email,
            role = %account.role,
            "Reconciled existing demo Supabase Auth identity"
        );
    } else {
        create_auth_user(state, account).await?;
        tracing::info!(
            auth_user_id = %account.id,
            email = %account.email,
            role = %account.role,
            "Created canonical demo Supabase Auth identity"
        );
    }
    Ok(())
}

async fn create_auth_user(state: &AppState, account: &CanonicalAccount) -> Result<()> {
    let url = format!(
        "{}/auth/v1/admin/users",
        state.supabase_config.url.trim_end_matches('/')
    );
    let response = elevated_request(state, Method::POST, &url)
        .json(&json!({
            "id": account.id,
            "email": account.email,
            "password": account.password,
            "email_confirm": true,
            "user_metadata": {
                "edutalent_demo": true,
                "role": account.role,
            }
        }))
        .send()
        .await
        .with_context(|| format!("failed to create {} Supabase Auth identity", account.role))?;
    let status = response.status();
    ensure!(
        status.is_success(),
        "failed to create {} Supabase Auth identity: HTTP {status}",
        account.role
    );
    Ok(())
}

async fn update_auth_user(
    state: &AppState,
    user_id: &str,
    email: &str,
    password: Option<&str>,
    email_confirm: bool,
    role: Option<&str>,
) -> Result<()> {
    let url = format!(
        "{}/auth/v1/admin/users/{user_id}",
        state.supabase_config.url.trim_end_matches('/')
    );
    let mut body = json!({
        "email": email,
        "email_confirm": email_confirm,
    });
    if let Some(password) = password {
        body["password"] = Value::String(password.to_string());
    }
    if let Some(role) = role {
        body["user_metadata"] = json!({
            "edutalent_demo": true,
            "role": role,
        });
    }

    let response = elevated_request(state, Method::PUT, &url)
        .json(&body)
        .send()
        .await
        .context("failed to update Supabase Auth identity")?;
    let status = response.status();
    ensure!(
        status.is_success(),
        "Supabase Auth identity update failed with HTTP {status}"
    );
    Ok(())
}

async fn delete_auth_user(state: &AppState, user_id: &str) -> Result<()> {
    let url = format!(
        "{}/auth/v1/admin/users/{user_id}",
        state.supabase_config.url.trim_end_matches('/')
    );
    let response = elevated_request(state, Method::DELETE, &url)
        .send()
        .await
        .context("failed to delete stale Supabase Auth identity")?;
    let status = response.status();
    ensure!(
        status.is_success(),
        "Supabase Auth identity deletion failed with HTTP {status}"
    );
    Ok(())
}

async fn verify_account(state: &AppState, account: &CanonicalAccount) -> Result<()> {
    let url = format!(
        "{}/auth/v1/token?grant_type=password",
        state.supabase_config.url.trim_end_matches('/')
    );
    let response = state
        .services
        .http_client
        .post(url)
        .header("apikey", &state.supabase_config.publishable_key)
        .json(&json!({
            "email": account.email,
            "password": account.password,
        }))
        .send()
        .await
        .with_context(|| format!("failed to verify {} password grant", account.role))?;
    let status = response.status();
    ensure!(
        status.is_success(),
        "{} password grant verification failed with HTTP {status}",
        account.role
    );
    let grant = response
        .json::<PasswordGrantResponse>()
        .await
        .with_context(|| format!("failed to decode {} password grant", account.role))?;

    let (token_user_id, token_email) = state
        .services
        .supabase_service
        .validate_and_extract_user(&grant.access_token)
        .await
        .map_err(|error| anyhow!("{} JWT verification failed: {error}", account.role))?;
    ensure!(
        token_user_id == account.id,
        "{} JWT subject does not match canonical user ID",
        account.role
    );
    ensure!(
        token_email.eq_ignore_ascii_case(&account.email),
        "{} JWT email does not match canonical email",
        account.role
    );

    let active_session = resolve_active_session(state, &token_user_id)
        .await
        .map_err(|error| {
            anyhow!(
                "{} canonical session verification failed: {error}",
                account.role
            )
        })?;
    ensure!(
        active_session.user.role == account.role,
        "{} canonical session resolved as unexpected role {}",
        account.role,
        active_session.user.role
    );
    ensure!(
        active_session
            .user
            .email
            .eq_ignore_ascii_case(&account.email),
        "{} canonical session email mismatch",
        account.role
    );

    tracing::info!(
        auth_user_id = %account.id,
        email = %account.email,
        role = %account.role,
        "Verified demo login against the canonical application session"
    );
    Ok(())
}

fn elevated_request(state: &AppState, method: Method, url: &str) -> RequestBuilder {
    let secret = &state.supabase_config.secret_key;
    let mut request = state
        .services
        .http_client
        .request(method, url)
        .header("apikey", secret)
        .header(reqwest::header::CONTENT_TYPE, "application/json");

    // Legacy service_role keys are JWTs and historically require Bearer auth.
    // New sb_secret_* keys must be sent through `apikey` and must not be used as
    // bearer JWTs.
    if !secret.starts_with("sb_secret_") {
        request = request.bearer_auth(secret);
    }
    request
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repair_rejects_non_demo_email_addresses() {
        assert!(validate_demo_email("teacher@school.example").is_err());
        assert!(validate_demo_email("e2e-teacher-a@example.test").is_ok());
    }

    #[test]
    fn repair_rejects_local_e2e_and_short_passwords() {
        assert!(validate_demo_password("e2e-password").is_err());
        assert!(validate_demo_password("too-short").is_err());
        assert!(validate_demo_password("A-strong-demo-password-123!").is_ok());
    }

    #[test]
    fn canonical_id_validation_accepts_deterministic_fixture_uuid() {
        assert!(
            validate_canonical_user_id("b0000000-0000-0000-0000-0000000000a2", "Teacher").is_ok()
        );
    }

    #[test]
    fn canonical_id_validation_rejects_nil_or_malformed_uuid() {
        assert!(
            validate_canonical_user_id("00000000-0000-0000-0000-000000000000", "Teacher").is_err()
        );
        assert!(validate_canonical_user_id("not-a-uuid", "Teacher").is_err());
    }

    #[test]
    fn activation_requires_exact_one_time_value() {
        assert_ne!(ENABLE_VALUE, "1");
        assert_ne!(ENABLE_VALUE, "true");
        assert!(ENABLE_VALUE.contains("five-example-test-users"));
    }
}
