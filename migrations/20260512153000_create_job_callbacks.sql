CREATE TABLE IF NOT EXISTS job_callbacks (
    job_id UUID PRIMARY KEY REFERENCES jobs(job_id) ON DELETE CASCADE,
    callback_url TEXT NOT NULL,
    callback_secret TEXT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_job_callbacks_updated_at
    ON job_callbacks (updated_at DESC);
