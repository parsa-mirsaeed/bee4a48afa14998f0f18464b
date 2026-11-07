//! Strict, versioned contract for non-authoritative AI grading suggestions.
//! No suggestion in this module changes a submission grade or is visible to students.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const GRADING_CONTRACT_VERSION: u16 = 1;
pub const MAX_CRITERIA: usize = 20;
pub const MAX_FEEDBACK_CHARS: usize = 4_000;
pub const MAX_EVIDENCE_CHARS: usize = 1_000;

// Unicode bidirectional formatting controls can visually reorder a teacher's
// rubric identifier or model feedback without being Rust control characters.
// Default-ignorable characters can make distinct rubric keys look identical.
// Reject these in identifiers; ordinary Persian/Arabic letters remain valid.
fn invisible_identifier_format(c: char) -> bool {
    matches!(
        c,
        '\u{00ad}'
            | '\u{034f}'
            | '\u{180e}'
            | '\u{200b}'..='\u{200d}'
            | '\u{2060}'
            | '\u{fe0e}'..='\u{fe0f}'
            | '\u{feff}'
    )
}

fn unsafe_bidi_format(c: char) -> bool {
    matches!(
        c,
        '\u{061c}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2066}'..='\u{2069}'
    )
}

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
        if !self.suggested_grade.is_finite() || !(0.0..=scale).contains(&self.suggested_grade) {
            return Err(Error::InvalidScore);
        }
        if !self.confidence.is_finite() || !(0.0..=1.0).contains(&self.confidence) {
            return Err(Error::InvalidConfidence);
        }
        if self.feedback.trim().is_empty()
            || self.feedback.chars().count() > MAX_FEEDBACK_CHARS
            || self
                .feedback
                .chars()
                .any(|c| (c.is_control() && !matches!(c, '\n' | '\t')) || unsafe_bidi_format(c))
        {
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
                || criterion.criterion_id.trim() != criterion.criterion_id
                || criterion.criterion_id.len() > 100
                || criterion.criterion_id.chars().any(|c| {
                    c.is_control() || unsafe_bidi_format(c) || invisible_identifier_format(c)
                })
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
                || criterion
                    .evidence
                    .chars()
                    .any(|c| (c.is_control() && !matches!(c, '\n' | '\t')) || unsafe_bidi_format(c))
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
        assert_eq!(
            s.validate(Uuid::new_v4()),
            Err(SuggestionValidationError::StaleSubmission)
        );
        s.submission_revision = Uuid::nil();
        assert_eq!(
            s.validate(Uuid::nil()),
            Err(SuggestionValidationError::StaleSubmission)
        );
    }

    #[test]
    fn rejects_non_finite_scores_and_confidence() {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut s = valid();
            s.suggested_grade = value;
            assert_eq!(
                s.validate(s.submission_revision),
                Err(SuggestionValidationError::InvalidScore)
            );
            let mut s = valid();
            s.confidence = value;
            assert_eq!(
                s.validate(s.submission_revision),
                Err(SuggestionValidationError::InvalidConfidence)
            );
            let mut s = valid();
            s.criteria[0].awarded_points = value;
            assert_eq!(
                s.validate(s.submission_revision),
                Err(SuggestionValidationError::InvalidCriteria)
            );
        }
    }

    #[test]
    fn rejects_inconsistent_rubric_and_duplicate_criteria() {
        let mut s = valid();
        s.criteria[0].awarded_points = 8.0;
        assert_eq!(
            s.validate(s.submission_revision),
            Err(SuggestionValidationError::InvalidCriteria)
        );
        let mut s = valid();
        s.criteria[1].criterion_id = "reasoning".into();
        assert_eq!(
            s.validate(s.submission_revision),
            Err(SuggestionValidationError::InvalidCriteria)
        );
    }

    #[test]
    fn rejects_unsupported_versions_scales_and_empty_evidence() {
        let mut s = valid();
        s.contract_version = 2;
        assert_eq!(
            s.validate(s.submission_revision),
            Err(SuggestionValidationError::UnsupportedVersion)
        );
        let mut s = valid();
        s.grade_scale = 10;
        assert_eq!(
            s.validate(s.submission_revision),
            Err(SuggestionValidationError::InvalidScale)
        );
        let mut s = valid();
        s.criteria[0].evidence.clear();
        assert_eq!(
            s.validate(s.submission_revision),
            Err(SuggestionValidationError::InvalidText)
        );
    }

    #[test]
    fn rejects_control_characters_in_teacher_facing_text() {
        for control in ['\u{0000}', '\u{001b}', '\u{007f}'] {
            let mut s = valid();
            s.feedback.push(control);
            assert_eq!(
                s.validate(s.submission_revision),
                Err(SuggestionValidationError::InvalidText)
            );
            let mut s = valid();
            s.criteria[0].evidence.push(control);
            assert_eq!(
                s.validate(s.submission_revision),
                Err(SuggestionValidationError::InvalidText)
            );
        }
        let mut s = valid();
        s.feedback.push('\n');
        s.criteria[0].evidence.push('\t');
        assert_eq!(s.validate(s.submission_revision), Ok(()));
    }

    #[test]
    fn rejects_empty_criterion_identifiers() {
        let mut s = valid();
        s.criteria[0].criterion_id.clear();
        assert_eq!(
            s.validate(s.submission_revision),
            Err(SuggestionValidationError::InvalidCriteria)
        );
    }

    #[test]
    fn rejects_whitespace_padded_criterion_identifiers() {
        for id in [" reasoning", "reasoning ", "\treasoning", "reasoning\n"] {
            let mut s = valid();
            s.criteria[0].criterion_id = id.into();
            assert_eq!(
                s.validate(s.submission_revision),
                Err(SuggestionValidationError::InvalidCriteria)
            );
        }
    }

    #[test]
    fn rejects_invisible_criterion_identifiers() {
        for mark in ['\u{200b}', '\u{200c}', '\u{200d}', '\u{2060}', '\u{feff}'] {
            let mut s = valid();
            s.criteria[0].criterion_id.push(mark);
            assert_eq!(
                s.validate(s.submission_revision),
                Err(SuggestionValidationError::InvalidCriteria)
            );
        }
    }

    #[test]
    fn rejects_control_characters_in_criterion_identifiers() {
        let mut s = valid();
        s.criteria[0].criterion_id = "reasoning\u{001b}[31m".into();
        assert_eq!(
            s.validate(s.submission_revision),
            Err(SuggestionValidationError::InvalidCriteria)
        );
    }

    #[test]
    fn rejects_bidi_spoofing_in_model_text_and_identifiers() {
        for mark in ['\u{202e}', '\u{2066}', '\u{200f}', '\u{061c}'] {
            let mut s = valid();
            s.feedback.push(mark);
            assert_eq!(
                s.validate(s.submission_revision),
                Err(SuggestionValidationError::InvalidText)
            );
            let mut s = valid();
            s.criteria[0].evidence.push(mark);
            assert_eq!(
                s.validate(s.submission_revision),
                Err(SuggestionValidationError::InvalidText)
            );
            let mut s = valid();
            s.criteria[0].criterion_id.push(mark);
            assert_eq!(
                s.validate(s.submission_revision),
                Err(SuggestionValidationError::InvalidCriteria)
            );
        }
    }

    #[test]
    fn rejects_unexpected_model_fields() {
        let s = valid();
        let mut value = serde_json::to_value(&s).expect("serialize");
        value["publish_grade"] = serde_json::json!(true);
        assert!(serde_json::from_value::<GradingSuggestion>(value).is_err());
    }
}

