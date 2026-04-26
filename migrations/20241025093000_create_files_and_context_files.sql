CREATE TABLE files (
    id BIGSERIAL PRIMARY KEY,
    sha256 BYTEA NOT NULL,
    size_bytes BIGINT NOT NULL,
    mime TEXT,
    storage_bucket TEXT NOT NULL,
    storage_key TEXT NOT NULL,
    status file_status NOT NULL DEFAULT 'pending',
    mir_job_id TEXT REFERENCES jobs (job_id),
    mir_artifact_uri TEXT,
    mir_artifact_sha256 BYTEA,
    mir_processed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (sha256)
);

CREATE UNIQUE INDEX files_sha256_idx ON files (sha256);

CREATE TABLE context_files (
    id BIGSERIAL PRIMARY KEY,
    context_id INTEGER NOT NULL REFERENCES contexts (id) ON DELETE CASCADE,
    file_id BIGINT NOT NULL REFERENCES files (id) ON DELETE CASCADE,
    origin file_origin NOT NULL,
    original_name TEXT NOT NULL,
    original_path TEXT,
    media_type TEXT,
    attached_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (context_id, file_id)
);

CREATE INDEX context_files_context_id_idx ON context_files (context_id);
CREATE INDEX context_files_file_id_idx ON context_files (file_id);

ALTER TABLE jobs
    ADD COLUMN file_id BIGINT REFERENCES files (id);

CREATE INDEX jobs_file_id_engine_idx ON jobs (file_id, engine);
