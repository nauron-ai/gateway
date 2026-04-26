use std::sync::Arc;

mod ingest_job;

use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode as HttpStatus},
    response::IntoResponse,
    routing::post,
};
use serde::Deserialize;
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

pub(crate) use self::ingest_job::CreateIngestJobResponse;
use self::ingest_job::{build_ingest_job_id, default_ingest_type_spec, validate_ingest_type_spec};
use crate::auth::AuthUser;
use crate::db::jobs::{JobEngine, JobSnapshotUpsert, JobStatus};
use crate::error::GatewayError;
use crate::inferencer::{
    IngestSchemaField, OneshotFailureResponse, OneshotRequest, OneshotResult,
    OneshotSuccessResponse,
};
use crate::state::AppState;

const IDEMPOTENCY_KEY_HEADER: &str = "Idempotency-Key";
const INGEST_MOVED_MESSAGE: &str = "endpoint moved to /v1/contexts/{context_id}/ingest/jobs";
const INGEST_IDEMPOTENCY_PREFIX: &str = "ingest";

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateOneshotBody {
    pub prompt: String,
    pub language: Option<String>,
    pub metadata: Option<Value>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/contexts/{context_id}/oneshots", post(create_oneshot))
        .route("/v1/contexts/{context_id}/ingest", post(ingest_gone))
        .route(
            "/v1/contexts/{context_id}/ingest/jobs",
            post(create_ingest_job),
        )
}

/// One-shot prompt against a context (non-streaming).
#[utoipa::path(
    post,
    path = "/v1/contexts/{context_id}/oneshots",
    request_body = CreateOneshotBody,
    params(
        ("context_id" = i64, Path, description = "Context identifier")
    ),
    responses(
        (status = 200, description = "Oneshot completed", body = OneshotSuccessResponse),
        (status = 400, description = "LLM failure or validation", body = OneshotFailureResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Chat"
)]
pub async fn create_oneshot(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthUser>,
    Path(context_id): Path<i64>,
    Json(body): Json<CreateOneshotBody>,
) -> Result<impl axum::response::IntoResponse, GatewayError> {
    let user_id = user.id.to_string();

    let request = OneshotRequest {
        context_id,
        prompt: body.prompt,
        language: body.language,
        metadata: body.metadata,
    };

    let result = state.inferencer_client.oneshot(&request, &user_id).await?;

    let response = match result {
        OneshotResult::Success(payload) => (HttpStatus::OK, Json(payload)).into_response(),
        OneshotResult::Failure(payload) => (HttpStatus::BAD_REQUEST, Json(payload)).into_response(),
    };

    Ok(response)
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateIngestBody {
    pub schema: Vec<IngestSchemaField>,
    pub instruction: Option<String>,
    pub language: Option<String>,
    pub metadata: Option<Value>,
}

#[utoipa::path(
    post,
    path = "/v1/contexts/{context_id}/ingest/jobs",
    summary = "Create ingest job",
    description = "Enqueues a structured ingest job for asynchronous processing. Use /v1/jobs/{job_id} to poll status/result.",
    request_body = CreateIngestBody,
    params(
        ("context_id" = i64, Path, description = "Context identifier"),
        ("Idempotency-Key" = String, Header, description = "Optional key to deduplicate job creation")
    ),
    responses(
        (status = 202, description = "Ingest job created", body = CreateIngestJobResponse),
        (status = 400, description = "Validation error", body = crate::error::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Chat"
)]
pub async fn create_ingest_job(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthUser>,
    Path(context_id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<CreateIngestBody>,
) -> Result<impl axum::response::IntoResponse, GatewayError> {
    let context_id: i32 = context_id
        .try_into()
        .map_err(|_| GatewayError::InvalidField {
            field: "context_id".to_string(),
            message: "context_id out of range".to_string(),
        })?;
    let user_id = user.id.to_string();
    let job_id = build_ingest_job_id(context_id, &user_id, &headers);

    if let Some(existing) = state.job_repo.get(job_id).await? {
        if existing.engine != JobEngine::Ingest {
            return Err(GatewayError::Conflict(format!(
                "job_id {job_id} already exists"
            )));
        }
        if existing.context_id != context_id {
            return Err(GatewayError::Conflict(format!(
                "job_id {job_id} belongs to a different context"
            )));
        }
        return Ok((
            HttpStatus::ACCEPTED,
            Json(CreateIngestJobResponse::new(job_id)),
        ));
    }

    let pipeline_id = Uuid::new_v4();
    let upsert = JobSnapshotUpsert {
        job_id,
        context_id,
        file_id: None,
        pipeline_id: Some(pipeline_id),
        source_job_id: None,
        engine: JobEngine::Ingest,
        kind: None,
        status: JobStatus::Pending,
        stage: Some(nauron_contracts::IngestStage::Queued.into()),
        progress_pct: None,
        stage_progress_current: None,
        stage_progress_total: None,
        stage_progress_pct: None,
        message: None,
        result_json: None,
        updated_at: chrono::Utc::now(),
    };
    state.job_repo.upsert_snapshot(upsert).await?;

    for field in &body.schema {
        validate_ingest_type_spec(field.r#type.as_ref())?;
    }

    let schema = body
        .schema
        .into_iter()
        .map(|field| nauron_contracts::IngestSchemaField {
            key: field.key,
            name: field.name,
            description: field.description,
            r#type: field.r#type.unwrap_or_else(default_ingest_type_spec),
            required: field.required.is_some_and(|value| value),
        })
        .collect();
    let start = nauron_contracts::IngestStart {
        schema_version: nauron_contracts::SchemaVersion::default(),
        job_id,
        context_id,
        user_id: Some(user_id),
        schema,
        instruction: body.instruction,
        language: body.language,
        metadata: body.metadata,
        submitted_at: Some(chrono::Utc::now()),
    };
    if let Err(err) = state.ingest_publisher.publish_json(job_id, &start).await {
        tracing::error!(job_id = %job_id, context_id, error = %err, "failed to publish ingest.start");
        return Err(err.into());
    }

    Ok((
        HttpStatus::ACCEPTED,
        Json(CreateIngestJobResponse::new(job_id)),
    ))
}

pub async fn ingest_gone() -> impl axum::response::IntoResponse {
    let body = crate::error::ErrorResponse::new(INGEST_MOVED_MESSAGE);
    (HttpStatus::GONE, Json(body))
}
