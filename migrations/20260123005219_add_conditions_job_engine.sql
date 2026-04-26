DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_enum
        JOIN pg_type ON pg_enum.enumtypid = pg_type.oid
        WHERE pg_type.typname = 'job_engine'
          AND pg_enum.enumlabel = 'conditions'
    ) THEN
        ALTER TYPE job_engine ADD VALUE 'conditions';
    END IF;
END $$;

