CREATE TABLE connection_events (
    id BIGSERIAL PRIMARY KEY,
    service TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    method TEXT NOT NULL,
    status INTEGER NOT NULL,
    latency_ms INTEGER NOT NULL,
    response_bytes BIGINT,
    error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX connection_events_service_created_idx
    ON connection_events (service, created_at DESC);

CREATE INDEX connection_events_created_idx
    ON connection_events (created_at DESC);
