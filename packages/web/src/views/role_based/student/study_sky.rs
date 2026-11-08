//! Study Sky: a presentation-only view of persisted assignment states.
//!
//! Every assignment returned by `get_student_assignments` becomes one star and
//! submitting work lights it. The module reads no clock and no entropy source,
//! so the same assignments always produce the same sky, and it grants nothing:
//! there is no score, level or reward, only counts of rows the server returned.

use crate::i18n::{assignment_status_label, format_product_date_text, use_locale};
use api::server_functions::dashboard_functions::{
    StudentAssignmentInfo, StudentAssignmentPresentationState as State,
};
use dioxus::prelude::*;

const STUDY_SKY_CSS: Asset = asset!("/assets/study-sky.css");

/// Upper bound on drawn stars; keeps the scene legible and the DOM small.
pub const SKY_STAR_CAP: usize = 18;

/// Vertical positions (percent of scene height) cycled along the star path.
const WAVE: [u32; 6] = [38, 64, 30, 58, 44, 68];

/// A point inside the scene, in whole percent of its width and height.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkyPoint {
    pub x: u32,
    pub y: u32,
}

/// One drawn star: its slot along the path and the state it reflects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkyStar {
    pub slot: usize,
    pub x: u32,
    pub y: u32,
    pub state: State,
}

/// Aggregate of every assignment the server returned, not only drawn stars.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SkyCounts {
    pub pending: usize,
    pub overdue: usize,
    pub submitted: usize,
    pub graded: usize,
}

impl SkyCounts {
    pub fn total(&self) -> usize {
        self.pending + self.overdue + self.submitted + self.graded
    }

    /// Stars that are lit: work that has been submitted or graded.
    pub fn lit(&self) -> usize {
        self.submitted + self.graded
    }

    pub fn percent_lit(&self) -> usize {
        let total = self.total();
        if total == 0 {
            0
        } else {
            self.lit().saturating_mul(100) / total
        }
    }
}

pub fn is_lit(state: State) -> bool {
    match state {
        State::Submitted | State::Graded => true,
        State::Pending | State::Overdue => false,
    }
}

pub fn state_modifier(state: State) -> &'static str {
    match state {
        State::Pending => "pending",
        State::Overdue => "overdue",
        State::Submitted => "submitted",
        State::Graded => "graded",
    }
}

pub fn count_states(items: &[StudentAssignmentInfo]) -> SkyCounts {
    let mut counts = SkyCounts::default();
    for item in items {
        match item.presentation_state {
            State::Pending => counts.pending += 1,
            State::Overdue => counts.overdue += 1,
            State::Submitted => counts.submitted += 1,
            State::Graded => counts.graded += 1,
        }
    }
    counts
}

/// Work that still needs the student: overdue first, then pending, each in the
/// server's due-date order. "Up next" is the first entry; the overview list and
/// the to-do count both come from this one queue so they cannot disagree.
pub fn action_queue(items: &[StudentAssignmentInfo]) -> Vec<&StudentAssignmentInfo> {
    let mut queue = Vec::new();
    for item in items {
        if item.presentation_state == State::Overdue {
            queue.push(item);
        }
    }
    for item in items {
        if item.presentation_state == State::Pending {
            queue.push(item);
        }
    }
    queue
}

/// Indices of the assignments that get a star, in server order.
///
/// Up to the cap every assignment is drawn. Beyond it, work that still needs
/// the student is kept first so a full sky can never hide a to-do item, and the
/// remaining slots go to the latest finished work.
pub fn visible_indices(items: &[StudentAssignmentInfo]) -> Vec<usize> {
    if items.len() <= SKY_STAR_CAP {
        return (0..items.len()).collect();
    }
    let mut actionable = Vec::new();
    let mut finished = Vec::new();
    for (index, item) in items.iter().enumerate() {
        if is_lit(item.presentation_state) {
            finished.push(index);
        } else {
            actionable.push(index);
        }
    }
    actionable.truncate(SKY_STAR_CAP);
    let room = SKY_STAR_CAP - actionable.len();
    let skip = finished.len().saturating_sub(room);
    actionable.extend(finished.into_iter().skip(skip));
    actionable.sort_unstable();
    actionable
}

/// Fixed position for `index` of `count` drawn stars: spread evenly along the
/// inline axis, with a repeating vertical wave. Pure integer arithmetic.
pub fn star_position(index: usize, count: usize) -> SkyPoint {
    let x = if count <= 1 {
        50
    } else {
        8 + (84 * index as u32) / (count as u32 - 1)
    };
    let y = WAVE[index % WAVE.len()];
    SkyPoint { x, y }
}

pub fn layout_stars(items: &[StudentAssignmentInfo]) -> Vec<SkyStar> {
    let visible = visible_indices(items);
    let count = visible.len();
    let mut stars = Vec::with_capacity(count);
    for (slot, index) in visible.into_iter().enumerate() {
        let point = star_position(slot, count);
        stars.push(SkyStar {
            slot,
            x: point.x,
            y: point.y,
            state: items[index].presentation_state,
        });
    }
    stars
}

