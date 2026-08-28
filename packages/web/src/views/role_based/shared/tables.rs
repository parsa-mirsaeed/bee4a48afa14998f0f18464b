use crate::i18n::use_locale;
use crate::ui::{DataState, DataStateKind, DataTable as UiDataTable, Grid, MetricCard};
use dioxus::prelude::*;

#[component]
pub fn DataTable(columns: Vec<TableColumn>, data: Vec<TableRow>, title: Option<String>) -> Element {
    let locale = use_locale();
    if data.is_empty() {
        return rsx! {
            DataState {
                kind: DataStateKind::Empty,
                title: title.unwrap_or_else(|| locale.t("common.no_data")),
                description: locale.t("common.no_data"),
            }
        };
    }

    let headers = columns.iter().map(|column| column.label.clone()).collect();
    let rows = data
        .iter()
        .map(|row| {
            columns
                .iter()
                .map(|column| row.get_cell_value(&column.key))
                .collect::<Vec<_>>()
        })
        .collect();
    let caption = title.unwrap_or_else(|| locale.t("common.data"));

    rsx! { UiDataTable { caption, headers, rows } }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableColumn {
    pub key: String,
    pub label: String,
}

impl TableColumn {
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableRow {
    pub cells: std::collections::HashMap<String, String>,
}

impl TableRow {
    pub fn new() -> Self {
        Self {
            cells: std::collections::HashMap::new(),
        }
    }

    pub fn with_cell(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.cells.insert(key.into(), value.into());
        self
    }

    pub fn get_cell_value(&self, key: &str) -> String {
        self.cells
            .get(key)
            .cloned()
            .unwrap_or_else(|| "-".to_string())
    }
}

impl Default for TableRow {
    fn default() -> Self {
        Self::new()
    }
}

#[component]
pub fn StatsGrid(stats: Vec<StatItem>, columns: Option<i32>) -> Element {
    let columns = columns.unwrap_or(4).clamp(1, 4).try_into().unwrap_or(4_u8);
    rsx! {
        Grid { columns,
            for stat in stats {
                MetricCard {
                    label: stat.label,
                    value: stat.value,
                    supporting: stat.change,
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StatItem {
    pub label: String,
    pub value: String,
    pub color: String,
    pub change: Option<String>,
    pub icon: Option<String>,
}

impl StatItem {
    pub fn new(
        label: impl Into<String>,
        value: impl Into<String>,
        color: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            color: color.into(),
            change: None,
            icon: None,
        }
    }

    pub fn with_change(mut self, change: impl Into<String>) -> Self {
        self.change = Some(change.into());
        self
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}
