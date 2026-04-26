DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'job_kind') THEN
        CREATE TYPE job_kind AS ENUM ('reused', 'mir_linked', 'fanout', 'retry');
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'mir_job_stage') THEN
        CREATE TYPE mir_job_stage AS ENUM (
            'received',
            'detect',
            'pandoc_run',
            'pandoc_assemble',
            'processing_run',
            'processing_assemble',
            'upload',
            'completed'
        );
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'rdf_job_stage') THEN
        CREATE TYPE rdf_job_stage AS ENUM (
            'received',
            'fetch_text',
            'segment',
            'information_extraction',
            'shacl_validate',
            'reasoning',
            'persist',
            'completed'
        );
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'ingest_job_stage') THEN
        CREATE TYPE ingest_job_stage AS ENUM ('queued', 'received', 'llm', 'persist', 'completed');
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'conditions_job_stage') THEN
        CREATE TYPE conditions_job_stage AS ENUM ('queued', 'received', 'retrieve', 'reason', 'completed');
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'chat_mode') THEN
        CREATE TYPE chat_mode AS ENUM ('emb', 'rdf-emb', 'bn');
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'user_theme') THEN
        CREATE TYPE user_theme AS ENUM ('dark');
    END IF;
END $$;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM jobs
        WHERE kind IS NOT NULL
          AND kind NOT IN ('reused', 'mir_linked', 'fanout', 'retry')
    ) THEN
        RAISE EXCEPTION 'jobs.kind contains values outside job_kind';
    END IF;

    IF EXISTS (
        SELECT 1 FROM jobs
        WHERE stage IS NOT NULL
          AND (
            (engine = 'mir'::job_engine AND stage NOT IN (
                'received',
                'detect',
                'pandoc_run',
                'pandoc_assemble',
                'processing_run',
                'processing_assemble',
                'upload',
                'completed'
            ))
            OR (engine = 'rdf'::job_engine AND stage NOT IN (
                'received',
                'fetch_text',
                'segment',
                'information_extraction',
                'shacl_validate',
                'reasoning',
                'persist',
                'completed'
            ))
            OR (engine = 'ingest'::job_engine AND stage NOT IN (
                'queued',
                'received',
                'llm',
                'persist',
                'completed'
            ))
            OR (engine = 'conditions'::job_engine AND stage NOT IN (
                'queued',
                'received',
                'retrieve',
                'reason',
                'completed'
            ))
            OR (engine IN ('lpg'::job_engine, 'bayessian'::job_engine))
          )
    ) THEN
        RAISE EXCEPTION 'jobs.stage contains values outside stage enums';
    END IF;

    IF EXISTS (
        SELECT 1 FROM user_settings
        WHERE default_chat_mode IS NOT NULL
          AND default_chat_mode NOT IN ('emb', 'rdf-emb', 'bn')
    ) THEN
        RAISE EXCEPTION 'user_settings.default_chat_mode contains values outside chat_mode';
    END IF;

    IF EXISTS (
        SELECT 1 FROM user_settings
        WHERE theme IS NOT NULL
          AND theme NOT IN ('dark')
    ) THEN
        RAISE EXCEPTION 'user_settings.theme contains values outside user_theme';
    END IF;
END $$;

ALTER TABLE jobs
    ADD COLUMN IF NOT EXISTS mir_stage mir_job_stage,
    ADD COLUMN IF NOT EXISTS rdf_stage rdf_job_stage,
    ADD COLUMN IF NOT EXISTS ingest_stage ingest_job_stage,
    ADD COLUMN IF NOT EXISTS conditions_stage conditions_job_stage;

UPDATE jobs
SET
    mir_stage = CASE WHEN engine = 'mir'::job_engine THEN stage::mir_job_stage ELSE NULL END,
    rdf_stage = CASE WHEN engine = 'rdf'::job_engine THEN stage::rdf_job_stage ELSE NULL END,
    ingest_stage = CASE WHEN engine = 'ingest'::job_engine THEN stage::ingest_job_stage ELSE NULL END,
    conditions_stage = CASE WHEN engine = 'conditions'::job_engine THEN stage::conditions_job_stage ELSE NULL END
WHERE stage IS NOT NULL;

DROP INDEX IF EXISTS jobs_mir_linked_active_idx;

ALTER TABLE jobs
    ALTER COLUMN kind TYPE job_kind USING kind::job_kind,
    DROP COLUMN stage;

CREATE UNIQUE INDEX jobs_mir_linked_active_idx
    ON jobs (pipeline_id, file_id, source_job_id)
    WHERE kind = 'mir_linked'::job_kind
      AND status <> 'retired'::job_status;

ALTER TABLE user_settings
    ALTER COLUMN default_chat_mode TYPE chat_mode USING default_chat_mode::chat_mode,
    ALTER COLUMN theme TYPE user_theme USING theme::user_theme;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'jobs_stage_matches_engine'
    ) THEN
        ALTER TABLE jobs
            ADD CONSTRAINT jobs_stage_matches_engine CHECK (
                (
                    engine = 'mir'::job_engine
                    AND rdf_stage IS NULL
                    AND ingest_stage IS NULL
                    AND conditions_stage IS NULL
                )
                OR (
                    engine = 'rdf'::job_engine
                    AND mir_stage IS NULL
                    AND ingest_stage IS NULL
                    AND conditions_stage IS NULL
                )
                OR (
                    engine = 'ingest'::job_engine
                    AND mir_stage IS NULL
                    AND rdf_stage IS NULL
                    AND conditions_stage IS NULL
                )
                OR (
                    engine = 'conditions'::job_engine
                    AND mir_stage IS NULL
                    AND rdf_stage IS NULL
                    AND ingest_stage IS NULL
                )
                OR (
                    engine IN ('lpg'::job_engine, 'bayessian'::job_engine)
                    AND mir_stage IS NULL
                    AND rdf_stage IS NULL
                    AND ingest_stage IS NULL
                    AND conditions_stage IS NULL
                )
            );
    END IF;
END $$;
