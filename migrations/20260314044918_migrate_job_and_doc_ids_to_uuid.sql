CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE OR REPLACE FUNCTION pg_temp.try_uuid(value TEXT) RETURNS UUID
LANGUAGE plpgsql
AS $$
BEGIN
    IF value IS NULL THEN
        RETURN NULL;
    END IF;
    RETURN value::UUID;
EXCEPTION
    WHEN invalid_text_representation THEN
        RETURN NULL;
END;
$$;

CREATE TEMP TABLE legacy_uuid_map ON COMMIT DROP AS
WITH legacy_ids AS (
    SELECT job_id AS legacy_id FROM jobs
    UNION ALL
    SELECT source_job_id FROM jobs
    UNION ALL
    SELECT mir_job_id FROM files
    UNION ALL
    SELECT doc_id FROM files
    UNION ALL
    SELECT result_json ->> 'job_id' FROM jobs WHERE result_json IS NOT NULL
    UNION ALL
    SELECT result_json ->> 'doc_id' FROM jobs WHERE result_json IS NOT NULL
    UNION ALL
    SELECT match ->> 'doc_id'
    FROM jobs
    CROSS JOIN LATERAL jsonb_array_elements(COALESCE(result_json -> 'response' -> 'results', '[]'::jsonb)) AS result
    CROSS JOIN LATERAL jsonb_array_elements(COALESCE(result -> 'matches', '[]'::jsonb)) AS match
)
SELECT legacy_id,
       COALESCE(pg_temp.try_uuid(legacy_id), gen_random_uuid()) AS uuid_id
FROM (
    SELECT DISTINCT legacy_id
    FROM legacy_ids
    WHERE legacy_id IS NOT NULL
) AS deduplicated;

ALTER TABLE legacy_uuid_map
    ADD PRIMARY KEY (legacy_id);

CREATE OR REPLACE FUNCTION pg_temp.map_uuid(value TEXT) RETURNS UUID
LANGUAGE sql
STABLE
AS $$
    SELECT CASE
        WHEN value IS NULL THEN NULL
        ELSE COALESCE(
            (SELECT uuid_id FROM legacy_uuid_map WHERE legacy_id = value),
            pg_temp.try_uuid(value)
        )
    END
$$;

CREATE OR REPLACE FUNCTION pg_temp.map_uuid_text(value TEXT) RETURNS TEXT
LANGUAGE sql
STABLE
AS $$
    SELECT CASE
        WHEN value IS NULL THEN NULL
        ELSE COALESCE(
            (SELECT uuid_id::TEXT FROM legacy_uuid_map WHERE legacy_id = value),
            pg_temp.try_uuid(value)::TEXT
        )
    END
$$;

CREATE OR REPLACE FUNCTION pg_temp.remap_top_level_uuid(payload JSONB, field_name TEXT) RETURNS JSONB
LANGUAGE plpgsql
AS $$
BEGIN
    IF payload IS NULL THEN
        RETURN NULL;
    END IF;
    IF payload ? field_name AND jsonb_typeof(payload -> field_name) = 'string' THEN
        RETURN jsonb_set(
            payload,
            ARRAY[field_name],
            to_jsonb(pg_temp.map_uuid_text(payload ->> field_name)),
            true
        );
    END IF;
    RETURN payload;
END;
$$;

CREATE OR REPLACE FUNCTION pg_temp.remap_condition_matches(payload JSONB) RETURNS JSONB
LANGUAGE sql
STABLE
AS $$
    SELECT CASE
        WHEN payload IS NULL
            OR COALESCE(jsonb_typeof(payload -> 'response' -> 'results'), '') <> 'array'
        THEN payload
        ELSE jsonb_set(
            payload,
            '{response,results}',
            COALESCE(
                (
                    SELECT jsonb_agg(
                        CASE
                            WHEN COALESCE(jsonb_typeof(result -> 'matches'), '') <> 'array'
                            THEN result
                            ELSE jsonb_set(
                                result,
                                '{matches}',
                                COALESCE(
                                    (
                                        SELECT jsonb_agg(
                                            CASE
                                                WHEN jsonb_typeof(match -> 'doc_id') = 'string'
                                                THEN jsonb_set(
                                                    match,
                                                    '{doc_id}',
                                                    to_jsonb(pg_temp.map_uuid_text(match ->> 'doc_id')),
                                                    true
                                                )
                                                ELSE match
                                            END
                                        )
                                        FROM jsonb_array_elements(result -> 'matches') AS match
                                    ),
                                    '[]'::jsonb
                                ),
                                true
                            )
                        END
                    )
                    FROM jsonb_array_elements(payload -> 'response' -> 'results') AS result
                ),
                '[]'::jsonb
            ),
            true
        )
    END
$$;

CREATE OR REPLACE FUNCTION pg_temp.remap_result_json(payload JSONB) RETURNS JSONB
LANGUAGE plpgsql
AS $$
DECLARE
    updated JSONB;
BEGIN
    IF payload IS NULL THEN
        RETURN NULL;
    END IF;
    updated := pg_temp.remap_top_level_uuid(payload, 'job_id');
    updated := pg_temp.remap_top_level_uuid(updated, 'doc_id');
    updated := pg_temp.remap_condition_matches(updated);
    RETURN updated;
END;
$$;

UPDATE jobs
SET result_json = pg_temp.remap_result_json(result_json)
WHERE result_json IS NOT NULL;

ALTER TABLE jobs DROP CONSTRAINT IF EXISTS jobs_pkey CASCADE;
ALTER TABLE jobs DROP CONSTRAINT IF EXISTS jobs_source_job_id_fkey;
ALTER TABLE files DROP CONSTRAINT IF EXISTS files_mir_job_id_fkey;

DROP INDEX IF EXISTS jobs_source_job_id_idx;
DROP INDEX IF EXISTS jobs_mir_linked_active_idx;
DROP INDEX IF EXISTS jobs_pipeline_engine_updated_job_idx;
DROP INDEX IF EXISTS files_doc_id_idx;

ALTER TABLE jobs
    ALTER COLUMN job_id TYPE UUID USING pg_temp.map_uuid(job_id),
    ALTER COLUMN source_job_id TYPE UUID USING pg_temp.map_uuid(source_job_id);

ALTER TABLE files
    ALTER COLUMN mir_job_id TYPE UUID USING pg_temp.map_uuid(mir_job_id),
    ALTER COLUMN doc_id TYPE UUID USING pg_temp.map_uuid(doc_id);

ALTER TABLE jobs
    ADD PRIMARY KEY (job_id);

ALTER TABLE jobs
    ADD CONSTRAINT jobs_source_job_id_fkey
    FOREIGN KEY (source_job_id)
    REFERENCES jobs (job_id);

ALTER TABLE files
    ADD CONSTRAINT files_mir_job_id_fkey
    FOREIGN KEY (mir_job_id)
    REFERENCES jobs (job_id);

CREATE INDEX jobs_source_job_id_idx
    ON jobs (source_job_id);

CREATE UNIQUE INDEX jobs_mir_linked_active_idx
    ON jobs (pipeline_id, file_id, source_job_id)
    WHERE kind = 'mir_linked'
      AND source_job_id IS NOT NULL
      AND status <> 'retired'::job_status;

CREATE INDEX jobs_pipeline_engine_updated_job_idx
    ON jobs (pipeline_id, engine, updated_at DESC, job_id DESC);

CREATE UNIQUE INDEX files_doc_id_idx
    ON files (doc_id)
    WHERE doc_id IS NOT NULL;
