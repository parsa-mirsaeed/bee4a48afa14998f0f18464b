use crate::domain::{AccessControl, SystemRole, User};
use crate::ui::{DataState, DataStateKind};
use api::product_capabilities::PRODUCTION_PRODUCT_CAPABILITIES;
use dioxus::prelude::*;

/// Application routing service.
///
/// Authenticated product navigation uses one canonical role-aware route tree:
/// `/dashboard` for the role home and `/dashboard/<section>` for deep links.
/// Backend authorization remains authoritative; this service only constrains
/// which product destinations the current shell may present or render.
pub struct RoutingService;

impl RoutingService {
    pub fn get_role_based_route(_user: &User) -> &'static str {
        "/dashboard"
    }

    pub fn get_role_based_route_for_role(_role: SystemRole) -> &'static str {
        "/dashboard"
    }

    pub fn default_dashboard_section(user: &User) -> &'static str {
        if user.role == SystemRole::PlatformAdmin {
            "knowledge-assets"
        } else {
            "overview"
        }
    }

    pub fn canonical_dashboard_path(section: &str) -> String {
        if section == "overview" {
            "/dashboard".to_string()
        } else {
            format!("/dashboard/{section}")
        }
    }

    pub fn can_access_dashboard_section(user: &User, section: &str) -> bool {
        let capabilities = PRODUCTION_PRODUCT_CAPABILITIES;
        match user.role {
            SystemRole::PlatformAdmin => matches!(section, "knowledge-assets" | "knowledge-audit"),
            SystemRole::SchoolManager => match section {
                "overview"
                | "users"
                | "classes"
                | "knowledge-submissions"
                | "settings"
                | "profile" => true,
                "reports" => capabilities.school_manager_reports,
                _ => false,
            },
            SystemRole::Teacher => matches!(
                section,
                "overview"
                    | "classes"
                    | "assignments"
                    | "knowledge-assets"
                    | "submissions"
                    | "students"
            ),
            SystemRole::Student => match section {
                "overview" | "classes" | "assignments" | "grades" => true,
                "schedule" => capabilities.timetable,
                _ => false,
            },
            SystemRole::Parent => match section {
                "overview" | "children" => true,
                "reports" => capabilities.parent_reports,
                "communication" => capabilities.parent_teacher_communication,
                _ => false,
            },
        }
    }

    pub fn resolve_dashboard_section(user: &User, requested: Option<&str>) -> Result<String, ()> {
        let section = requested.unwrap_or_else(|| Self::default_dashboard_section(user));
        if Self::can_access_dashboard_section(user, section) {
            Ok(section.to_string())
        } else {
            Err(())
        }
    }

    /// Check if user can access a specific route.
    pub fn can_access_route(user: &User, route: &str) -> bool {
        if Self::is_public_route(route) {
            return true;
        }

        if route == "/dashboard" {
            return true;
        }

        if let Some(section) = route.strip_prefix("/dashboard/") {
            if !section.contains('/') {
                return Self::can_access_dashboard_section(user, section);
            }
        }

        if let Some(required_role) = Self::get_required_role_for_route(route) {
            return user.role == required_role;
        }

        if let Some(required_permission) = Self::get_required_permission_for_route(route) {
            return user.has_permission(required_permission);
        }

        Self::is_protected_route(route)
    }

    pub fn is_public_route(route: &str) -> bool {
        matches!(
            route,
            "/" | "/login" | "/forgot-password" | "/reset-password"
        )
    }

    pub fn is_protected_route(route: &str) -> bool {
        route == "/dashboard"
            || route.starts_with("/dashboard/")
            || route.starts_with("/profile")
            || route.starts_with("/settings")
    }

    /// Retained for guarded legacy aliases. Canonical section routes do not
    /// encode a role in the URL; the authenticated user's role selects the
    /// renderer and `can_access_dashboard_section` constrains the destination.
    pub fn get_required_role_for_route(route: &str) -> Option<SystemRole> {
        match route {
            value if value.starts_with("/dashboard/platform-admin/") => {
                Some(SystemRole::PlatformAdmin)
            }
            value if value.starts_with("/dashboard/school-manager/") => {
                Some(SystemRole::SchoolManager)
            }
            value if value.starts_with("/dashboard/teacher/") => Some(SystemRole::Teacher),
            value if value.starts_with("/dashboard/student/") => Some(SystemRole::Student),
            value if value.starts_with("/dashboard/parent/") => Some(SystemRole::Parent),
            _ => None,
        }
    }

    pub fn get_required_permission_for_route(route: &str) -> Option<&'static str> {
        match route {
            value if value.contains("/knowledge-assets") => Some("review_knowledge_assets"),
            value if value.contains("/reports") => Some("view_reports"),
            value if value.contains("/classes/manage") => Some("manage_classes"),
            value if value.contains("/assignments/create") => Some("create_assignments"),
            value if value.contains("/assignments/grade") => Some("grade_assignments"),
            value if value.contains("/admin") => Some("manage_users"),
            _ => None,
        }
    }

    pub fn get_navigation_items(
        user: &User,
        locale: &crate::i18n::LocaleContext,
    ) -> Vec<NavigationItem> {
        let capabilities = PRODUCTION_PRODUCT_CAPABILITIES;
        let mut items = Vec::new();

        if user.role != SystemRole::PlatformAdmin {
            items.push(NavigationItem::section(
                "overview",
                locale.t("nav.overview"),
                "grid_view",
            ));
        }

        match user.role {
            SystemRole::PlatformAdmin => {
                items.extend_from_slice(&[
                    NavigationItem::section(
                        "knowledge-assets",
                        locale.t("nav.knowledge_assets"),
                        "library_books",
                    ),
                    NavigationItem::section(
                        "knowledge-audit",
                        locale.t("nav.knowledge_audit"),
                        "policy",
                    ),
                ]);
            }
            SystemRole::SchoolManager => {
                items.extend_from_slice(&[
                    NavigationItem::section("users", locale.t("nav.user_management"), "groups"),
                    NavigationItem::section("classes", locale.t("nav.class_management"), "class"),
                    NavigationItem::section(
                        "knowledge-submissions",
                        locale.t("nav.knowledge_submissions"),
                        "upload_file",
                    ),
                ]);
                if capabilities.school_manager_reports {
                    items.push(NavigationItem::section(
                        "reports",
                        locale.t("nav.reports"),
                        "bar_chart",
                    ));
                }
                items.push(NavigationItem::section(
                    "settings",
                    locale.t("nav.settings"),
                    "settings",
                ));
                items.push(NavigationItem::section(
                    "profile",
                    locale.t("nav.profile"),
                    "person_outline",
                ));
            }
            SystemRole::Teacher => {
                items.extend_from_slice(&[
                    NavigationItem::section("classes", locale.t("nav.my_classes"), "class"),
                    NavigationItem::section(
                        "assignments",
                        locale.t("nav.assignments"),
                        "assignment",
                    ),
                    NavigationItem::section(
                        "knowledge-assets",
                        locale.t("nav.knowledge_assets"),
                        "library_books",
                    ),
                    NavigationItem::section("submissions", locale.t("nav.grading"), "grading"),
                    NavigationItem::section("students", locale.t("nav.students"), "people"),
                ]);
            }
            SystemRole::Student => {
                items.extend_from_slice(&[
                    NavigationItem::section("classes", locale.t("nav.my_classes"), "class"),
                    NavigationItem::section(
                        "assignments",
                        locale.t("nav.assignments"),
                        "assignment",
                    ),
                    NavigationItem::section("grades", locale.t("nav.grades"), "grade"),
                ]);
                if capabilities.timetable {
                    items.push(NavigationItem::section(
                        "schedule",
                        locale.t("schedule.title"),
                        "calendar_month",
                    ));
                }
            }
            SystemRole::Parent => {
                items.push(NavigationItem::section(
                    "children",
                    locale.t("nav.children"),
                    "child_care",
                ));
                if capabilities.parent_reports {
                    items.push(NavigationItem::section(
                        "reports",
                        locale.t("nav.reports"),
                        "description",
                    ));
                }
                if capabilities.parent_teacher_communication {
                    items.push(NavigationItem::section(
                        "communication",
                        locale.t("nav.communication"),
                        "chat",
                    ));
                }
            }
        }

        items
    }

    pub fn get_active_navigation_item<'a>(
        navigation_items: &'a [NavigationItem],
        current_section: &str,
    ) -> Option<&'a NavigationItem> {
        navigation_items
            .iter()
            .find(|item| item.id == current_section)
    }

    pub fn redirect_to_role_dashboard(user: &User) -> String {
        Self::get_role_based_route(user).to_string()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NavigationItem {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub route: String,
    pub active: bool,
}

impl NavigationItem {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        icon: impl Into<String>,
        route: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: icon.into(),
            route: route.into(),
            active: false,
        }
    }

    pub fn section(
        id: impl Into<String>,
        label: impl Into<String>,
        icon: impl Into<String>,
    ) -> Self {
        let id = id.into();
        Self::new(
            id.clone(),
            label,
            icon,
            RoutingService::canonical_dashboard_path(&id),
        )
    }

    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
}

