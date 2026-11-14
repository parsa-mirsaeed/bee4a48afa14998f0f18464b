# Offline AI Grading Quality Benchmark

This evaluation-only tool never calls a model, reads student work, changes rubrics, or publishes grades. A de-identified teacher-curated reference benchmark must be supplied separately. Do not include student names, identifiers, raw text or generated feedback.

Input JSONL contains exactly case_id, cohort, language, grade_scale, reference_grade, predicted_grade, confidence. Use synthetic case IDs and only generic subject/language labels. A null predicted_grade with null confidence represents abstention. Confidence is 0..1.

Synthetic example:

    {"case_id":"example_001","cohort":"mathematics","language":"fa","grade_scale":20,"reference_grade":18,"predicted_grade":17,"confidence":0.7}

Run tests:

    python3 -m unittest discover -s scripts/ai_grading -p 'test_*.py'

Illustrative limits only, not an approved rollout policy:

    python3 scripts/ai_grading/evaluate.py --input benchmark.jsonl --output report.json --min-samples 50 --min-coverage 0.90 --max-mae-pp 5 --max-severe-rate 0.05 --max-overconfident-rate 0.01

Exit 0: all overall/cohort/language quality thresholds pass; exit 1: quality failure; exit 2: invalid input. Overall, each cohort, each language, and every observed cohort-language intersection must independently satisfy sample count, coverage and error thresholds to prevent masking a weak subgroup. Report schema version 2 adds `cohort_languages` and failure scopes `cohort_language:<cohort>/<language>`. This does not establish whether unobserved intersection groups are represented; dataset approval must ensure appropriate coverage. The harness does NOT certify model correctness, representativeness, pedagogical suitability, consent/privacy, or authorization to auto-publish grades. Teacher feedback review and live-model benchmarks remain required before rollout.

Acceptance comparisons use unrounded numerical metrics, including exact coverage and severe-disagreement fractions; report JSON retains full precision. Repeated JSON object keys are rejected to avoid ambiguous last-value-wins records. Do not round thresholds or metrics before determining pass/fail.

## Release evidence integrity gate (synthetic CI; real approval stays external)

Run the stronger, fail-closed provenance and subgroup gate **only** with a separately approved, de-identified teacher-curated prediction set from the **actual pinned native PDF LLM/prompt**. The generic evaluator alone cannot prove provenance or that both languages were represented.

    python3 scripts/ai_grading/release_gate.py \
      --manifest private-approved-evidence.json \
      --benchmark private-approved-grades.jsonl \
      --output private-quality-summary.json

The manifest uses exactly this schema (hashes shown as placeholders; real values must be lowercase SHA-256 of the exact bytes, prompt and approved rubric policy):

    {
      "schema_version": 1,
      "benchmark_sha256": "<64 lowercase hex characters>",
      "provider": "openai",
      "model": "gpt-4o",
      "prompt_sha256": "<64 lowercase hex characters>",
      "rubric_policy_sha256": "<64 lowercase hex characters>",
      "reference_approval_id": "approved_anonymous_reference_id",
      "required_cohort_languages": ["math_g7/en", "math_g7/fa"],
      "thresholds": {
        "min_samples": 50, "min_coverage": 0.9,
        "max_mae_pp": 5, "max_severe_rate": 0.05,
        "max_overconfident_rate": 0.01,
        "severe_error_pp": 10, "high_confidence": 0.8
      }
    }

A threshold pass produces `technical_evidence_passed: true` but **always** `release_approved: false`. The script cannot authenticate a teacher's signature, independently adjudicate a rubric, verify that predictions were genuinely produced by the pinned production model, or replace privacy/legal and pedagogical signoff. Data and private reports must not be committed to Git or uploaded to public CI. Avoid student identifiers, raw answers, names, comments, filenames, access keys, and teacher names in inputs/manifest. Only synthetic fixtures run in public CI. Additional grades/subjects, scanned/handwritten cohorts, and severity thresholds must be explicitly specified in the approved coverage plan before release.
