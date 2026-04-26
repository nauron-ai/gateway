ALTER TABLE jobs
ADD COLUMN stage_progress_current INTEGER CHECK (stage_progress_current >= 0),
ADD COLUMN stage_progress_total INTEGER CHECK (stage_progress_total >= 0),
ADD COLUMN stage_progress_pct SMALLINT CHECK (stage_progress_pct BETWEEN 0 AND 100);