/// Route guard components.
pub struct RouteGuard;

impl RouteGuard {
    pub fn require_route_access(
        route: String,
        fallback: Option<Element>,
        children: Element,
    ) -> Element {
        let mut current_user = use_signal(|| None);

        use_effect(move || {
            spawn(async move {
                if let Ok(Some(user)) = crate::application::AppAuthService::get_current_user().await
                {
                    current_user.set(Some(user));
                }
            });
        });

        rsx! {
            if let Some(user) = current_user.read().as_ref() {
                if RoutingService::can_access_route(user, &route) {
                    {children}
                } else if let Some(fallback_content) = fallback {
                    {fallback_content}
                } else {
                    DataState {
                        kind: DataStateKind::Permission,
                        title: "Access denied".to_string(),
                        description: "This destination is not available for your current role.".to_string(),
                    }
                }
            } else {
                DataState {
                    kind: DataStateKind::Loading,
                    title: "Loading".to_string(),
                    description: "Checking your session and access.".to_string(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(role: SystemRole) -> User {
        User::new(
            "route-test-user".to_string(),
            "route-test@example.test".to_string(),
            role,
            None,
        )
    }

    #[test]
    fn every_role_redirects_to_the_canonical_dashboard() {
        for role in [
            SystemRole::PlatformAdmin,
            SystemRole::SchoolManager,
            SystemRole::Teacher,
            SystemRole::Student,
            SystemRole::Parent,
        ] {
            assert_eq!(
                RoutingService::get_role_based_route_for_role(role),
                "/dashboard"
            );
        }
    }

    #[test]
    fn canonical_section_paths_are_history_safe_deep_links() {
        assert_eq!(
            RoutingService::canonical_dashboard_path("overview"),
            "/dashboard"
        );
        assert_eq!(
            RoutingService::canonical_dashboard_path("users"),
            "/dashboard/users"
        );
    }

    #[test]
    fn dashboard_section_matrix_rejects_role_mismatches() {
        let manager = user(SystemRole::SchoolManager);
        let teacher = user(SystemRole::Teacher);
        let student = user(SystemRole::Student);
        let parent = user(SystemRole::Parent);
        let platform = user(SystemRole::PlatformAdmin);

        assert!(RoutingService::can_access_dashboard_section(
            &manager, "users"
        ));
        assert!(!RoutingService::can_access_dashboard_section(
            &teacher, "users"
        ));
        assert!(RoutingService::can_access_dashboard_section(
            &teacher,
            "submissions"
        ));
        assert!(!RoutingService::can_access_dashboard_section(
            &student,
            "submissions"
        ));
        assert!(RoutingService::can_access_dashboard_section(
            &parent, "children"
        ));
        assert!(!RoutingService::can_access_dashboard_section(
            &parent,
            "assignments"
        ));
        assert!(RoutingService::can_access_dashboard_section(
            &platform,
            "knowledge-assets"
        ));
        assert!(!RoutingService::can_access_dashboard_section(
            &platform, "overview"
        ));
    }

    #[test]
    fn direct_dashboard_route_authorization_uses_section_matrix() {
        let teacher = user(SystemRole::Teacher);
        assert!(RoutingService::can_access_route(
            &teacher,
            "/dashboard/assignments"
        ));
        assert!(!RoutingService::can_access_route(
            &teacher,
            "/dashboard/users"
        ));
    }
}
