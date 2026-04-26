CREATE EXTENSION IF NOT EXISTS "pgcrypto";

ALTER TABLE context_files
    ADD COLUMN pipeline_id UUID;

UPDATE context_files
SET pipeline_id = gen_random_uuid()
WHERE pipeline_id IS NULL;

ALTER TABLE context_files
    ALTER COLUMN pipeline_id SET NOT NULL;

CREATE UNIQUE INDEX context_files_pipeline_id_idx
    ON context_files (pipeline_id);

ALTER TABLE jobs
    ADD COLUMN pipeline_id UUID;

UPDATE jobs AS j
SET pipeline_id = cf.pipeline_id
FROM context_files AS cf
WHERE j.context_id = cf.context_id
  AND j.file_id = cf.file_id;

UPDATE jobs
SET pipeline_id = gen_random_uuid()
WHERE pipeline_id IS NULL;

ALTER TABLE jobs
    ALTER COLUMN pipeline_id SET NOT NULL;

CREATE INDEX jobs_pipeline_id_idx
    ON jobs (pipeline_id);
