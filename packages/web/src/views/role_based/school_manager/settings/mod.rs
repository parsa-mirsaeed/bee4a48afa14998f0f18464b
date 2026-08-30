use crate::i18n::use_locale;
use crate::views::role_based::components::DashboardSection;
use dioxus::prelude::*;

pub mod general;
pub mod notifications;
pub mod profile;
pub mod security;

use general::GeneralSettings;
use notifications::NotificationSettings;
use profile::ProfileSettings;
use security::SecuritySettings;

#[component]
pub fn SettingsSection() -> Element {
    let mut active_tab = use_signal(|| "profile".to_string());
    let locale = use_locale();
    let settings_label = locale.t("school_manager.settings.title");

    let tab = |id: &'static str, label: String, icon: &'static str| {
        let selected = active_tab() == id;
        let class = if selected {
            "et-ui-tab et-ui-tab--active"
        } else {
            "et-ui-tab"
        };
        rsx! {
            button {
                id: "manager-settings-tab-{id}",
                class: "{class}",
                r#type: "button",
                role: "tab",
                "aria-selected": if selected { "true" } else { "false" },
                "aria-controls": "manager-settings-panel-{id}",
                tabindex: if selected { "0" } else { "-1" },
                onclick: move |_| active_tab.set(id.to_string()),
                span { class: "material-icons-outlined", "aria-hidden": "true", "{icon}" }
                "{label}"
            }
        }
    };

    rsx! {
        DashboardSection {
            title: locale.t("school_manager.settings.title"),
            description: Some(locale.t("school_manager.settings.description")),
            children: rsx! {
                div { class: "et-ui-stack et-ui-stack--lg",
                    div {
                        class: "et-ui-tabs",
                        role: "tablist",
                        "aria-label": "{settings_label}",
                        {tab("profile", locale.t("school_manager.settings.tabs.profile"), "person_outline")}
                        {tab("security", locale.t("school_manager.settings.tabs.security"), "lock_outline")}
                        {tab("general", locale.t("school_manager.settings.tabs.general"), "settings")}
                        {tab("notifications", locale.t("school_manager.settings.tabs.notifications"), "notifications_none")}
                    }
                    div {
                        id: "manager-settings-panel-{active_tab}",
                        role: "tabpanel",
                        "aria-labelledby": "manager-settings-tab-{active_tab}",
                        tabindex: "0",
                        match active_tab().as_str() {
                            "profile" => rsx! { ProfileSettings {} },
                            "security" => rsx! { SecuritySettings {} },
                            "general" => rsx! { GeneralSettings {} },
                            "notifications" => rsx! { NotificationSettings {} },
                            _ => rsx! { ProfileSettings {} },
                        }
                    }
                }
            }
        }
    }
}
