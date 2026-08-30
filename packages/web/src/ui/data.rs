use crate::ui::feedback::FeedbackTone;
use dioxus::prelude::*;

#[component]
pub fn MetricCard(label: String, value: String, supporting: Option<String>) -> Element {
    rsx! {
        article { class: "et-ui-metric-card",
            p { class: "et-ui-metric-card__label", "{label}" }
            p { class: "et-ui-metric-card__value", "{value}" }
            if let Some(supporting) = supporting {
                p { class: "et-ui-metric-card__support", "{supporting}" }
            }
        }
    }
}

#[component]
pub fn DataList(items: Vec<(String, String)>) -> Element {
    rsx! {
        dl { class: "et-ui-data-list",
            for (label, value) in items {
                div { class: "et-ui-data-list__row",
                    dt { "{label}" }
                    dd { "{value}" }
                }
            }
        }
    }
}

#[component]
pub fn DataTable(caption: String, headers: Vec<String>, rows: Vec<Vec<String>>) -> Element {
    rsx! {
        div { class: "et-ui-table-scroll",
            table { class: "et-ui-table",
                caption { class: "sr-only", "{caption}" }
                thead {
                    tr { for header in headers { th { scope: "col", "{header}" } } }
                }
                tbody {
                    for (row_index, row) in rows.into_iter().enumerate() {
                        tr { key: "{row_index}",
                            for (cell_index, cell) in row.into_iter().enumerate() {
                                if cell_index == 0 {
                                    th { scope: "row", "{cell}" }
                                } else {
                                    td { "{cell}" }
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
pub fn MobileDataCard(
    title: String,
    items: Vec<(String, String)>,
    actions: Option<Element>,
) -> Element {
    rsx! {
        article { class: "et-ui-mobile-data-card",
            h3 { class: "et-ui-mobile-data-card__title", "{title}" }
            DataList { items }
            if let Some(actions) = actions {
                div { class: "et-ui-mobile-data-card__actions", {actions} }
            }
        }
    }
}

#[component]
pub fn StatusBadge(label: String, tone: Option<FeedbackTone>) -> Element {
    let class = match tone.unwrap_or(FeedbackTone::Neutral) {
        FeedbackTone::Neutral => "et-ui-tone--neutral",
        FeedbackTone::Info => "et-ui-tone--info",
        FeedbackTone::Success => "et-ui-tone--success",
        FeedbackTone::Warning => "et-ui-tone--warning",
        FeedbackTone::Danger => "et-ui-tone--danger",
    };
    rsx! { span { class: "et-ui-status-badge {class}", "{label}" } }
}

#[component]
pub fn MetadataList(items: Vec<(String, String)>) -> Element {
    rsx! {
        ul { class: "et-ui-metadata-list",
            for (label, value) in items {
                li { class: "et-ui-metadata-list__item",
                    span { class: "et-ui-metadata-list__label", "{label}" }
                    span { class: "et-ui-metadata-list__value", "{value}" }
                }
            }
        }
    }
}
