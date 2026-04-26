ALTER TABLE files
    ADD COLUMN IF NOT EXISTS doc_id TEXT;

UPDATE files
SET doc_id = mir_job_id
WHERE doc_id IS NULL
  AND mir_job_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS files_doc_id_idx
    ON files (doc_id)
    WHERE doc_id IS NOT NULL;