/// Teacher-authored criterion identity and weight; never inferred from model output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExpectedCriterion {
    pub criterion_id: String,
    pub possible_points: f64,
}

/// Validate teacher-supplied rubric data before any student text leaves the
/// authorized application boundary. Names remain Unicode (including Persian)
/// while default-ignorable, bidi-spoofing, padded and duplicate IDs are denied.
pub fn validate_authoritative_rubric(
    rubric: &[ExpectedCriterion],
    grade_scale: i16,
) -> Result<(), SuggestionValidationError> {
    use SuggestionValidationError as Error;
    if !matches!(grade_scale, 20 | 100) || rubric.is_empty() || rubric.len() > MAX_CRITERIA {
        return Err(Error::InvalidCriteria);
    }
    let mut ids = std::collections::HashSet::new();
    let mut sum = 0.0;
    for criterion in rubric {
        if criterion.criterion_id.is_empty()
            || criterion.criterion_id.trim() != criterion.criterion_id
            || criterion.criterion_id.len() > 100
            || criterion
                .criterion_id
                .chars()
                .any(|c| c.is_control() || unsafe_bidi_format(c) || invisible_identifier_format(c))
            || !ids.insert(criterion.criterion_id.as_str())
            || !criterion.possible_points.is_finite()
            || criterion.possible_points <= 0.0
        {
            return Err(Error::InvalidCriteria);
        }
        sum += criterion.possible_points;
    }
    if !sum.is_finite() || (sum - f64::from(grade_scale)).abs() > 0.000001 {
        return Err(Error::InvalidCriteria);
    }
    Ok(())
}

