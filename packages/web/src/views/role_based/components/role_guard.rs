use crate::application::{AuthHooks, RoutingService};
use crate::domain::{SystemRole, User};
use crate::i18n::use_locale;
use crate::infrastructure::auth_provider::{CURRENT_USER_STATE, IS_INITIALIZING};
use crate::ui::{DataState, DataStateKind};
use crate::Route;
use dioxus::prelude::*;

fn use_login_redirect() {
    let nav = use_navigator();
    use_effect(move || {
        let initializing = *IS_INITIALIZING.read();
        let authenticated = CURRENT_USER_STATE.read().is_some();
        if !initializing && !authenticated {
            let _ = nav.push(Route::LoginPage {});
        }
    });
}

#[component]
fn GuardLoading() -> Element {
    let locale = use_locale();
    rsx! {
        DataState {
            kind: DataStateKind::Loading,
            title: locale.t("common.loading"),
            description: locale.t("session.checking"),
        }
    }
}

#[component]
pub fn RoleGuard(
    required_role: SystemRole,
    fallback: Option<Element>,
    children: Element,
) -> Element {
    use_login_redirect();
    if *IS_INITIALIZING.read() {
        return rsx! { GuardLoading {} };
    }

    match AuthHooks::use_current_user().ok().flatten() {
        Some(user) if user.role == required_role => rsx! { {children} },
        Some(_) if fallback.is_some() => fallback.unwrap(),
        Some(user) => rsx! { RoleAccessDeniedMessage { user, required_role } },
        None => rsx! { GuardLoading {} },
    }
}

#[component]
pub fn MultiRoleGuard(
    required_roles: Vec<SystemRole>,
    fallback: Option<Element>,
    children: Element,
) -> Element {
    use_login_redirect();
    if *IS_INITIALIZING.read() {
        return rsx! { GuardLoading {} };
    }

    match AuthHooks::use_current_user().ok().flatten() {
        Some(user) if required_roles.contains(&user.role) => rsx! { {children} },
        Some(_) if fallback.is_some() => fallback.unwrap(),
        Some(user) => rsx! { MultiRoleAccessDeniedMessage { user, required_roles } },
        None => rsx! { GuardLoading {} },
    }
}

#[component]
pub fn PermissionGuard(
    required_permission: String,
    fallback: Option<Element>,
    children: Element,
) -> Element {
    use_login_redirect();
    if *IS_INITIALIZING.read() {
        return rsx! { GuardLoading {} };
    }

    match AuthHooks::use_current_user().ok().flatten() {
        Some(user) if user.has_permission(&required_permission) => rsx! { {children} },
        Some(_) if fallback.is_some() => fallback.unwrap(),
        Some(user) => rsx! { PermissionDeniedMessage { user, required_permission } },
        None => rsx! { GuardLoading {} },
    }
}

#[component]
pub fn AuthGuard(fallback: Option<Element>, children: Element) -> Element {
    use_login_redirect();
    if *IS_INITIALIZING.read() {
        return rsx! { GuardLoading {} };
    }

    if AuthHooks::use_current_user().ok().flatten().is_some() {
        rsx! { {children} }
    } else if let Some(fallback_content) = fallback {
        fallback_content
    } else {
        rsx! { GuardLoading {} }
    }
}

#[component]
pub fn RouteGuard(route: String, fallback: Option<Element>, children: Element) -> Element {
    use_login_redirect();
    if *IS_INITIALIZING.read() {
        return rsx! { GuardLoading {} };
    }

    match AuthHooks::use_current_user().ok().flatten() {
        Some(user) if RoutingService::can_access_route(&user, &route) => rsx! { {children} },
        Some(_) if fallback.is_some() => fallback.unwrap(),
        Some(user) => rsx! { RouteAccessDeniedMessage { user, route } },
        None => rsx! { GuardLoading {} },
    }
}

#[component]
fn DeniedState(user: User) -> Element {
    let locale = use_locale();
    let nav = use_navigator();
    let dashboard_route = RoutingService::get_role_based_route(&user).to_string();
    rsx! {
        DataState {
            kind: DataStateKind::Permission,
            title: locale.t("errors.access_denied"),
            description: locale.t("errors.destination_unavailable"),
            action_label: locale.t("nav.dashboard"),
            on_action: move |_| {
                let _ = nav.push(dashboard_route.clone());
            },
        }
    }
}

#[component]
pub fn RoleAccessDeniedMessage(user: User, required_role: SystemRole) -> Element {
    let _ = required_role;
    rsx! { DeniedState { user } }
}

#[component]
pub fn MultiRoleAccessDeniedMessage(user: User, required_roles: Vec<SystemRole>) -> Element {
    let _ = required_roles;
    rsx! { DeniedState { user } }
}

#[component]
pub fn PermissionDeniedMessage(user: User, required_permission: String) -> Element {
    let _ = required_permission;
    rsx! { DeniedState { user } }
}

#[component]
pub fn RouteAccessDeniedMessage(user: User, route: String) -> Element {
    let _ = route;
    rsx! { DeniedState { user } }
}

#[component]
pub fn NotAuthenticatedMessage() -> Element {
    let locale = use_locale();
    let nav = use_navigator();
    rsx! {
        DataState {
            kind: DataStateKind::Permission,
            title: locale.t("session.sign_in_required"),
            description: locale.t("session.sign_in_required_description"),
            action_label: locale.t("auth.sign_in"),
            on_action: move |_| {
                let _ = nav.push(Route::LoginPage {});
            },
        }
    }
}

#[component]
pub fn LoadingSpinner() -> Element {
    rsx! { GuardLoading {} }
}

#[component]
pub fn AdminOnly(fallback: Option<Element>, children: Element) -> Element {
    rsx! {
        RoleGuard {
            required_role: SystemRole::SchoolManager,
            fallback,
            children,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_user(role: SystemRole) -> User {
        User::new(
            "test-1".to_string(),
            "test@example.com".to_string(),
            role,
            None,
        )
    }

    #[test]
    fn role_guard_role_comparison_is_explicit() {
        let manager = create_test_user(SystemRole::SchoolManager);
        let teacher = create_test_user(SystemRole::Teacher);
        assert_eq!(manager.role, SystemRole::SchoolManager);
        assert_ne!(teacher.role, SystemRole::SchoolManager);
    }

    #[test]
    fn permission_checking_uses_the_domain_user() {
        let manager = create_test_user(SystemRole::SchoolManager);
        assert!(manager.has_permission("manage_users") || !manager.get_permissions().is_empty());
    }
}
