//! Internationalization (i18n) Module for EduTalent
//!
//! Provides bilingual support for English and Farsi (Persian) with:
//! - Farsi as the default language
//! - RTL layout support
//! - Locale-specific grading (20-based for Farsi, 100-based for English)
//! - Type-safe translation keys

mod assignment_status;
mod date_presentation;
pub mod grading;
pub mod locale;
pub mod provider;
pub mod translations;
mod ui_translations;

pub use assignment_status::assignment_status_label;
pub use date_presentation::format_product_date;
pub use grading::*;
pub use locale::*;
pub use provider::*;
pub use translations::*;
pub use ui_translations::role_label;
pub(crate) use ui_translations::supplemental_translation;
