use std::{sync::Arc, time::Instant};

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;

use crate::state::AppState;
use nauron_contracts::health::{ComponentStatus, HealthResponse, ServiceStatus};
use utoipa::ToSchema;

#[utoipa::path(
    get,
    path = "/healthz",
    summary = "Health check",
    description = "Checks connectivity to all gateway dependencies: database, Kafka queues (MIR/RDF), object storage, \
inferencer service, and vector API. Returns 200 if all healthy, 503 if any degraded. Includes latency metrics per component.",
    responses(
        (status = 200, description = "All dependencies are healthy", body = HealthResponse<GatewayComponents>),
        (status = 503, description = "At least one dependency is unavailable", body = HealthResponse<GatewayComponents>)
    ),
    security([]),
    tag = "Health"
)]
pub(crate) async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    async fn measure<F, Fut, T, E>(f: F) -> (Result<T, E>, u128)
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        let start = Instant::now();
        let res = f().await;
        let elapsed = start.elapsed();
        // Round up to the nearest millisecond to avoid reporting 0ms for sub-ms checks.
        let latency_ms = std::cmp::max(1, elapsed.as_micros().div_ceil(1000));
        (res, latency_ms)
    }

    let (db_status, db_latency) = measure(|| state.context_repo.health_check()).await;
    let (kafka_status, kafka_latency) =
        measure(|| async { state.mir_publisher.check_health() }).await;
    let (rdf_kafka_status, rdf_kafka_latency) =
        measure(|| async { state.rdf_publisher.check_health() }).await;
    let (storage_status, storage_latency) =
        measure(|| state.storage.check_bucket(&state.config.input_bucket)).await;
    let (inferencer_status, inferencer_latency) =
        measure(|| state.inferencer_client.check_health()).await;
    let (vector_status, vector_latency) = measure(|| state.vector_api_client.health_check()).await;

    let services = GatewayComponents {
        database: ComponentStatus {
            status: if db_status.is_ok() {
                ServiceStatus::Ok
            } else {
                ServiceStatus::Degraded
            },
            latency_ms: db_latency,
            message: db_status.err().map(|e| e.to_string()),
            detail: Some("connected".to_string()),
        },
        mir_queue: ComponentStatus {
            status: if kafka_status.is_ok() {
                ServiceStatus::Ok
            } else {
                ServiceStatus::Degraded
            },
            latency_ms: kafka_latency,
            message: kafka_status.err().map(|e| e.to_string()),
            detail: Some("connected".to_string()),
        },
        rdf_queue: ComponentStatus {
            status: if rdf_kafka_status.is_ok() {
                ServiceStatus::Ok
            } else {
                ServiceStatus::Degraded
            },
            latency_ms: rdf_kafka_latency,
            message: rdf_kafka_status.err().map(|e| e.to_string()),
            detail: Some("connected".to_string()),
        },
        storage: ComponentStatus {
            status: if storage_status.is_ok() {
                ServiceStatus::Ok
            } else {
                ServiceStatus::Degraded
            },
            latency_ms: storage_latency,
            message: storage_status.err().map(|e| e.to_string()),
            detail: Some("reachable".to_string()),
        },
        inferencer: ComponentStatus {
            status: if inferencer_status.is_ok() {
                ServiceStatus::Ok
            } else {
                ServiceStatus::Degraded
            },
            latency_ms: inferencer_latency,
            message: inferencer_status.err().map(|e| e.to_string()),
            detail: Some("reachable".to_string()),
        },
        vector_api: ComponentStatus {
            status: if vector_status.is_ok() {
                ServiceStatus::Ok
            } else {
                ServiceStatus::Degraded
            },
            latency_ms: vector_latency,
            message: vector_status.err().map(|e| e.to_string()),
            detail: Some("reachable".to_string()),
        },
    };

    let overall = if services.is_healthy() {
        ServiceStatus::Ok
    } else {
        ServiceStatus::Degraded
    };
    let status_code = if overall == ServiceStatus::Ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status_code,
        Json(HealthResponse {
            service: "gateway".to_string(),
            status: overall,
            uptime_seconds: 0, // Gateway doesn't track uptime yet
            components: services,
        }),
    )
}

#[derive(Serialize, ToSchema)]
pub(crate) struct GatewayComponents {
    database: ComponentStatus,
    mir_queue: ComponentStatus,
    rdf_queue: ComponentStatus,
    storage: ComponentStatus,
    inferencer: ComponentStatus,
    vector_api: ComponentStatus,
}

impl GatewayComponents {
    fn is_healthy(&self) -> bool {
        self.database.status.is_success()
            && self.mir_queue.status.is_success()
            && self.rdf_queue.status.is_success()
            && self.storage.status.is_success()
            && self.inferencer.status.is_success()
            && self.vector_api.status.is_success()
    }
}
