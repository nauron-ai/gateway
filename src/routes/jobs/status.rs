use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::db::files::FileRecord;
use crate::db::jobs::{JobEngine, JobStatus};
use crate::error::{ErrorResponse, GatewayError};
use crate::job_mode::JobLaunchMode;
use crate::state::AppState;
use crate::tracker::{JobResultPayload, JobSnapshot};

use super::JobStageResponse;

#[utoipa::path(
    get,
    path = "/v1/jobs/{job_id}",
    summary = "Get job status",
    description = "Returns current state of a processing job including status, progress percentage, current stage, \
and result payload if completed. Use for polling job completion or displaying progress.",
    params(
        ("job_id" = Uuid, Path, description = "Job identifier")
    ),
    responses(
        (status = 200, description = "Current job state", body = JobStatusResponse),
        (status = 404, description = "Job not found", body = ErrorResponse)
    ),
    tag = "Jobs"
)]
pub(crate) async fn job_status(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
) -> Result<impl IntoResponse, GatewayError> {
    let snapshot = load_snapshot(&state, job_id).await?;
    let file = match snapshot.file_id {
        Some(file_id) => state.file_repo.find_by_id(file_id).await?,
        None => None,
    };
    Ok(Json(JobStatusResponse::from_snapshot(
        snapshot,
        file.as_ref(),
    )))
}

#[derive(Serialize, Clone, ToSchema)]
pub(crate) struct JobStatusResponse {
    job_id: Uuid,
    pipeline_id: uuid::Uuid,
    context_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_id: Option<i64>,
    job_mode: JobLaunchMode,
    engine: JobEngine,
    state: JobStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    stage: Option<JobStageResponse>,
    percent: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stage_current: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stage_total: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stage_percent: Option<u8>,
    message: Option<String>,
    updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Object)]
    result: Option<JobResultPayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    artifact_uri: Option<String>,
}

impl JobStatusResponse {
    pub(crate) fn from_snapshot(snapshot: JobSnapshot, file: Option<&FileRecord>) -> Self {
        Self {
            job_id: snapshot.job_id,
            pipeline_id: snapshot.pipeline_id,
            context_id: Some(snapshot.context_id),
            file_id: snapshot.file_id,
            job_mode: JobLaunchMode::from_kind(snapshot.kind),
            engine: snapshot.engine,
            state: snapshot.status,
            stage: snapshot.stage.map(JobStageResponse::from),
            percent: snapshot.percent,
            stage_current: snapshot.stage_current,
            stage_total: snapshot.stage_total,
            stage_percent: snapshot.stage_percent,
            message: snapshot.message,
            updated_at: snapshot.updated_at,
            result: snapshot.result,
            artifact_uri: file.and_then(|file| file.mir_artifact_uri.clone()),
        }
    }
}

pub(crate) async fn load_snapshot(
    state: &Arc<AppState>,
    job_id: Uuid,
) -> Result<JobSnapshot, GatewayError> {
    let record = state
        .job_repo
        .get(job_id)
        .await?
        .ok_or_else(|| GatewayError::NotFound(job_id.to_string()))?;
    JobSnapshot::try_from(record).map_err(GatewayError::from)
}
