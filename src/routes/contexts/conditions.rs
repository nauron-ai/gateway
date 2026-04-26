use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode as HttpStatus},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::ensure_context_owner;
use crate::auth::AuthUser;
use crate::db::jobs::{JobEngine, JobSnapshotUpsert, JobStatus};
use crate::idempotency::build_deterministic_job_id;
use crate::{error::GatewayError, state::AppState};
use nauron_contracts::conditions::{
    ConditionContextMode, ConditionEvaluationOptions, ConditionEvaluationRequest, ConditionLimits,
    ConditionSpec, ConditionValidationError, validate_request,
};

const IDEMPOTENCY_KEY_HEADER: &str = "Idempotency-Key";
const CONDITIONS_EVALUATE_IDEMPOTENCY_PREFIX: &str = "conditions_evaluate";

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct EvaluateConditionsRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_doc_id: Option<Uuid>,
    pub conditions: Vec<ConditionSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<ConditionEvaluationOptions>,
}

fn to_validation_error(err: ConditionValidationError) -> GatewayError {
    GatewayError::InvalidField {
        field: err.field,
        message: err.message,
    }
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct CreateEvaluateConditionsJobResponse {
    pub job_id: Uuid,
    pub job_status_url: String,
}

impl CreateEvaluateConditionsJobResponse {
    fn new(job_id: Uuid) -> Self {
        Self {
            job_status_url: format!("/v1/jobs/{job_id}"),
            job_id,
        }
    }
}

#[utoipa::path(
    post,
    path = "/v1/contexts/{context_id}/conditions/evaluate/jobs",
    tag = "Conditions",
    summary = "Create conditions evaluate job",
    description = "Enqueues a conditions evaluate job for asynchronous processing. Use /v1/jobs/{job_id} to poll status/result.",
    params(
        ("context_id" = i32, Path, description = "Context identifier"),
        ("Idempotency-Key" = String, Header, description = "Optional key to deduplicate job creation")
    ),
    request_body(content = EvaluateConditionsRequest),
    responses(
        (status = 202, description = "Evaluate job created", body = CreateEvaluateConditionsJobResponse),
        (status = 400, description = "Validation error", body = crate::error::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse)
    )
)]
pub async fn create_evaluate_conditions_job(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthUser>,
    Path(context_id): Path<i32>,
    headers: HeaderMap,
    Json(payload): Json<EvaluateConditionsRequest>,
) -> Result<impl IntoResponse, GatewayError> {
    let context = ensure_context_owner(&state, context_id, &user).await?;
    let limits = ConditionLimits::default();
    let user_id = user.id.to_string();
    let job_id = build_conditions_evaluate_job_id(context_id, &user_id, &headers);

    if let Some(existing) = state.job_repo.get(job_id).await? {
        if existing.engine != JobEngine::Conditions {
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
            Json(CreateEvaluateConditionsJobResponse::new(job_id)),
        ));
    }

    let internal_request = ConditionEvaluationRequest {
        context_id: context_id as i64,
        document_hint: payload.document_hint,
        query_hint: payload.query_hint,
        target_doc_id: payload.target_doc_id,
        conditions: payload.conditions,
        options: payload.options,
        context_mode: Some(match context.mode {
            crate::db::contexts::ContextMode::Emb => ConditionContextMode::Emb,
            crate::db::contexts::ContextMode::Rdf => ConditionContextMode::Rdf,
            crate::db::contexts::ContextMode::Lpg => ConditionContextMode::Lpg,
        }),
    };
    validate_request(&internal_request, limits).map_err(to_validation_error)?;

    let pipeline_id = Uuid::new_v4();
    let upsert = JobSnapshotUpsert {
        job_id,
        context_id,
        file_id: None,
        pipeline_id: Some(pipeline_id),
        source_job_id: None,
        engine: JobEngine::Conditions,
        kind: None,
        status: JobStatus::Pending,
        stage: Some(nauron_contracts::conditions::ConditionsEvaluateStage::Queued.into()),
        progress_pct: None,
        stage_progress_current: None,
        stage_progress_total: None,
        stage_progress_pct: None,
        message: None,
        result_json: None,
        updated_at: chrono::Utc::now(),
    };
    state.job_repo.upsert_snapshot(upsert).await?;

    let start = nauron_contracts::conditions::ConditionsEvaluateStart {
        schema_version: nauron_contracts::SchemaVersion::default(),
        job_id,
        context_id,
        document_hint: internal_request.document_hint,
        query_hint: internal_request.query_hint,
        target_doc_id: internal_request.target_doc_id,
        conditions: internal_request.conditions,
        options: internal_request.options,
        context_mode: internal_request.context_mode,
        submitted_at: Some(chrono::Utc::now()),
    };
    state
        .conditions_evaluate_publisher
        .publish_json(job_id, &start)
        .await?;

    Ok((
        HttpStatus::ACCEPTED,
        Json(CreateEvaluateConditionsJobResponse::new(job_id)),
    ))
}

fn build_conditions_evaluate_job_id(context_id: i32, user_id: &str, headers: &HeaderMap) -> Uuid {
    let key = headers
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match key {
        Some(key) => build_deterministic_job_id(
            CONDITIONS_EVALUATE_IDEMPOTENCY_PREFIX,
            context_id,
            user_id,
            key,
        ),
        None => Uuid::new_v4(),
    }
}
