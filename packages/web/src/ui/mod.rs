//! Canonical EduTalent UI primitives.
//!
//! PR-2 establishes this module as the only place for new shared interaction
//! primitives. Feature screens may migrate incrementally in later PRs, but the
//! authenticated shell, authentication and notification surfaces consume these
//! components immediately.

pub mod actions;
pub mod data;
pub mod feedback;
pub mod forms;
pub mod layers;
pub mod navigation;
pub mod structure;

pub use actions::*;
pub use data::*;
pub use feedback::*;
pub use forms::*;
pub use layers::*;
pub use navigation::*;
pub use structure::*;
