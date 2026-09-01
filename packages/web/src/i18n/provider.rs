//! Locale provider and language switching for the web application.

use super::{
    supplemental_translation, t, teacher_assignments_translation, teacher_dashboard_translation,
    Locale, LocalizedGrade,
};
use dioxus::prelude::*;

const LOCALE_STORAGE_KEY: &str = "edutalent_locale";

#[derive(Clone, Copy)]
pub struct LocaleContext {
    locale: Signal<Locale>,
}

impl LocaleContext {
    pub fn current(&self) -> Locale {
        *self.locale.read()
    }

    pub fn set(&mut self, locale: Locale) {
        *self.locale.write() = locale;
    }

    pub fn toggle(&mut self) {
        let next = match self.current() {
            Locale::En => Locale::Fa,
            Locale::Fa => Locale::En,
        };
        self.set(next);
    }

    pub fn is_rtl(&self) -> bool {
        self.current().is_rtl()
    }

    pub fn dir(&self) -> &'static str {
        self.current().dir_attr()
    }

    pub fn t(&self, key: &'static str) -> String {
        teacher_dashboard_translation(key, self.current())
            .or_else(|| teacher_assignments_translation(key, self.current()))
            .or_else(|| supplemental_translation(key, self.current()))
            .map(str::to_owned)
            .unwrap_or_else(|| t(key, self.current()))
    }

    pub fn format_grade(&self, value: f64) -> String {
        LocalizedGrade::new(value, self.current()).format_display()
    }

    pub fn max_grade(&self) -> f64 {
        self.current().max_grade()
    }
}

pub fn use_locale() -> LocaleContext {
    use_context::<LocaleContext>()
}

pub fn try_use_locale() -> Option<LocaleContext> {
    try_use_context::<LocaleContext>()
}

fn apply_document_locale(locale: Locale) {
    #[cfg(target_arch = "wasm32")]
    if let Some(document) = web_sys::window().and_then(|window| window.document()) {
        if let Some(root) = document.document_element() {
            let _ = root.set_attribute("lang", locale.code());
            let _ = root.set_attribute("dir", locale.dir_attr());
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    let _ = locale;
}

fn persisted_locale() -> Option<Locale> {
    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window()?;
        let storage = window.local_storage().ok()??;
        let lang = storage.get_item(LOCALE_STORAGE_KEY).ok()??;
        Locale::from_code(&lang)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

fn persist_locale(locale: Locale) {
    #[cfg(target_arch = "wasm32")]
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item(LOCALE_STORAGE_KEY, locale.code());
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    let _ = locale;
}

#[component]
pub fn LocaleProvider(children: Element) -> Element {
    // The initial client render must exactly match SSR. Browser storage is read
    // only after hydration, when a preference can safely replace the default.
    let stored_locale = use_signal(Locale::default);
    let mut hydrated_locale = stored_locale;

    let context = LocaleContext {
        locale: stored_locale,
    };
    use_context_provider(|| context);
    let locale = context.current();

    // SSR renders the deterministic default locale. Once the client hydrates,
    // reload the user's persisted preference so hydration never permanently
    // pins the server-side locale.
    use_effect(move || {
        if let Some(locale) = persisted_locale() {
            hydrated_locale.set(locale);
        }
    });
    // Read the signal inside the effect so the document attributes are updated
    // after hydration restores a persisted preference.
    use_effect(move || apply_document_locale(*stored_locale.read()));

    rsx! {
        div {
            class: "locale-wrapper",
            dir: "{locale.dir_attr()}",
            lang: "{locale.code()}",
            {children}
        }
    }
}

#[component]
pub fn LanguageSwitcher(
    #[props(default = "".to_string())] class: String,
    #[props(default = false)] dropdown: bool,
) -> Element {
    let mut locale_ctx = use_locale();
    let current = locale_ctx.current();
    let label = locale_ctx.t("common.select_language");

    if dropdown {
        rsx! {
            select {
                class: "language-switcher-dropdown {class}",
                value: "{current.code()}",
                "aria-label": "{label}",
                onchange: move |evt| {
                    if let Some(new_locale) = Locale::from_code(&evt.value()) {
                        locale_ctx.set(new_locale);
                        apply_document_locale(new_locale);
                        persist_locale(new_locale);
                    }
                },
                for locale in Locale::all().iter() {
                    option {
                        value: "{locale.code()}",
                        selected: *locale == current,
                        "{locale.native_name()}"
                    }
                }
            }
        }
    } else {
        rsx! {
            button {
                r#type: "button",
                class: "language-switcher-toggle {class}",
                "aria-label": "{label}",
                title: "{label}",
                onclick: move |_| {
                    locale_ctx.toggle();
                    let new_locale = locale_ctx.current();
                    apply_document_locale(new_locale);
                    persist_locale(new_locale);
                },
                span { class: "current-lang", "{current.native_name()}" }
                span { class: "material-icons-outlined text-sm et-language-switcher-icon", "aria-hidden": "true", "translate" }
            }
        }
    }
}

#[component]
pub fn T(translation_key: &'static str) -> Element {
    let locale_ctx = use_locale();
    let translated = locale_ctx.t(translation_key);
    rsx! { "{translated}" }
}

#[component]
pub fn GradeDisplay(
    value: f64,
    #[props(default = "".to_string())] class: String,
    #[props(default = true)] show_full: bool,
) -> Element {
    let locale_ctx = use_locale();
    let grade = LocalizedGrade::new(value, locale_ctx.current());
    let display = if show_full {
        grade.format_display()
    } else {
        grade.format_value()
    };
    rsx! { span { class: "grade-display {class}", "{display}" } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locale_context_creation() {
        assert_eq!(Locale::default(), Locale::Fa);
    }
}
