use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    db::{
        files::FileRecord,
        jobs::{JobEngine, JobKind, JobListCursor, JobListParams, JobRecord, JobStatus},
    },
    error::{ErrorResponse, GatewayError},
    job_mode::JobLaunchMode,
    routes::jobs::JobStageResponse,
    state::AppState,
};

use super::super::pagination::resolve_limit;

#[utoipa::path(
    get,
    path = "/v1/contexts/{context_id}/jobs",
    summary = "List jobs in context",
    description = "Returns paginated list of processing jobs for this context. Jobs track document processing through MIR (extraction), \
RDF (knowledge graph), and other pipeline stages. Includes status, progress, and error information.",
    params(
        ("context_id" = i32, Path, description = "Context identifier"),
        ContextJobsQuery
    ),
    responses(
        (status = 200, description = "List of jobs defined within the context", body = ContextJobsResponse),
        (status = 404, description = "Context not found", body = ErrorResponse)
    ),
    tag = "Jobs"
)]
pub async fn list_context_jobs(
    State(state): State<Arc<AppState>>,
    Path(context_id): Path<i32>,
    Query(query): Query<ContextJobsQuery>,
) -> Result<impl IntoResponse, GatewayError> {
    let limit = resolve_limit(query.limit)?;
    let cursor = build_cursor(query.cursor_updated_at, query.cursor_job_id)?;

    let records = state
        .job_repo
        .list_by_context(JobListParams {
            context_id,
            limit,
            cursor,
        })
        .await?;

    if records.is_empty() {
        let exists = state.context_repo.get(context_id).await?.is_some();
        if !exists {
            return Err(GatewayError::ContextNotFound(context_id));
        }
    }

    let next_cursor = if (records.len() as i64) == limit {
        records.last().map(|record| ContextJobCursor {
            updated_at: record.updated_at,
            job_id: record.job_id,
        })
    } else {
        None
    };

    let file_map = load_file_map(&state, &records).await?;
    let jobs = records
        .into_iter()
        .map(|record| {
            let file = record.file_id.and_then(|file_id| file_map.get(&file_id));
            ContextJob::from_record(record, file)
        })
        .collect();
    Ok(Json(ContextJobsResponse { jobs, next_cursor }))
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct ContextJobsQuery {
    pub limit: Option<i64>,
    pub cursor_updated_at: Option<DateTime<Utc>>,
    pub cursor_job_id: Option<Uuid>,
}

#[derive(Serialize, ToSchema)]
pub struct ContextJobsResponse {
    jobs: Vec<ContextJob>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<ContextJobCursor>,
}

#[derive(Serialize, ToSchema)]
pub struct ContextJob {
    job_id: Uuid,
    pipeline_id: uuid::Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_id: Option<i64>,
    engine: JobEngine,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<JobKind>,
    job_mode: JobLaunchMode,
    status: JobStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    stage: Option<JobStageResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    progress_pct: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stage_current: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stage_total: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stage_percent: Option<u8>,
    message: Option<String>,
    updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sha256_hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    artifact_uri: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ContextJobCursor {
    updated_at: DateTime<Utc>,
    job_id: Uuid,
}

impl ContextJob {
    fn from_record(record: JobRecord, file: Option<&FileRecord>) -> Self {
        let job_mode = JobLaunchMode::from_kind(record.kind);
        let sha256_hex = file.map(|file| hex::encode(&file.sha256));
        let artifact_uri = file.and_then(|file| file.mir_artifact_uri.clone());
        Self {
            job_id: record.job_id,
            pipeline_id: record.pipeline_id,
            file_id: record.file_id,
            engine: record.engine,
            kind: record.kind,
            job_mode,
            status: record.status,
            stage: record.stage.map(JobStageResponse::from),
            progress_pct: record.progress_pct.map(|pct| pct.clamp(0, 100) as u8),
            stage_current: record
                .stage_progress_current
                .and_then(|value| u32::try_from(value).ok()),
            stage_total: record
                .stage_progress_total
                .and_then(|value| u32::try_from(value).ok()),
            stage_percent: record.stage_progress_pct.map(|pct| pct.clamp(0, 100) as u8),
            message: record.message,
            updated_at: record.updated_at,
            sha256_hex,
            artifact_uri,
        }
    }
}

async fn load_file_map(
    state: &Arc<AppState>,
    records: &[JobRecord],
) -> Result<HashMap<i64, FileRecord>, GatewayError> {
    let mut seen = HashSet::new();
    let mut ids = Vec::new();
    for record in records {
        let Some(file_id) = record.file_id else {
            continue;
        };
        if !seen.insert(file_id) {
            continue;
        }
        ids.push(file_id);
    }

    state
        .file_repo
        .find_many_by_ids(&ids)
        .await
        .map_err(GatewayError::from)
}

fn build_cursor(
    updated_at: Option<DateTime<Utc>>,
    job_id: Option<Uuid>,
) -> Result<Option<JobListCursor>, GatewayError> {
    match (updated_at, job_id) {
        (Some(ts), Some(job_id)) => Ok(Some(JobListCursor {
            updated_at: ts,
            job_id,
        })),
        (None, None) => Ok(None),
        _ => Err(GatewayError::InvalidField {
            field: "cursor".into(),
            message: "provide both cursor_updated_at and cursor_job_id".into(),
        }),
    }
}