impl GradingSuggestion {
    /// Validate model output against the *authoritative* teacher rubric, not
    /// against the model's self-reported possible points.
    pub fn validate_against_rubric(
        &self,
        current_revision: Uuid,
        expected: &[ExpectedCriterion],
    ) -> Result<(), SuggestionValidationError> {
        use SuggestionValidationError as Error;
        self.validate(current_revision)?;
        validate_authoritative_rubric(expected, self.grade_scale)?;
        if expected.len() != self.criteria.len() {
            return Err(Error::InvalidCriteria);
        }
        let mut seen = std::collections::HashSet::new();
        let mut total = 0.0;
        for item in expected {
            if item.criterion_id.trim().is_empty()
                || item.criterion_id.trim() != item.criterion_id
                || item.criterion_id.len() > 100
                || item.criterion_id.chars().any(|c| {
                    c.is_control() || unsafe_bidi_format(c) || invisible_identifier_format(c)
                })
                || !seen.insert(item.criterion_id.as_str())
                || !item.possible_points.is_finite()
                || item.possible_points <= 0.0
            {
                return Err(Error::InvalidCriteria);
            }
            let scored = self
                .criteria
                .iter()
                .find(|scored| scored.criterion_id == item.criterion_id)
                .ok_or(Error::InvalidCriteria)?;
            if (scored.possible_points - item.possible_points).abs() > 0.000001 {
                return Err(Error::InvalidCriteria);
            }
            total += item.possible_points;
        }
        if !total.is_finite() || (total - f64::from(self.grade_scale)).abs() > 0.000001 {
            return Err(Error::InvalidCriteria);
        }
        Ok(())
    }
}

#[cfg(test)]
mod rubric_binding_tests {
    use super::*;

    fn suggestion() -> GradingSuggestion {
        GradingSuggestion {
            contract_version: GRADING_CONTRACT_VERSION,
            submission_revision: Uuid::new_v4(),
            grade_scale: 20,
            suggested_grade: 18.0,
            confidence: 0.9,
            feedback: "Evidence-based feedback".into(),
            criteria: vec![
                CriterionScore {
                    criterion_id: "logic".into(),
                    awarded_points: 9.0,
                    possible_points: 10.0,
                    evidence: "Logical steps".into(),
                },
                CriterionScore {
                    criterion_id: "answer".into(),
                    awarded_points: 9.0,
                    possible_points: 10.0,
                    evidence: "Correct result".into(),
                },
            ],
        }
    }

    fn rubric() -> Vec<ExpectedCriterion> {
        vec![
            ExpectedCriterion {
                criterion_id: "logic".into(),
                possible_points: 10.0,
            },
            ExpectedCriterion {
                criterion_id: "answer".into(),
                possible_points: 10.0,
            },
        ]
    }

    #[test]
    fn accepts_exact_teacher_rubric() {
        let s = suggestion();
        assert_eq!(
            s.validate_against_rubric(s.submission_revision, &rubric()),
            Ok(())
        );
    }

    #[test]
    fn rejects_model_reweighted_rubric_even_when_total_matches() {
        let mut s = suggestion();
        s.criteria[0].possible_points = 11.0;
        s.criteria[1].possible_points = 9.0;
        assert_eq!(s.validate(s.submission_revision), Ok(()));
        assert_eq!(
            s.validate_against_rubric(s.submission_revision, &rubric()),
            Err(SuggestionValidationError::InvalidCriteria)
        );
    }

