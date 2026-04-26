-- Add a neutral "retired" status to flag superseded jobs without impacting pipeline health.
ALTER TYPE job_status ADD VALUE IF NOT EXISTS 'retired';
