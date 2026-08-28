use crate::domain::User;
use crate::i18n::use_locale;
use crate::ui::{
    Button, ButtonSize, ButtonVariant, DataState, DataStateKind, FeedbackTone, IconButton,
    InlineAlert, Popover,
};
use dioxus::prelude::*;

/// Compact product top bar with functional navigation and notification state.
#[component]
pub fn Header(user: User, page_title: String, on_open_navigation: EventHandler) -> Element {
    let locale = use_locale();

    rsx! {
        header { class: "et-topbar",
            div { class: "et-topbar-start",
                button {
                    class: "et-mobile-menu-button",
                    r#type: "button",
                    "aria-label": locale.t("navigation.open"),
                    onclick: move |_| on_open_navigation.call(()),
                    span { class: "material-icons-outlined", "aria-hidden": "true", "menu" }
                }

                div { class: "et-page-heading-wrap",
                    h1 { class: "et-topbar-title", "{page_title}" }
                    p { class: "et-topbar-role", "{user.role.display_name()}" }
                }
            }

            div { class: "et-topbar-end", NotificationDropdown {} }
        }
    }
}

#[component]
pub fn NotificationDropdown() -> Element {
    let mut is_open = use_signal(|| false);
    let mut mark_all_pending = use_signal(|| false);
    let mut busy_notification = use_signal(|| None::<String>);
    let mut operation_error = use_signal(|| None::<String>);
    let locale = use_locale();
    let t_notifications = locale.t("nav.notifications");
    let t_mark_all_read = locale.t("notifications.mark_all_read");
    let t_no_notifications = locale.t("notifications.no_new");
    let t_loading = locale.t("notifications.loading");
    let t_failed_load = locale.t("notifications.failed_load");

    let mut notification_resource = use_resource(move || async move {
        api::server_functions::notification_functions::get_unread_notifications(Some(10)).await
    });

    let notification_count = notification_resource
        .read()
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .map(|response| response.unread_count)
        .unwrap_or(0);
    let badge_text = if notification_count > 99 {
        "99+".to_string()
    } else {
        notification_count.to_string()
    };

    rsx! {
        div { class: "et-notification-surface",
            div { class: "et-notification-trigger",
                IconButton {
                    label: t_notifications.clone(),
                    icon: "notifications".to_string(),
                    expanded: is_open(),
                    onclick: move |_| {
                        operation_error.set(None);
                        is_open.set(!is_open());
                    },
                }
                if notification_count > 0 {
                    span { class: "et-notification-badge", "{badge_text}" }
                }
            }

            Popover {
                open: is_open(),
                label: t_notifications.clone(),
                class: "et-notification-panel".to_string(),
                on_close: move |_| is_open.set(false),
                children: rsx! {
                    div { class: "et-notification-panel__header",
                        h2 { class: "et-notification-panel__title", "{t_notifications}" }
                        if notification_count > 0 {
                            Button {
                                label: t_mark_all_read.clone(),
                                variant: ButtonVariant::Ghost,
                                size: ButtonSize::Sm,
                                pending: mark_all_pending(),
                                onclick: move |_| {
                                    if mark_all_pending() {
                                        return;
                                    }
                                    mark_all_pending.set(true);
                                    operation_error.set(None);
                                    let failed = locale.t("notifications.action_failed");
                                    spawn(async move {
                                        match api::server_functions::notification_functions::mark_all_notifications_as_read().await {
                                            Ok(_) => notification_resource.restart(),
                                            Err(_) => operation_error.set(Some(failed)),
                                        }
                                        mark_all_pending.set(false);
                                    });
                                },
                            }
                        }
                    }

                    if let Some(message) = operation_error() {
                        InlineAlert {
                            message,
                            tone: FeedbackTone::Danger,
                        }
                    }

                    div { class: "et-notification-panel__list",
                        match notification_resource.read().clone() {
                            None => rsx! {
                                DataState {
                                    kind: DataStateKind::Loading,
                                    title: t_loading.clone(),
                                    description: t_loading.clone(),
                                }
                            },
                            Some(Err(_)) => rsx! {
                                DataState {
                                    kind: DataStateKind::Error,
                                    title: t_failed_load.clone(),
                                    description: t_failed_load.clone(),
                                    action_label: locale.t("common.refresh"),
                                    on_action: move |_| notification_resource.restart(),
                                }
                            },
                            Some(Ok(response)) if response.notifications.is_empty() => rsx! {
                                DataState {
                                    kind: DataStateKind::Empty,
                                    title: t_no_notifications.clone(),
                                    description: t_no_notifications.clone(),
                                }
                            },
                            Some(Ok(response)) => rsx! {
                                for notification in response.notifications.into_iter() {
                                    {
                                        let notification_id = notification.id.to_string();
                                        let pending = busy_notification().as_deref() == Some(notification_id.as_str());
                                        rsx! {
                                            NotificationItem {
                                                key: "{notification.id}",
                                                icon: notification.icon.unwrap_or_else(|| "notifications".to_string()),
                                                title: notification.title,
                                                message: notification.message,
                                                time: format_time_ago(notification.created_at, &locale),
                                                is_read: notification.is_read,
                                                pending,
                                                on_click: move |_| {
                                                    if busy_notification().is_some() {
                                                        return;
                                                    }
                                                    let notification_id = notification_id.clone();
                                                    let failed = locale.t("notifications.action_failed");
                                                    busy_notification.set(Some(notification_id.clone()));
                                                    operation_error.set(None);
                                                    spawn(async move {
                                                        match api::server_functions::notification_functions::mark_notification_as_read(notification_id).await {
                                                            Ok(_) => notification_resource.restart(),
                                                            Err(_) => operation_error.set(Some(failed)),
                                                        }
                                                        busy_notification.set(None);
                                                    });
                                                }
                                            }
                                        }
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

fn format_time_ago(
    time: chrono::DateTime<chrono::Utc>,
    locale: &crate::i18n::LocaleContext,
) -> String {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(time);

    if duration.num_days() > 0 {
        locale
            .t("time.days_ago")
            .replace("{0}", &duration.num_days().to_string())
    } else if duration.num_hours() > 0 {
        locale
            .t("time.hours_ago")
            .replace("{0}", &duration.num_hours().to_string())
    } else if duration.num_minutes() > 0 {
        locale
            .t("time.minutes_ago")
            .replace("{0}", &duration.num_minutes().to_string())
    } else {
        locale.t("time.just_now")
    }
}

#[component]
pub fn NotificationItem(
    icon: String,
    title: String,
    message: String,
    time: String,
    is_read: bool,
    pending: bool,
    on_click: EventHandler,
) -> Element {
    let row_class = if is_read {
        "et-notification-item"
    } else {
        "et-notification-item et-notification-item--unread"
    };

    rsx! {
        button {
            r#type: "button",
            class: "{row_class}",
            disabled: pending,
            "aria-busy": if pending { "true" } else { "false" },
            onclick: move |_| {
                if !pending {
                    on_click.call(());
                }
            },
            div { class: "et-notification-item__icon",
                span { class: "material-icons-outlined", "aria-hidden": "true", "{icon}" }
            }
            div {
                div { class: "et-notification-item__title-row",
                    span { class: "et-notification-item__title", "{title}" }
                    span { class: "et-notification-item__time", "{time}" }
                }
                p { class: "et-notification-item__message", "{message}" }
            }
        }
    }
}

/// Kept for source compatibility with older imports. User controls now live in
/// the sidebar where identity, language and sign-out form one coherent group.
#[component]
pub fn UserDropdown(_user: User) -> Element {
    rsx! { div {} }
}
