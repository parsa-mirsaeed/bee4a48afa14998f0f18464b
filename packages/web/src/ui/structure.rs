use dioxus::prelude::*;

#[component]
pub fn AppShell(children: Element) -> Element {
    rsx! { div { class: "et-app-shell", {children} } }
}

#[component]
pub fn PageHeader(
    title: String,
    eyebrow: Option<String>,
    description: Option<String>,
    actions: Option<Element>,
) -> Element {
    rsx! {
        header { class: "et-ui-page-header",
            div { class: "et-ui-page-header__copy",
                if let Some(eyebrow) = eyebrow {
                    p { class: "et-ui-eyebrow", "{eyebrow}" }
                }
                h1 { class: "et-ui-page-title", "{title}" }
                if let Some(description) = description {
                    p { class: "et-ui-page-description", "{description}" }
                }
            }
            if let Some(actions) = actions {
                div { class: "et-ui-page-header__actions", {actions} }
            }
        }
    }
}

#[component]
pub fn Section(title: Option<String>, description: Option<String>, children: Element) -> Element {
    rsx! {
        section { class: "et-ui-section",
            if title.is_some() || description.is_some() {
                div { class: "et-ui-section__header",
                    if let Some(title) = title {
                        h2 { class: "et-ui-section__title", "{title}" }
                    }
                    if let Some(description) = description {
                        p { class: "et-ui-section__description", "{description}" }
                    }
                }
            }
            {children}
        }
    }
}

#[component]
pub fn Card(children: Element, class: Option<String>) -> Element {
    let class = class.unwrap_or_default();
    rsx! { div { class: "et-ui-card {class}", {children} } }
}

#[component]
pub fn Panel(children: Element, class: Option<String>) -> Element {
    let class = class.unwrap_or_default();
    rsx! { div { class: "et-ui-panel {class}", {children} } }
}

#[component]
pub fn Divider() -> Element {
    rsx! { div { class: "et-ui-divider", role: "separator" } }
}

#[component]
pub fn Stack(children: Element, gap: Option<String>, class: Option<String>) -> Element {
    let gap = gap.unwrap_or_else(|| "md".to_string());
    let class = class.unwrap_or_default();
    rsx! { div { class: "et-ui-stack et-ui-stack--{gap} {class}", {children} } }
}

#[component]
pub fn Grid(children: Element, columns: Option<u8>, class: Option<String>) -> Element {
    let columns = columns.unwrap_or(2).clamp(1, 4);
    let class = class.unwrap_or_default();
    rsx! { div { class: "et-ui-grid et-ui-grid--{columns} {class}", {children} } }
}
