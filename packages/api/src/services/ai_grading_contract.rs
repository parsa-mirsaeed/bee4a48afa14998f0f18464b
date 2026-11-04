//! Strict, versioned contract for non-authoritative AI grading suggestions.
//! No suggestion in this module changes a submission grade or is visible to students.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const GRADING_CONTRACT_VERSION: u16 = 1;
pub const MAX_CRITERIA: usize = 20;
pub const MAX_FEEDBACK_CHARS: usize = 4_000;
pub const MAX_EVIDENCE_CHARS: usize = 1_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CriterionScore {
    pub criterion_id: String,
    pub awarded_points: f64,
    pub possible_points: f64,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GradingSuggestion {
    pub contract_version: u16,
    /// Immutable submission revision captured before model invocation.
    pub submission_revision: Uuid,
    pub grade_scale: i16,
    pub suggested_grade: f64,
    pub confidence: f64,
    pub feedback: String,
    pub criteria: Vec<CriterionScore>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuggestionValidationError {
    UnsupportedVersion,
    StaleSubmission,
    InvalidScale,
    InvalidScore,
    InvalidConfidence,
    InvalidCriteria,
    InvalidText,
}

impl GradingSuggestion {
    /// Fail closed before persisting any LLM-produced grading recommendation.
    pub fn validate(&self, current_revision: Uuid) -> Result<(), SuggestionValidationError> {
        use SuggestionValidationError as Error;
        if self.contract_version != GRADING_CONTRACT_VERSION {
            return Err(Error::UnsupportedVersion);
        }
        if self.submission_revision.is_nil() || self.submission_revision != current_revision {
            return Err(Error::StaleSubmission);
        }
        if !matches!(self.grade_scale, 20 | 100) {
            return Err(Error::InvalidScale);
        }
        let scale = f64::from(self.grade_scale);
        if !self.suggested_grade.is_finite()
            || !(0.0..=scale).contains(&self.suggested_grade)
        {
            return Err(Error::InvalidScore);
        }
        if !self.confidence.is_finite() || !(0.0..=1.0).contains(&self.confidence) {
            return Err(Error::InvalidConfidence);
        }
        if self.feedback.trim().is_empty() || self.feedback.chars().count() > MAX_FEEDBACK_CHARS {
            return Err(Error::InvalidText);
        }
        if self.criteria.is_empty() || self.criteria.len() > MAX_CRITERIA {
            return Err(Error::InvalidCriteria);
        }
        let mut ids = std::collections::HashSet::new();
        let mut awarded = 0.0;
        let mut possible = 0.0;
        for criterion in &self.criteria {
            if criterion.criterion_id.trim().is_empty()
                || criterion.criterion_id.len() > 100
                || !ids.insert(criterion.criterion_id.as_str())
                || !criterion.awarded_points.is_finite()
                || !criterion.possible_points.is_finite()
                || criterion.possible_points <= 0.0
                || criterion.awarded_points < 0.0
                || criterion.awarded_points > criterion.possible_points
            {
                return Err(Error::InvalidCriteria);
            }
            if criterion.evidence.trim().is_empty()
                || criterion.evidence.chars().count() > MAX_EVIDENCE_CHARS
            {
                return Err(Error::InvalidText);
            }
            awarded += criterion.awarded_points;
            possible += criterion.possible_points;
        }
        // Criteria are expressed directly on the declared grading scale.
        if !awarded.is_finite()
            || !possible.is_finite()
            || (possible - scale).abs() > 0.000001
            || (awarded - self.suggested_grade).abs() > 0.000001
        {
            return Err(Error::InvalidCriteria);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> GradingSuggestion {
        GradingSuggestion {
            contract_version: GRADING_CONTRACT_VERSION,
            submission_revision: Uuid::new_v4(),
            grade_scale: 20,
            suggested_grade: 18.0,
            confidence: 0.85,
            feedback: "Strong reasoning".into(),
            criteria: vec![
                CriterionScore {
                    criterion_id: "reasoning".into(),
                    awarded_points: 9.0,
                    possible_points: 10.0,
                    evidence: "Step two explains the result".into(),
                },
                CriterionScore {
                    criterion_id: "accuracy".into(),
                    awarded_points: 9.0,
                    possible_points: 10.0,
                    evidence: "Correct final answer".into(),
                },
            ],
        }
    }

    #[test]
    fn accepts_consistent_suggestion() {
        let s = valid();
        assert_eq!(s.validate(s.submission_revision), Ok(()));
    }

    #[test]
    fn rejects_stale_and_nil_revisions() {
        let mut s = valid();
        assert_eq!(s.validate(Uuid::new_v4()), Err(SuggestionValidationError::StaleSubmission));
        s.submission_revision = Uuid::nil();
        assert_eq!(s.validate(Uuid::nil()), Err(SuggestionValidationError::StaleSubmission));
    }

    #[test]
    fn rejects_non_finite_scores_and_confidence() {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut s = valid();
            s.suggested_grade = value;
            assert_eq!(s.validate(s.submission_revision), Err(SuggestionValidationError::InvalidScore));
            let mut s = valid();
            s.confidence = value;
            assert_eq!(s.validate(s.submission_revision), Err(SuggestionValidationError::InvalidConfidence));
            let mut s = valid();
            s.criteria[0].awarded_points = value;
            assert_eq!(s.validate(s.submission_revision), Err(SuggestionValidationError::InvalidCriteria));
        }
    }

    #[test]
    fn rejects_inconsistent_rubric_and_duplicate_criteria() {
        let mut s = valid();
        s.criteria[0].awarded_points = 8.0;
        assert_eq!(s.validate(s.submission_revision), Err(SuggestionValidationError::InvalidCriteria));
        let mut s = valid();
        s.criteria[1].criterion_id = "reasoning".into();
        assert_eq!(s.validate(s.submission_revision), Err(SuggestionValidationError::InvalidCriteria));
    }

    #[test]
    fn rejects_unsupported_versions_scales_and_empty_evidence() {
        let mut s = valid();
        s.contract_version = 2;
        assert_eq!(s.validate(s.submission_revision), Err(SuggestionValidationError::UnsupportedVersion));
        let mut s = valid();
        s.grade_scale = 10;
        assert_eq!(s.validate(s.submission_revision), Err(SuggestionValidationError::InvalidScale));
        let mut s = valid();
        s.criteria[0].evidence.clear();
        assert_eq!(s.validate(s.submission_revision), Err(SuggestionValidationError::InvalidText));
    }

    #[test]
    fn rejects_unexpected_model_fields() {
        let s = valid();
        let mut value = serde_json::to_value(&s).expect("serialize");
        value["publish_grade"] = serde_json::json!(true);
        assert!(serde_json::from_value::<GradingSuggestion>(value).is_err());
    }
}
