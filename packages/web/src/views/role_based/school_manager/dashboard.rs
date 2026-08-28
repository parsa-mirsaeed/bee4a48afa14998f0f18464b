use super::settings::profile::ProfileSettings;
use super::{
    ClassManagementSection, ManagerKnowledgeUploadSection, ReportsSection, SettingsSection,
    UserManagementSection,
};
use crate::application::AuthHooks;
use crate::i18n::{use_locale, Locale};
use crate::ui::{DataState, DataStateKind};
use crate::views::role_based::components::ResponsiveDashboardLayout;
use dioxus::prelude::*;

#[component]
pub fn SchoolManagerDashboard(section: String) -> Element {
    let current_user = AuthHooks::use_current_user().ok().flatten();
    let locale = use_locale();
    let nav = use_navigator();

    if let Some(user) = current_user {
        if !user.role.is_administrative() {
            return rsx! {
                DataState {
                    kind: DataStateKind::Permission,
                    title: locale.t("errors.access_denied"),
                    description: locale.t("errors.access_denied_description"),
                }
            };
        }

        let content = match section.as_str() {
            "users" => rsx! { UserManagementSection {} },
            "classes" => rsx! { ClassManagementSection {} },
            "knowledge-submissions" => rsx! { ManagerKnowledgeUploadSection {} },
            "settings" => rsx! { SettingsSection {} },
            "profile" => rsx! { ProfileSettings {} },
            "reports"
                if api::product_capabilities::PRODUCTION_PRODUCT_CAPABILITIES
                    .school_manager_reports =>
            {
                rsx! { ReportsSection {} }
            }
            _ => {
                let nav = nav.clone();
                rsx! {
                    SchoolManagerOverviewSection {
                        on_navigate: move |next: String| {
                            if next == "overview" {
                                nav.push(crate::Route::DashboardRoute {});
                            } else {
                                nav.push(crate::Route::DashboardSectionRoute { section: next });
                            }
                        },
                    }
                }
            }
        };

        rsx! {
            ResponsiveDashboardLayout {
                user,
                active_section: section,
                children: rsx! { {content} }
            }
        }
    } else {
        rsx! {
            DataState {
                kind: DataStateKind::Loading,
                title: locale.t("common.loading"),
                description: locale.t("session.checking"),
            }
        }
    }
}

#[component]
pub fn SchoolManagerOverviewSection(on_navigate: EventHandler<String>) -> Element {
    let locale = use_locale();
    let is_fa = locale.current() == Locale::Fa;
    let intro = if is_fa {
        "عملیات مدرسه را از مسیرهای واقعی و فعال سامانه مدیریت کنید. فقط قابلیت‌های در دسترس نمایش داده می‌شوند."
    } else {
        "Manage the school through production-backed workflows. Only available capabilities are shown."
    };
    let quick_actions = if is_fa {
        "اقدام‌های اصلی"
    } else {
        "Primary actions"
    };
    let knowledge_title = if is_fa {
        "ارسال منابع دانشی"
    } else {
        "Knowledge submissions"
    };
    let knowledge_description = if is_fa {
        "منابع کنترل‌شده مدرسه را برای بررسی ثبت کنید."
    } else {
        "Register governed school sources for review."
    };
    let truthfulness_note = if is_fa {
        "خلاصه‌های عملیاتی فقط زمانی نمایش داده می‌شوند که از داده واقعی مدرسه پشتیبانی شوند؛ شاخص‌های ساختگی فعالیت، پایداری، تأخیر یا روند نمایش داده نمی‌شوند."
    } else {
        "Operational summaries appear only when backed by real school data. Synthetic activity, uptime, latency, storage and trend metrics are intentionally omitted."
    };

    rsx! {
        div { class: "et-page-stack",
            header { class: "et-overview-intro",
                h2 { class: "et-overview-title", "{locale.t(\"dashboard.overview\")}" }
                p { class: "et-overview-copy", "{intro}" }
            }

            section { class: "et-section",
                div { class: "et-section-heading",
                    h3 { class: "et-section-title", "{quick_actions}" }
                }
                div { class: "et-action-grid",
                    ManagerAction {
                        icon: "groups".to_string(),
                        title: locale.t("school_manager.actions.add_user"),
                        description: locale.t("school_manager.actions.add_user_desc"),
                        on_click: move |_| on_navigate.call("users".to_string()),
                    }
                    ManagerAction {
                        icon: "class".to_string(),
                        title: locale.t("school_manager.actions.create_class"),
                        description: locale.t("school_manager.actions.create_class_desc"),
                        on_click: move |_| on_navigate.call("classes".to_string()),
                    }
                    ManagerAction {
                        icon: "upload_file".to_string(),
                        title: knowledge_title.to_string(),
                        description: knowledge_description.to_string(),
                        on_click: move |_| on_navigate.call("knowledge-submissions".to_string()),
                    }
                    ManagerAction {
                        icon: "settings".to_string(),
                        title: locale.t("school_manager.actions.system_settings"),
                        description: locale.t("school_manager.actions.system_settings_desc"),
                        on_click: move |_| on_navigate.call("settings".to_string()),
                    }
                }
            }

            div { class: "et-info-note",
                span { class: "material-icons-outlined text-lg", "aria-hidden": "true", "verified_user" }
                p { "{truthfulness_note}" }
            }
        }
    }
}

#[component]
fn ManagerAction(
    icon: String,
    title: String,
    description: String,
    on_click: EventHandler,
) -> Element {
    rsx! {
        button {
            class: "et-action-card",
            r#type: "button",
            onclick: move |_| on_click.call(()),
            div { class: "et-action-card-top",
                span { class: "et-action-icon",
                    span { class: "material-icons-outlined text-xl", "aria-hidden": "true", "{icon}" }
                }
                span { class: "material-icons-outlined et-action-arrow", "aria-hidden": "true", "arrow_forward" }
            }
            div {
                h3 { class: "et-action-title", "{title}" }
                p { class: "et-action-description", "{description}" }
            }
        }
    }
}
