use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{
    db::connections::{ConnectionEvent, ConnectionStats},
    error::GatewayError,
    state::AppState,
};

#[utoipa::path(
    get,
    path = "/admin/connections",
    summary = "Get connection metrics (admin)",
    description = "Returns metrics for outbound gateway calls to downstream services (inferencer, vector_api, etc.). \
Shows request counts, error rates, latency percentiles (p50/p95/p99), and recent events. Admin only.",
    params(ConnectionEventsQuery),
    responses(
        (status = 200, description = "Summary of recent outbound gateway calls", body = ConnectionEventsResponse)
    ),
    tag = "Admin"
)]
pub async fn list_connection_events(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ConnectionEventsQuery>,
) -> Result<impl IntoResponse, GatewayError> {
    let limit = resolve_limit(query.limit)?;
    let hours = resolve_hours(query.hours)?;
    let since = Utc::now() - Duration::hours(hours);
    let service_filter = query.service.as_deref();

    let stats = state
        .connection_repo
        .list_stats_since(since, service_filter)
        .await?;
    let events = state
        .connection_repo
        .list_events_since(since, limit, service_filter)
        .await?;

    let services = stats
        .into_iter()
        .map(ServiceConnectionSummary::from)
        .collect();
    let recent = events
        .into_iter()
        .map(RecentConnectionEvent::from)
        .collect();

    Ok(Json(ConnectionEventsResponse {
        window_hours: hours,
        services,
        recent,
    }))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ConnectionEventsQuery {
    /// Optional filter to a single downstream service (e.g. "inferencer", "vector_api")
    pub service: Option<String>,
    /// Max number of events to return (default 200, max 1000)
    pub limit: Option<i64>,
    /// Lookback window in hours (default 24, max 168)
    pub hours: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConnectionEventsResponse {
    pub window_hours: i64,
    pub services: Vec<ServiceConnectionSummary>,
    pub recent: Vec<RecentConnectionEvent>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ServiceConnectionSummary {
    pub service: String,
    pub requests: i64,
    pub error_rate: f64,
    pub p50_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub last_status: i32,
    pub last_seen: chrono::DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RecentConnectionEvent {
    pub id: i64,
    pub service: String,
    pub endpoint: String,
    pub method: String,
    pub status: i32,
    pub latency_ms: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
}

impl From<ConnectionStats> for ServiceConnectionSummary {
    fn from(stats: ConnectionStats) -> Self {
        Self {
            service: stats.service,
            requests: stats.request_count,
            error_rate: stats.error_rate,
            p50_latency_ms: stats.p50_latency_ms,
            p95_latency_ms: stats.p95_latency_ms,
            p99_latency_ms: stats.p99_latency_ms,
            last_status: stats.last_status,
            last_seen: stats.last_seen,
            last_error: stats.last_error,
        }
    }
}

impl From<ConnectionEvent> for RecentConnectionEvent {
    fn from(event: ConnectionEvent) -> Self {
        Self {
            id: event.id,
            service: event.service,
            endpoint: event.endpoint,
            method: event.method,
            status: event.status,
            latency_ms: event.latency_ms,
            response_bytes: event.response_bytes,
            error: event.error,
            created_at: event.created_at,
        }
    }
}

fn resolve_limit(raw: Option<i64>) -> Result<i64, GatewayError> {
    const DEFAULT_LIMIT: i64 = 200;
    const MAX_LIMIT: i64 = 10_000;
    match raw {
        None => Ok(DEFAULT_LIMIT),
        Some(value) if (1..=MAX_LIMIT).contains(&value) => Ok(value),
        _ => Err(GatewayError::InvalidField {
            field: "limit".into(),
            message: format!("must be between 1 and {MAX_LIMIT}"),
        }),
    }
}

fn resolve_hours(raw: Option<i64>) -> Result<i64, GatewayError> {
    const DEFAULT_HOURS: i64 = 24;
    const MAX_HOURS: i64 = 168;
    match raw {
        None => Ok(DEFAULT_HOURS),
        Some(value) if (1..=MAX_HOURS).contains(&value) => Ok(value),
        _ => Err(GatewayError::InvalidField {
            field: "hours".into(),
            message: format!("must be between 1 and {MAX_HOURS}"),
        }),
    }
}
