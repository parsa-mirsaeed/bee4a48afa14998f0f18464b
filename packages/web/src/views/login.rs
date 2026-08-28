use crate::application::{AppAuthService, AuthHooks};
use crate::domain::{AuthCredentials, AuthResult};
use crate::i18n::{use_locale, LanguageSwitcher};
use crate::ui::{
    Button, ButtonSize, ButtonVariant, Dialog, EmailField, FeedbackTone, InlineAlert, PasswordField,
};
use crate::Route;
use dioxus::prelude::*;

#[component]
pub fn LoginPage() -> Element {
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut email_error = use_signal(|| None::<String>);
    let mut password_error = use_signal(|| None::<String>);
    let mut form_error = use_signal(|| None::<String>);
    let mut is_loading = use_signal(|| false);
    let mut show_forgot_dialog = use_signal(|| false);
    let nav = use_navigator();
    let locale = use_locale();

    let is_authenticated = AuthHooks::use_is_authenticated();
    use_effect(move || {
        if is_authenticated {
            nav.replace(Route::DashboardRoute {});
        }
    });

    let t_welcome_back = locale.t("auth.welcome_back");
    let t_login_subtitle = locale.t("auth.login_subtitle");
    let t_email_label = locale.t("auth.email");
    let t_password_label = locale.t("auth.password");
    let t_forgot_password = locale.t("auth.forgot_password");
    let t_sign_in = locale.t("auth.sign_in");
    let t_signing_in = locale.t("auth.signing_in");
    let t_protected_by = locale.t("auth.protected_by");
    let t_invalid_credentials = locale.t("auth.invalid_credentials");
    let t_email_required = locale.t("auth.email_required");
    let t_email_invalid = locale.t("auth.email_invalid");
    let t_password_required = locale.t("auth.password_required");
    let t_account_inactive = locale.t("auth.account_inactive");
    let t_account_locked = locale.t("auth.account_locked");
    let t_email_not_confirmed = locale.t("auth.email_not_confirmed");
    let t_account_requires_admin = locale.t("auth.account_requires_admin");
    let t_service_unavailable = locale.t("auth.service_unavailable");

    rsx! {
        div { class: "et-auth-shell",
            aside { class: "et-auth-context", "aria-label": "EduTalent",
                div { class: "et-auth-context__brand",
                    span { class: "et-auth-context__mark",
                        span { class: "material-icons-outlined", "aria-hidden": "true", "school" }
                    }
                    span { "EduTalent" }
                }
                div {
                    h1 { class: "et-auth-context__headline", "{locale.t(\"auth.context_headline\")}" }
                    p { class: "et-auth-context__copy", "{locale.t(\"auth.context_copy\")}" }
                }
                p { class: "et-auth-context__footnote", "{locale.t(\"auth.offline_note\")}" }
            }

            main { class: "et-auth-panel",
                div { class: "et-auth-panel__top",
                    LanguageSwitcher { class: "et-auth-language".to_string() }
                }

                div { class: "et-auth-form-wrap",
                    h1 { class: "et-auth-title", "{t_welcome_back}" }
                    p { class: "et-auth-subtitle", "{t_login_subtitle}" }

                    form {
                        class: "et-auth-form",
                        action: "/api/auth/login",
                        method: "POST",
                        novalidate: true,
                        onsubmit: move |event| {
                            event.prevent_default();
                            if is_loading() {
                                return;
                            }

                            let email_value = email().trim().to_string();
                            let password_value = password();
                            let mut invalid = false;

                            email_error.set(None);
                            password_error.set(None);
                            form_error.set(None);

                            if email_value.is_empty() {
                                email_error.set(Some(t_email_required.clone()));
                                invalid = true;
                            } else if !email_value.contains('@') {
                                email_error.set(Some(t_email_invalid.clone()));
                                invalid = true;
                            }

                            if password_value.is_empty() {
                                password_error.set(Some(t_password_required.clone()));
                                invalid = true;
                            }

                            if invalid {
                                return;
                            }

                            is_loading.set(true);
                            let invalid_credentials = t_invalid_credentials.clone();
                            let account_inactive = t_account_inactive.clone();
                            let account_locked = t_account_locked.clone();
                            let email_not_confirmed = t_email_not_confirmed.clone();
                            let account_requires_admin = t_account_requires_admin.clone();
                            let service_unavailable = t_service_unavailable.clone();

                            spawn(async move {
                                let credentials = AuthCredentials {
                                    email: email_value,
                                    password: password_value,
                                };

                                match AppAuthService::login(credentials).await {
                                    AuthResult::Success(_) => {
                                        is_loading.set(false);
                                        nav.replace(Route::DashboardRoute {});
                                    }
                                    AuthResult::InvalidCredentials => {
                                        is_loading.set(false);
                                        form_error.set(Some(invalid_credentials));
                                    }
                                    AuthResult::AccountInactive => {
                                        is_loading.set(false);
                                        form_error.set(Some(account_inactive));
                                    }
                                    AuthResult::AccountLocked => {
                                        is_loading.set(false);
                                        form_error.set(Some(account_locked));
                                    }
                                    AuthResult::EmailNotConfirmed => {
                                        is_loading.set(false);
                                        form_error.set(Some(email_not_confirmed));
                                    }
                                    AuthResult::TemporaryPassword(_) => {
                                        is_loading.set(false);
                                        form_error.set(Some(account_requires_admin));
                                    }
                                    AuthResult::ServerError(_) => {
                                        is_loading.set(false);
                                        form_error.set(Some(service_unavailable));
                                    }
                                }
                            });
                        },

                        EmailField {
                            label: t_email_label,
                            value: email(),
                            name: "email".to_string(),
                            autocomplete: "email".to_string(),
                            placeholder: "you@example.com".to_string(),
                            required: true,
                            disabled: is_loading(),
                            error: email_error(),
                            on_change: move |value| {
                                email.set(value);
                                email_error.set(None);
                                form_error.set(None);
                            },
                        }

                        div {
                            PasswordField {
                                label: t_password_label,
                                value: password(),
                                name: "password".to_string(),
                                autocomplete: "current-password".to_string(),
                                reveal_label: locale.t("auth.reveal_password"),
                                hide_label: locale.t("auth.hide_password"),
                                required: true,
                                disabled: is_loading(),
                                error: password_error(),
                                on_change: move |value| {
                                    password.set(value);
                                    password_error.set(None);
                                    form_error.set(None);
                                },
                            }
                            button {
                                class: "et-auth-help",
                                r#type: "button",
                                disabled: is_loading(),
                                onclick: move |_| show_forgot_dialog.set(true),
                                "{t_forgot_password}"
                            }
                        }

                        if let Some(message) = form_error() {
                            InlineAlert {
                                message,
                                tone: FeedbackTone::Danger,
                            }
                        }

                        Button {
                            label: if is_loading() { t_signing_in.clone() } else { t_sign_in.clone() },
                            button_type: "submit".to_string(),
                            size: ButtonSize::Lg,
                            pending: is_loading(),
                            disabled: is_loading(),
                            icon: "arrow_forward".to_string(),
                        }
                    }

                    div { class: "et-auth-footer", "{t_protected_by}" }
                }
            }
        }

        Dialog {
            open: show_forgot_dialog(),
            title: locale.t("auth.recovery_unavailable_title"),
            close_label: locale.t("common.close"),
            on_close: move |_| show_forgot_dialog.set(false),
            children: rsx! {
                div { class: "et-ui-stack et-ui-stack--md",
                    InlineAlert {
                        message: locale.t("auth.recovery_unavailable_description"),
                        tone: FeedbackTone::Info,
                    }
                    Button {
                        label: locale.t("common.close"),
                        variant: ButtonVariant::Secondary,
                        onclick: move |_| show_forgot_dialog.set(false),
                    }
                }
            },
        }
    }
}