    #[test]
    fn rejects_missing_duplicate_and_unexpected_criteria() {
        let s = suggestion();
        let mut expected = rubric();
        expected[1].criterion_id = "unknown".into();
        assert_eq!(
            s.validate_against_rubric(s.submission_revision, &expected),
            Err(SuggestionValidationError::InvalidCriteria)
        );
        let mut expected = rubric();
        expected[1].criterion_id = "logic".into();
        assert_eq!(
            s.validate_against_rubric(s.submission_revision, &expected),
            Err(SuggestionValidationError::InvalidCriteria)
        );
        assert_eq!(
            s.validate_against_rubric(s.submission_revision, &rubric()[..1]),
            Err(SuggestionValidationError::InvalidCriteria)
        );
    }

    #[test]
    fn rejects_empty_authoritative_rubric_ids() {
        let s = suggestion();
        let mut expected = rubric();
        expected[0].criterion_id.clear();
        assert_eq!(
            s.validate_against_rubric(s.submission_revision, &expected),
            Err(SuggestionValidationError::InvalidCriteria)
        );
    }

    #[test]
    fn rejects_whitespace_padded_authoritative_rubric_ids() {
        let s = suggestion();
        for id in [" logic", "logic ", "\tlogic"] {
            let mut expected = rubric();
            expected[0].criterion_id = id.into();
            assert_eq!(
                s.validate_against_rubric(s.submission_revision, &expected),
                Err(SuggestionValidationError::InvalidCriteria)
            );
        }
    }

    #[test]
    fn rejects_invalid_authoritative_criterion_identifiers() {
        let s = suggestion();
        for id in ["logic\u{001b}[31m".to_string(), "x".repeat(101)] {
            let mut expected = rubric();
            expected[0].criterion_id = id;
            assert_eq!(
                s.validate_against_rubric(s.submission_revision, &expected),
                Err(SuggestionValidationError::InvalidCriteria)
            );
        }
    }

    #[test]
    fn rejects_invisible_authoritative_rubric_ids() {
        let s = suggestion();
        for mark in ['\u{200b}', '\u{200c}', '\u{200d}', '\u{2060}', '\u{feff}'] {
            let mut expected = rubric();
            expected[0].criterion_id.push(mark);
            assert_eq!(
                s.validate_against_rubric(s.submission_revision, &expected),
                Err(SuggestionValidationError::InvalidCriteria)
            );
        }
    }

    #[test]
    fn rejects_bidi_spoofing_in_authoritative_rubric_ids() {
        let s = suggestion();
        let mut expected = rubric();
        expected[0].criterion_id.push('\u{202e}');
        assert_eq!(
            s.validate_against_rubric(s.submission_revision, &expected),
            Err(SuggestionValidationError::InvalidCriteria)
        );
    }

    #[test]
    fn rejects_invalid_authoritative_weights() {
        let s = suggestion();
        for weight in [f64::NAN, f64::INFINITY, -1.0, 0.0] {
            let mut expected = rubric();
            expected[0].possible_points = weight;
            assert_eq!(
                s.validate_against_rubric(s.submission_revision, &expected),
                Err(SuggestionValidationError::InvalidCriteria)
            );
        }
    }
}

/// Maximum accepted model response size, including JSON syntax.
pub const MAX_GRADING_RESPONSE_BYTES: usize = 32_768;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuggestionParseError {
    Empty,
    TooLarge,
    InvalidJson,
    InvalidSuggestion(SuggestionValidationError),
}

