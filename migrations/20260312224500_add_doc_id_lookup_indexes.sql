CREATE INDEX IF NOT EXISTS context_files_file_attached_id_idx
    ON context_files (file_id, attached_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS jobs_pipeline_engine_updated_job_idx
    ON jobs (pipeline_id, engine, updated_at DESC, job_id DESC);
