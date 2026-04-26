ALTER TABLE jobs
    ADD COLUMN IF NOT EXISTS source_job_id TEXT REFERENCES jobs (job_id);

CREATE INDEX IF NOT EXISTS jobs_source_job_id_idx
    ON jobs (source_job_id);

UPDATE jobs AS linked
SET source_job_id = files.mir_job_id
FROM files
WHERE linked.kind = 'mir_linked'
  AND linked.file_id = files.id
  AND linked.source_job_id IS NULL
  AND files.mir_job_id IS NOT NULL;

UPDATE jobs
SET status = 'failure'::job_status
WHERE engine = 'mir'::job_engine
  AND status = 'retryable'::job_status
  AND result_json ->> 'status' = 'failure';

WITH ranked AS (
    SELECT job_id,
           ROW_NUMBER() OVER (
               PARTITION BY pipeline_id, file_id, source_job_id
               ORDER BY updated_at DESC, job_id DESC
           ) AS rn
    FROM jobs
    WHERE kind = 'mir_linked'
      AND source_job_id IS NOT NULL
      AND status <> 'retired'::job_status
)
UPDATE jobs
SET status = 'retired'::job_status,
    message = 'superseded duplicate linked MIR snapshot',
    updated_at = now()
FROM ranked
WHERE jobs.job_id = ranked.job_id
  AND ranked.rn > 1;

UPDATE jobs AS linked
SET status = CASE
        WHEN source.status = 'failure'::job_status THEN 'failure'::job_status
        ELSE source.status
    END,
    stage = source.stage,
    progress_pct = source.progress_pct,
    stage_progress_current = source.stage_progress_current,
    stage_progress_total = source.stage_progress_total,
    stage_progress_pct = source.stage_progress_pct,
    message = source.message,
    result_json = COALESCE(source.result_json, linked.result_json),
    updated_at = GREATEST(linked.updated_at, source.updated_at)
FROM jobs AS source
WHERE linked.kind = 'mir_linked'
  AND linked.source_job_id = source.job_id
  AND linked.status <> 'retired'::job_status;

UPDATE jobs
SET status = 'retired'::job_status,
    message = COALESCE(message, 'linked MIR source missing'),
    updated_at = now()
WHERE kind = 'mir_linked'
  AND (
      source_job_id IS NULL
      OR NOT EXISTS (
          SELECT 1
          FROM jobs AS source
          WHERE source.job_id = jobs.source_job_id
      )
  )
  AND status <> 'retired'::job_status;

CREATE UNIQUE INDEX IF NOT EXISTS jobs_mir_linked_active_idx
    ON jobs (pipeline_id, file_id, source_job_id)
    WHERE kind = 'mir_linked'
      AND source_job_id IS NOT NULL
      AND status <> 'retired'::job_status;
