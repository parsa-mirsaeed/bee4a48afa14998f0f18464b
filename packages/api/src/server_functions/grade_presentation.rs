//! Shared presentation rules for persisted academic grades.
//!
//! The numeric value stored on a submission is expressed in its declared
//! scale.  It must never be treated as a percentage merely because a caller
//! needs a letter grade.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GradePresentation {
    pub letter_grade: String,
    pub points: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradePresentationError {
    InvalidScale,
    InvalidGrade,
}

impl fmt::Display for GradePresentationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidScale => formatter.write_str("grade scale must be positive"),
            Self::InvalidGrade => formatter.write_str("grade must be finite"),
        }
    }
}

impl std::error::Error for GradePresentationError {}

/// Produces the display value from the persisted grade and its declared scale.
///
/// The source scale is retained in `points`; only the derived letter grade is
/// calculated from a normalized percentage.
pub fn present_grade(
    grade: f64,
    grade_scale: i16,
) -> Result<GradePresentation, GradePresentationError> {
    if grade_scale <= 0 {
        return Err(GradePresentationError::InvalidScale);
    }
    if !grade.is_finite() {
        return Err(GradePresentationError::InvalidGrade);
    }

    let percentage = grade / f64::from(grade_scale) * 100.0;
    Ok(GradePresentation {
        letter_grade: percentage_to_letter_grade(percentage).to_owned(),
        points: format!("{grade:.0}/{grade_scale}"),
    })
}

pub fn percentage_to_letter_grade(percentage: f64) -> &'static str {
    if percentage >= 93.0 {
        "A"
    } else if percentage >= 90.0 {
        "A-"
    } else if percentage >= 87.0 {
        "B+"
    } else if percentage >= 83.0 {
        "B"
    } else if percentage >= 80.0 {
        "B-"
    } else if percentage >= 77.0 {
        "C+"
    } else if percentage >= 73.0 {
        "C"
    } else if percentage >= 70.0 {
        "C-"
    } else if percentage >= 67.0 {
        "D+"
    } else if percentage >= 63.0 {
        "D"
    } else if percentage >= 60.0 {
        "D-"
    } else {
        "F"
    }
}

#[cfg(test)]
mod tests {
    use super::{present_grade, GradePresentationError};

    #[test]
    fn preserves_twenty_point_scale_when_deriving_letter_grade() {
        assert_eq!(
            present_grade(18.0, 20).unwrap(),
            super::GradePresentation {
                letter_grade: "A-".to_owned(),
                points: "18/20".to_owned(),
            }
        );
    }

    #[test]
    fn preserves_other_declared_scales() {
        assert_eq!(present_grade(45.0, 50).unwrap().letter_grade, "A-");
        assert_eq!(present_grade(45.0, 50).unwrap().points, "45/50");
        assert_eq!(present_grade(100.0, 100).unwrap().letter_grade, "A");
        assert_eq!(present_grade(100.0, 100).unwrap().points, "100/100");
    }

    #[test]
    fn legacy_null_scale_policy_is_the_hundred_point_fallback_at_query_boundary() {
        assert_eq!(present_grade(90.0, 100).unwrap().points, "90/100");
    }

    #[test]
    fn invalid_scale_or_grade_is_rejected() {
        assert_eq!(present_grade(18.0, 0), Err(GradePresentationError::InvalidScale));
        assert_eq!(
            present_grade(f64::NAN, 20),
            Err(GradePresentationError::InvalidGrade)
        );
    }
}
