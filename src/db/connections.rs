use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use tracing::warn;

#[derive(Clone)]
pub struct ConnectionEventRepository {
    pool: PgPool,
}

#[derive(Debug, Clone)]
pub struct NewConnectionEvent {
    pub service: String,
    pub endpoint: String,
    pub method: String,
    pub status: i32,
    pub latency_ms: i32,
    pub response_bytes: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ConnectionEvent {
    pub id: i64,
    pub service: String,
    pub endpoint: String,
    pub method: String,
    pub status: i32,
    pub latency_ms: i32,
    pub response_bytes: Option<i64>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ConnectionStats {
    pub service: String,
    pub request_count: i64,
    pub error_rate: f64,
    pub p50_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub last_status: i32,
    pub last_seen: DateTime<Utc>,
    pub last_error: Option<String>,
}

impl ConnectionEventRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn spawn_insert(&self, event: NewConnectionEvent) {
        let repo = self.clone();
        tokio::spawn(async move {
            if let Err(err) = repo.insert_event(event).await {
                warn!(target: "metrics", error = %err, "failed to record connection event");
            }
        });
    }

    pub async fn insert_event(&self, event: NewConnectionEvent) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO connection_events (
                service,
                endpoint,
                method,
                status,
                latency_ms,
                response_bytes,
                error
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            event.service,
            event.endpoint,
            event.method,
            event.status,
            event.latency_ms,
            event.response_bytes,
            event.error
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_events_since(
        &self,
        since: DateTime<Utc>,
        limit: i64,
        service: Option<&str>,
    ) -> Result<Vec<ConnectionEvent>, sqlx::Error> {
        sqlx::query_as!(
            ConnectionEvent,
            r#"
            SELECT
                id,
                service,
                endpoint,
                method,
                status,
                latency_ms,
                response_bytes,
                error,
                created_at
            FROM connection_events
            WHERE created_at >= $2
              AND ($3::text IS NULL OR service = $3)
            ORDER BY created_at DESC
            LIMIT $1
            "#,
            limit,
            since,
            service
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn list_stats_since(
        &self,
        since: DateTime<Utc>,
        service: Option<&str>,
    ) -> Result<Vec<ConnectionStats>, sqlx::Error> {
        sqlx::query_as!(
            ConnectionStats,
            r#"
            WITH windowed AS (
                SELECT *
                FROM connection_events
                WHERE created_at >= $1
                  AND ($2::text IS NULL OR service = $2)
            ),
            last_events AS (
                SELECT DISTINCT ON (service)
                    service,
                    status,
                    created_at,
                    error
                FROM windowed
                ORDER BY service, created_at DESC
            )
            SELECT
                w.service,
                COUNT(*)::bigint AS "request_count!",
                COALESCE(
                    (COUNT(*) FILTER (WHERE w.status = 0 OR w.status >= 400))::float
                        / NULLIF(COUNT(*)::float, 0.0),
                    0.0
                )::float8 AS "error_rate!",
                percentile_cont(0.5) WITHIN GROUP (ORDER BY latency_ms)::float8 AS "p50_latency_ms!",
                percentile_cont(0.95) WITHIN GROUP (ORDER BY latency_ms)::float8 AS "p95_latency_ms!",
                percentile_cont(0.99) WITHIN GROUP (ORDER BY latency_ms)::float8 AS "p99_latency_ms!",
                l.status AS "last_status!",
                l.created_at AS "last_seen!",
                l.error AS "last_error?"
            FROM windowed w
            JOIN last_events l ON l.service = w.service
            GROUP BY w.service, l.status, l.created_at, l.error
            ORDER BY w.service
            "#,
            since,
            service
        )
        .fetch_all(&self.pool)
        .await
    }
}