/// SVG path data (viewBox 0 0 100 100) joining the stars in order.
pub fn constellation_path(stars: &[SkyStar]) -> String {
    let mut path = String::new();
    for (position, star) in stars.iter().enumerate() {
        let command = if position == 0 { 'M' } else { 'L' };
        let x = star.x;
        let y = star.y;
        path.push_str(&format!("{command}{x} {y} "));
    }
    path.trim_end().to_string()
}

/// What the hero shows while the assignment query is in each state.
#[derive(Clone, PartialEq)]
pub enum SkyData {
    Loading,
    Failed,
    Ready(Vec<StudentAssignmentInfo>),
}

#[component]
pub fn StudySkyHero(data: SkyData, on_navigate: EventHandler<String>) -> Element {
    let locale = use_locale();
    let current = locale.current();
    let is_ready = matches!(data, SkyData::Ready(_));
    let items: &[StudentAssignmentInfo] = match &data {
        SkyData::Ready(items) => items,
        _ => &[],
    };
    let counts = count_states(items);
    let stars = layout_stars(items);
    let constellation = constellation_path(&stars);
    let lit = counts.lit();
    let total = counts.total();
    let percent = counts.percent_lit();
    let hidden = total.saturating_sub(stars.len());
    let queue = action_queue(items);
    let next = queue.first().copied();
    let is_empty = is_ready && total == 0;
    let hero_class = if stars.is_empty() {
        "ss-hero ss-hero--plain"
    } else {
        "ss-hero"
    };
    let label = |state: State| assignment_status_label(state.display_name(), current);
    let legend = [
        ("pending", label(State::Pending), counts.pending),
        ("overdue", label(State::Overdue), counts.overdue),
        ("submitted", label(State::Submitted), counts.submitted),
        ("graded", label(State::Graded), counts.graded),
    ];

    rsx! {
        document::Stylesheet { href: STUDY_SKY_CSS }
        section { class: "{hero_class}", "aria-labelledby": "ss-hero-title",
            div { class: "ss-hero__copy",
                p { class: "ss-hero__eyebrow",
                    span { class: "material-icons-outlined", "aria-hidden": "true", "auto_awesome" }
                    "{locale.t(\"student.sky.eyebrow\")}"
                }
                h2 { id: "ss-hero-title", class: "ss-hero__title", "{locale.t(\"dashboard.overview\")}" }
                p { class: "ss-hero__intro", "{locale.t(\"student.dashboard.intro\")}" }

                if is_empty {
                    p { class: "ss-hero__empty", "{locale.t(\"student.sky.empty\")}" }
                }

                if total > 0 {
                    div { class: "ss-hero__stat",
                        span { class: "ss-hero__stat-value", "{lit} / {total}" }
                        span { class: "ss-hero__stat-label", "{locale.t(\"student.sky.lit_caption\")}" }
                    }
                    div {
                        class: "ss-track",
                        role: "progressbar",
                        "aria-label": locale.t("play.student.progress"),
                        "aria-valuemin": "0",
                        "aria-valuemax": "100",
                        "aria-valuenow": "{percent}",
                        div { class: "ss-track__fill", style: "--ss-progress: {percent};" }
                    }
                    ul { class: "ss-legend", "aria-label": locale.t("student.sky.legend_label"),
                        for (modifier, text, count) in legend {
                            li { key: "{modifier}", class: "ss-legend__item",
                                span { class: "ss-swatch ss-swatch--{modifier}", "aria-hidden": "true" }
                                span { class: "ss-legend__label", "{text}" }
                                strong { class: "ss-legend__count", "{count}" }
                            }
                        }
                    }
                    if hidden > 0 {
                        p { class: "ss-hero__more", "+{hidden} {locale.t(\"student.sky.more_hidden\")}" }
                    }
                    if let Some(assignment) = next {
                        {
                            let modifier = state_modifier(assignment.presentation_state);
                            let status = assignment_status_label(
                                assignment.presentation_state.display_name(),
                                current,
                            );
                            let due_date = format_product_date_text(&assignment.due_date, current);
                            rsx! {
                                div { class: "ss-next ss-next--{modifier}",
                                    p { class: "ss-next__label",
                                        "{locale.t(\"student.sky.up_next\")}"
                                        span { class: "ss-next__status", "{status}" }
                                    }
                                    h3 { class: "ss-next__title", "{assignment.title}" }
                                    p { class: "ss-next__meta",
                                        "{assignment.class_name} · {locale.t(\"student.assignments.due_label\")} {due_date}"
                                    }
                                }
                            }
                        }
                    }
                    button {
                        class: "ss-hero__cta",
                        r#type: "button",
                        onclick: move |_| on_navigate.call("assignments".to_string()),
                        "{locale.t(\"play.student.open_tasks\")}"
                        span { class: "material-icons-outlined", "aria-hidden": "true", "arrow_forward" }
                    }
                }
            }

            if !stars.is_empty() {
                div { class: "ss-scene", "aria-hidden": "true",
                    div { class: "ss-world",
                        div { class: "ss-layer ss-layer--far" }
                        div { class: "ss-layer ss-layer--mid",
                            svg {
                                class: "ss-lines",
                                "viewBox": "0 0 100 100",
                                "preserveAspectRatio": "none",
                                path { class: "ss-lines__path", "d": "{constellation}" }
                            }
                        }
                        div { class: "ss-layer ss-layer--near",
                            for star in stars.iter() {
                                span {
                                    key: "{star.slot}",
                                    class: "ss-star ss-star--{state_modifier(star.state)}",
                                    style: "--ss-x: {star.x}; --ss-y: {star.y}; --ss-i: {star.slot};",
                                    span { class: "ss-star__shape" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, state: State) -> StudentAssignmentInfo {
        StudentAssignmentInfo {
            id: id.to_string(),
            title: format!("Title {id}"),
            class_name: "Class".to_string(),
            due_date: "Mar 05, 2026".to_string(),
            presentation_state: state,
            grade: None,
            points: None,
        }
    }

    fn items(states: &[State]) -> Vec<StudentAssignmentInfo> {
        let mut list = Vec::new();
        for (index, state) in states.iter().enumerate() {
            list.push(item(&format!("a{index}"), *state));
        }
        list
    }

    #[test]
    fn layout_is_deterministic_and_capped() {
        let many = items(&[State::Pending; 40]);
        let first = layout_stars(&many);
        let second = layout_stars(&many);
        assert_eq!(first, second);
        assert_eq!(first.len(), SKY_STAR_CAP);
    }

    #[test]
    fn positions_stay_inside_the_sky_and_advance_along_x() {
        for count in 1..=SKY_STAR_CAP {
            let mut previous_x = None;
            for slot in 0..count {
                let point = star_position(slot, count);
                assert!(point.x <= 100 && point.y <= 100);
                if let Some(previous) = previous_x {
                    assert!(point.x > previous);
                }
                previous_x = Some(point.x);
            }
        }
    }

    #[test]
    fn a_capped_sky_never_hides_work_that_needs_the_student() {
        let mut states = vec![State::Graded; 30];
        states.extend([State::Overdue, State::Pending]);
        let list = items(&states);
        let visible = visible_indices(&list);
        assert_eq!(visible.len(), SKY_STAR_CAP);
        assert!(visible.contains(&30));
        assert!(visible.contains(&31));
        assert!(visible.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn up_next_is_the_first_overdue_item_else_the_first_pending_item() {
        let states = [
            State::Graded,
            State::Pending,
            State::Overdue,
            State::Submitted,
            State::Pending,
        ];
        let list = items(&states);
        let queue = action_queue(&list);
        let ids: Vec<&str> = queue.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(ids, ["a2", "a1", "a4"]);

        let only_pending = items(&[State::Submitted, State::Pending, State::Pending]);
        let queue = action_queue(&only_pending);
        assert_eq!(queue.first().map(|item| item.id.as_str()), Some("a1"));
    }

    #[test]
    fn the_to_do_count_and_the_queue_always_agree() {
        let states = [
            State::Graded,
            State::Pending,
            State::Overdue,
            State::Submitted,
            State::Pending,
        ];
        let list = items(&states);
        let counts = count_states(&list);
        assert_eq!(counts.total(), list.len());
        let to_do = counts.pending + counts.overdue;
        assert_eq!(to_do, action_queue(&list).len());
        assert_eq!(counts.lit(), 2);
        assert_eq!(counts.percent_lit(), 40);
    }

    #[test]
    fn the_constellation_is_one_path_through_every_star() {
        let states = [State::Pending, State::Graded, State::Submitted];
        let list = items(&states);
        let stars = layout_stars(&list);
        let path = constellation_path(&stars);
        assert!(path.starts_with('M'));
        assert_eq!(path.matches('L').count(), 2);
    }

    #[test]
    fn no_assignments_means_no_stars_and_no_progress() {
        assert!(layout_stars(&[]).is_empty());
        assert_eq!(count_states(&[]).percent_lit(), 0);
        assert!(action_queue(&[]).is_empty());
    }

    #[test]
    fn production_source_has_no_clock_or_entropy_source() {
        let source = include_str!("study_sky.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("study sky implementation before tests");
        for forbidden in [
            "Utc::now",
            "SystemTime",
            "Instant::now",
            "js_sys",
            "Math.random",
            "rand::",
            "getrandom",
            "Uuid::new_v4",
        ] {
            assert!(!production.contains(forbidden), "found {forbidden}");
        }
    }
}
