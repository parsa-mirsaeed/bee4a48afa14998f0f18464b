# Offline AI Grading Quality Benchmark

This evaluation-only tool never calls a model, reads student work, changes rubrics, or publishes grades. A de-identified teacher-curated reference benchmark must be supplied separately. Do not include student names, identifiers, raw text or generated feedback.

Input JSONL contains exactly case_id, cohort, language, grade_scale, reference_grade, predicted_grade, confidence. Use synthetic case IDs and only generic subject/language labels. A null predicted_grade with null confidence represents abstention. Confidence is 0..1.

Synthetic example:

    {"case_id":"example_001","cohort":"mathematics","language":"fa","grade_scale":20,"reference_grade":18,"predicted_grade":17,"confidence":0.7}

Run tests:

    python3 -m unittest discover -s scripts/ai_grading -p 'test_*.py'

Illustrative limits only, not an approved rollout policy:

    python3 scripts/ai_grading/evaluate.py --input benchmark.jsonl --output report.json --min-samples 50 --min-coverage 0.90 --max-mae-pp 5 --max-severe-rate 0.05 --max-overconfident-rate 0.01

Exit 0: all overall/cohort/language quality thresholds pass; exit 1: quality failure; exit 2: invalid input. Every group must satisfy sample count and coverage to prevent misleading headline accuracy. The harness does NOT certify model correctness, representativeness, pedagogical suitability, consent/privacy, or authorization to auto-publish grades. Teacher feedback review and live-model benchmarks remain required before rollout.

Acceptance comparisons use unrounded numerical metrics, including exact coverage and severe-disagreement fractions; report JSON retains full precision. Repeated JSON object keys are rejected to avoid ambiguous last-value-wins records. Do not round thresholds or metrics before determining pass/fail.