/// Parse a raw gateway response as one strict JSON object. No Markdown fences,
/// trailing prose, partial JSON extraction, or fallback grade are permitted.
/// Call only after the caller has independently authorized the school/teacher
/// and loaded the immutable revision and authoritative rubric from storage.
pub fn parse_grading_suggestion(
    raw: &str,
    current_revision: Uuid,
    rubric: &[ExpectedCriterion],
) -> Result<GradingSuggestion, SuggestionParseError> {
    if raw.trim().is_empty() {
        return Err(SuggestionParseError::Empty);
    }
    if raw.len() > MAX_GRADING_RESPONSE_BYTES {
        return Err(SuggestionParseError::TooLarge);
    }
    // Deserialize directly: an intermediate Value would silently collapse
    // duplicate JSON keys before the strict struct decoder could reject them.
    let suggestion: GradingSuggestion =
        serde_json::from_str(raw).map_err(|_| SuggestionParseError::InvalidJson)?;
    suggestion
        .validate_against_rubric(current_revision, rubric)
        .map_err(SuggestionParseError::InvalidSuggestion)?;
    Ok(suggestion)
}

#[cfg(test)]
mod untrusted_response_tests {
    use super::*;

    fn valid_response() -> (String, Uuid, Vec<ExpectedCriterion>) {
        let revision = Uuid::new_v4();
        let rubric = vec![ExpectedCriterion {
            criterion_id: "accuracy".into(),
            possible_points: 20.0,
        }];
        let suggestion = GradingSuggestion {
            contract_version: GRADING_CONTRACT_VERSION,
            submission_revision: revision,
            grade_scale: 20,
            suggested_grade: 18.0,
            confidence: 0.8,
            feedback: "Clear answer".into(),
            criteria: vec![CriterionScore {
                criterion_id: "accuracy".into(),
                awarded_points: 18.0,
                possible_points: 20.0,
                evidence: "Correct result".into(),
            }],
        };
        (
            serde_json::to_string(&suggestion).unwrap(),
            revision,
            rubric,
        )
    }

    #[test]
    fn parses_valid_authoritative_suggestion() {
        let (raw, revision, rubric) = valid_response();
        let parsed = parse_grading_suggestion(&raw, revision, &rubric).unwrap();
        assert_eq!(parsed.suggested_grade, 18.0);
    }

    #[test]
    fn rejects_fenced_prose_and_non_objects() {
        let (raw, revision, rubric) = valid_response();
        for response in [
            format!("\x60\x60\x60json\\n{raw}\\n\x60\x60\x60"),
            format!("{raw}\\nHere is your grade"),
            "[]".to_string(),
            "null".to_string(),
            String::new(),
        ] {
            assert!(parse_grading_suggestion(&response, revision, &rubric).is_err());
        }
    }

    #[test]
    fn rejects_oversize_responses_before_parsing() {
        let (_, revision, rubric) = valid_response();
        let raw = "x".repeat(MAX_GRADING_RESPONSE_BYTES + 1);
        assert_eq!(
            parse_grading_suggestion(&raw, revision, &rubric),
            Err(SuggestionParseError::TooLarge)
        );
    }

    #[test]
    fn rejects_stale_and_model_modified_rubrics() {
        let (raw, revision, rubric) = valid_response();
        assert_eq!(
            parse_grading_suggestion(&raw, Uuid::new_v4(), &rubric),
            Err(SuggestionParseError::InvalidSuggestion(
                SuggestionValidationError::StaleSubmission
            ))
        );
        let mut altered = rubric;
        altered[0].criterion_id = "invented".into();
        assert_eq!(
            parse_grading_suggestion(&raw, revision, &altered),
            Err(SuggestionParseError::InvalidSuggestion(
                SuggestionValidationError::InvalidCriteria
            ))
        );
    }

    #[test]
    fn rejects_duplicate_json_fields() {
        let (raw, revision, rubric) = valid_response();
        let duplicated = raw.replacen(
            "\"contract_version\":1,",
            "\"contract_version\":1,\"contract_version\":1,",
            1,
        );
        assert_ne!(duplicated, raw);
        assert_eq!(
            parse_grading_suggestion(&duplicated, revision, &rubric),
            Err(SuggestionParseError::InvalidJson)
        );
    }

    #[test]
    fn rejects_extra_fields_in_untrusted_json() {
        let (raw, revision, rubric) = valid_response();
        let mut value: serde_json::Value = serde_json::from_str(&raw).unwrap();
        value["automatically_publish"] = serde_json::json!(true);
        assert_eq!(
            parse_grading_suggestion(&value.to_string(), revision, &rubric),
            Err(SuggestionParseError::InvalidJson)
        );
    }
}
