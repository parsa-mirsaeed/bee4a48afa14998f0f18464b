-- The API has always persisted and returned a grading scale for submissions.
-- Make that contract explicit for every migrated environment.
ALTER TABLE submissions
    ADD COLUMN IF NOT EXISTS grade_scale SMALLINT;

-- Historical persisted grades were percentage values before the scale column
-- existed. Preserve that meaning; only ungraded rows may use the new default.
UPDATE submissions
SET grade_scale = CASE
    WHEN grade IS NULL THEN 20
    ELSE 100
END
WHERE grade_scale IS NULL;

ALTER TABLE submissions
    ALTER COLUMN grade_scale SET DEFAULT 20,
    ALTER COLUMN grade_scale SET NOT NULL;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'submissions_grade_scale_check'
          AND conrelid = 'submissions'::regclass
    ) THEN
        ALTER TABLE submissions
            ADD CONSTRAINT submissions_grade_scale_check
            CHECK (grade_scale IN (20, 100));
    END IF;
END
$$;