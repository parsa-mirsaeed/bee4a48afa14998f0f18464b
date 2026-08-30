use crate::ui::{DataState, DataStateKind};
use dioxus::prelude::*;

/// Non-interactive production state for a capability intentionally excluded
/// from the current release. No control is rendered that could imply support.
#[component]
pub fn UnavailableFeature(title: String, description: String) -> Element {
    rsx! {
        DataState {
            kind: DataStateKind::Unavailable,
            title,
            description,
        }
    }
}
