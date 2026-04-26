CREATE TYPE job_engine AS ENUM ('mir', 'rdf', 'lpg', 'bayessian');
CREATE TYPE job_status AS ENUM ('pending', 'in_progress', 'success', 'failure', 'retryable');

CREATE TABLE IF NOT EXISTS jobs (
    job_id TEXT PRIMARY KEY,
    context_id INTEGER NOT NULL REFERENCES contexts (id) ON DELETE CASCADE,
    engine job_engine NOT NULL DEFAULT 'mir',
    kind TEXT,
    status job_status NOT NULL DEFAULT 'pending',
    stage TEXT,
    progress_pct SMALLINT CHECK (progress_pct BETWEEN 0 AND 100),
    message TEXT,
    result_json JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS jobs_context_id_idx ON jobs (context_id);
CREATE INDEX IF NOT EXISTS jobs_context_status_idx ON jobs (context_id, status);
