CREATE TABLE IF NOT EXISTS contexts (
    id SERIAL PRIMARY KEY,
    title TEXT,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
