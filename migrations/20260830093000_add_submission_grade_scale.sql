-- The API has always persisted and returned a grading scale for submissions.
-- Make that contract explicit for every migrated environment.
ALTER TABLE submissions
    ADD COLUMN IF NOT EXISTS grade_scale SMALLINT;

UPDATE submissions
SET grade_scale = 20
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