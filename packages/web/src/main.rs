use dioxus::prelude::*;

mod application;
mod components;
mod domain;
mod i18n;
mod infrastructure;
mod ui;
mod utils;
mod views;

#[cfg(test)]
mod product_truthfulness_tests;

pub use i18n::{t, use_locale, LanguageSwitcher, Locale, LocaleProvider, LocalizedGrade};

use application::{AuthHooks, RoutingService};
use ui::{DataState, DataStateKind};
use views::login::LoginPage;
use views::role_based::components::role_guard::AuthGuard;
use views::role_based::{
    ParentDashboard, PlatformAdminDashboard, SchoolManagerDashboard, StudentDashboard,
    TeacherDashboard,
};

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[route("/")]
    LoginPage,

    #[route("/dashboard")]
    DashboardRoute,

    #[route("/dashboard/:section")]
    DashboardSectionRoute { section: String },

    #[route("/admin")]
    AdminRedirect,
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const DASHBOARD_REMAKE_CSS: Asset = asset!("/assets/dashboard-remake.css");
const DESIGN_SYSTEM_CSS: Asset = asset!("/assets/design-system.css");
const DESIGN_SYSTEM_COMPAT_CSS: Asset = asset!("/assets/design-system-compat.css");

#[cfg(feature = "server")]
async fn database_readiness(
    axum::Extension(state): axum::Extension<api::app_state::AppState>,
) -> Result<&'static str, axum::http::StatusCode> {
    api::readiness::check_database(&state)
        .await
        .map(|_| "ready")
        .map_err(|error| {
            tracing::warn!(error = %error, "Database readiness probe failed");
            axum::http::StatusCode::SERVICE_UNAVAILABLE
        })
}

#[cfg(feature = "server")]
#[tokio::main]
async fn main() {
    use axum::routing::post;
    use axum::Extension;
    use tracing::Level;

    let log_level = match std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "info".into())
        .as_str()
    {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" | "warning" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };
    dioxus_logger::init(log_level).ok();

    let config = api::config::Config::from_env().expect("Failed to load config");
    api::app_state::initialize_app_state(config)
        .await
        .expect("Failed to init state");

    let app_state = api::app_state::APP_STATE
        .get()
        .expect("App state should be initialized")
        .clone();

    let _knowledge_ingestion_worker = api::services::start_knowledge_ingestion_worker(
        app_state.services.raw_pool.clone(),
        app_state.services.pool.clone(),
    );
    let _assignment_personalization_worker = api::services::start_assignment_personalization_worker(
        app_state.services.raw_pool.clone(),
        app_state.services.pool.clone(),
    );

    let router = axum::Router::new()
        .route("/healthz", axum::routing::get(|| async { "ok" }))
        .route("/readyz", axum::routing::get(database_readiness))
        .route("/api/auth/login", post(api::handlers::login_handler))
        .route("/api/auth/logout", post(api::handlers::logout_handler))
        .route(
            "/api/manager/knowledge-submissions/upload",
            post(api::handlers::knowledge_upload_handler).layer(
                axum::extract::DefaultBodyLimit::max(
                    api::handlers::MAX_KNOWLEDGE_UPLOAD_BODY_BYTES,
                ),
            ),
        )
        .route(
            "/api/admin/knowledge-assets/source",
            axum::routing::get(api::handlers::knowledge_source_handler),
        )
        .serve_dioxus_application(ServeConfig::builder(), App)
        .layer(axum::middleware::from_fn(
            api::middleware::block_legacy_teacher_material_ingestion,
        ))
        .layer(axum::middleware::from_fn(
            api::middleware::endpoint_authorization_middleware,
        ))
        .layer(axum::middleware::from_fn(
            api::middleware::auth_guard::auth_middleware,
        ))
        .layer(Extension(app_state));

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8080);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, router).await.unwrap();
}

