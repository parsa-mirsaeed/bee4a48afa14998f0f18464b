use dioxus::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavItem {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub active: bool,
}

#[component]
pub fn SidebarNav(label: String, items: Vec<NavItem>, on_select: EventHandler<String>) -> Element {
    rsx! {
        nav { class: "et-ui-sidebar-nav", "aria-label": "{label}",
            ul { class: "et-ui-sidebar-nav__list",
                for item in items {
                    {
                        let id = item.id.clone();
                        let class = if item.active {
                            "et-ui-sidebar-nav__item et-ui-sidebar-nav__item--active"
                        } else {
                            "et-ui-sidebar-nav__item"
                        };
                        rsx! {
                            li { key: "{item.id}",
                                button {
                                    class,
                                    r#type: "button",
                                    "aria-current": if item.active { "page" } else { "false" },
                                    "aria-label": "{item.label}",
                                    onclick: move |_| on_select.call(id.clone()),
                                    span { class: "material-icons-outlined", "aria-hidden": "true", "{item.icon}" }
                                    span { class: "et-ui-sidebar-nav__label", "{item.label}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn MobileNavDrawer(open: bool, label: String, children: Element) -> Element {
    rsx! {
        div {
            class: if open { "et-ui-mobile-nav et-ui-mobile-nav--open" } else { "et-ui-mobile-nav" },
            "aria-label": "{label}",
            hidden: !open,
            {children}
        }
    }
}

#[component]
pub fn Breadcrumbs(items: Vec<(String, String)>) -> Element {
    rsx! {
        nav { class: "et-ui-breadcrumbs", "aria-label": "Breadcrumb",
            ol {
                for (index, (label, href)) in items.into_iter().enumerate() {
                    li { key: "{index}",
                        if href.is_empty() {
                            span { "aria-current": "page", "{label}" }
                        } else {
                            a { href: "{href}", "{label}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Tabs(
    label: String,
    items: Vec<(String, String)>,
    active: String,
    on_change: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "et-ui-tabs", role: "tablist", "aria-label": "{label}",
            for (id, item_label) in items {
                {
                    let selected = id == active;
                    let next = id.clone();
                    rsx! {
                        button {
                            key: "{id}",
                            class: if selected { "et-ui-tab et-ui-tab--active" } else { "et-ui-tab" },
                            r#type: "button",
                            role: "tab",
                            "aria-selected": if selected { "true" } else { "false" },
                            onclick: move |_| on_change.call(next.clone()),
                            "{item_label}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn SegmentedControl(
    label: String,
    items: Vec<(String, String)>,
    active: String,
    on_change: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "et-ui-segmented", role: "group", "aria-label": "{label}",
            for (id, item_label) in items {
                {
                    let selected = id == active;
                    let next = id.clone();
                    rsx! {
                        button {
                            key: "{id}",
                            class: if selected { "et-ui-segmented__item et-ui-segmented__item--active" } else { "et-ui-segmented__item" },
                            r#type: "button",
                            "aria-pressed": if selected { "true" } else { "false" },
                            onclick: move |_| on_change.call(next.clone()),
                            "{item_label}"
                        }
                    }
                }
            }
        }
    }
}
