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
mod parent_translations;
mod platform_admin_translations;
pub mod provider;
mod student_translations;
mod teacher_assignments_translations;
mod teacher_classes_translations;
mod teacher_dashboard_translations;
pub mod translations;
mod ui_translations;

pub use assignment_status::assignment_status_label;
pub use date_presentation::{
    format_product_date, format_product_date_text, format_product_datetime_text,
};
pub use grading::*;
pub use locale::*;
pub(crate) use parent_translations::{format_parent_class_count, parent_translation};
pub(crate) use platform_admin_translations::{
    platform_admin_actor_label, platform_admin_audit_action_label,
    platform_admin_language_label, platform_admin_lifecycle_guidance,
    platform_admin_status_label, platform_admin_target_type_label,
    platform_admin_translation,
};
pub use provider::*;
pub(crate) use student_translations::student_translation;
pub(crate) use teacher_assignments_translations::teacher_assignments_translation;
pub(crate) use teacher_classes_translations::{
    format_teacher_vectorization_duration, teacher_classes_translation,
};
pub(crate) use teacher_dashboard_translations::teacher_dashboard_translation;
pub use translations::*;
pub use ui_translations::role_label;
pub(crate) use ui_translations::supplemental_translation;