#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    crate::utils::cache::init_app_cache();

    use_effect(move || {
        spawn(async move {
            if let Err(error) = application::SessionUtils::initialize_session().await {
                web_sys::console::error_1(
                    &format!("Failed to initialize session: {error:?}").into(),
                );
            }
        });
    });

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: DASHBOARD_REMAKE_CSS }
        document::Link { rel: "stylesheet", href: DESIGN_SYSTEM_CSS }
        document::Link { rel: "stylesheet", href: DESIGN_SYSTEM_COMPAT_CSS }
        LocaleProvider { Router::<Route> {} }
    }
}

#[component]
fn DashboardRoute() -> Element {
    rsx! {
        AuthGuard {
            fallback: None,
            children: rsx! { RoleBasedDashboard { requested_section: None } }
        }
    }
}

#[component]
fn DashboardSectionRoute(section: String) -> Element {
    rsx! {
        AuthGuard {
            fallback: None,
            children: rsx! { RoleBasedDashboard { requested_section: Some(section) } }
        }
    }
}

#[component]
fn RoleBasedDashboard(requested_section: Option<String>) -> Element {
    let current_user = AuthHooks::use_current_user();
    let locale = use_locale();

    match current_user {
        Ok(Some(user)) => {
            let requested = requested_section.as_deref();
            let requested = if let Some(required_role) = requested.and_then(legacy_role_alias) {
                if required_role != user.role {
                    return rsx! {
                        DataState {
                            kind: DataStateKind::Permission,
                            title: locale.t("errors.access_denied"),
                            description: locale.t("errors.destination_unavailable"),
                        }
                    };
                }
                None
            } else {
                requested
            };

            match RoutingService::resolve_dashboard_section(&user, requested) {
                Ok(section) => match user.role {
                    domain::SystemRole::PlatformAdmin => {
                        rsx! { PlatformAdminDashboard { section } }
                    }
                    domain::SystemRole::SchoolManager => {
                        rsx! { SchoolManagerDashboard { section } }
                    }
                    domain::SystemRole::Teacher => rsx! { TeacherDashboard { section } },
                    domain::SystemRole::Student => rsx! { StudentDashboard { section } },
                    domain::SystemRole::Parent => rsx! { ParentDashboard { section } },
                },
                Err(()) => rsx! {
                    DataState {
                        kind: DataStateKind::Permission,
                        title: locale.t("errors.access_denied"),
                        description: locale.t("errors.destination_unavailable"),
                    }
                },
            }
        }
        Ok(None) => rsx! {
            DataState {
                kind: DataStateKind::Loading,
                title: locale.t("session.sign_in_required"),
                description: locale.t("session.sign_in_required_description"),
            }
        },
        Err(_) => rsx! {
            DataState {
                kind: DataStateKind::Error,
                title: locale.t("session.unavailable_title"),
                description: locale.t("session.unavailable_description"),
            }
        },
    }
}

fn legacy_role_alias(section: &str) -> Option<domain::SystemRole> {
    match section {
        "platform-admin" => Some(domain::SystemRole::PlatformAdmin),
        "school-manager" => Some(domain::SystemRole::SchoolManager),
        "teacher" => Some(domain::SystemRole::Teacher),
        "student" => Some(domain::SystemRole::Student),
        "parent" => Some(domain::SystemRole::Parent),
        _ => None,
    }
}

#[component]
fn AdminRedirect() -> Element {
    let nav = use_navigator();
    let locale = use_locale();

    use_effect(move || {
        nav.replace(Route::DashboardRoute {});
    });

    rsx! {
        DataState {
            kind: DataStateKind::Loading,
            title: locale.t("common.loading"),
            description: locale.t("navigation.redirecting_dashboard"),
        }
    }
}

#[cfg(test)]
mod route_tests {
    use super::*;

    #[test]
    fn legacy_role_aliases_remain_explicit_and_guardable() {
        assert_eq!(
            legacy_role_alias("teacher"),
            Some(domain::SystemRole::Teacher)
        );
        assert_eq!(
            legacy_role_alias("school-manager"),
            Some(domain::SystemRole::SchoolManager)
        );
        assert_eq!(legacy_role_alias("assignments"), None);
    }
}
