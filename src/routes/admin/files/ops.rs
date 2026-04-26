use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
use chrono::{DateTime, Utc};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    db::{
        files::{ContextFileRecord, ContextPipelineRef, FileOrigin, FileRecord, FileStatus},
        jobs::JobStatus,
    },
    error::GatewayError,
    job_mode::JobLaunchMode,
    routes::load::job_actions,
    state::AppState,
};

#[utoipa::path(
    get,
    path = "/admin/files/{file_id}",
    summary = "Get file details (admin)",
    description = "Returns detailed information about a file including storage location, processing status, \
MIR artifacts, and all context attachments. Admin only.",
    params(("file_id" = i64, Path, description = "File identifier")),
    responses(
        (status = 200, description = "File details", body = FileDetailsResponse),
        (status = 404, description = "File not found", body = crate::error::ErrorResponse)
    ),
    tag = "Admin"
)]
pub async fn file_details(
    Path(file_id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<FileDetailsResponse>, GatewayError> {
    let file = state
        .file_repo
        .find_by_id(file_id)
        .await?
        .ok_or(GatewayError::FileNotFound(file_id))?;
    let contexts = state
        .file_repo
        .list_context_records_by_file(file_id)
        .await?
        .into_iter()
        .map(FileContextAttachment::from_record)
        .collect();
    Ok(Json(FileDetailsResponse::from_record(file, contexts)))
}

#[utoipa::path(
    post,
    path = "/admin/files/{file_id}/retry",
    summary = "Retry file processing (admin)",
    description = "Resets file to pending state and starts a new MIR processing job. Admin only. \
Returns 409 if file is already being processed by another job.",
    params(("file_id" = i64, Path, description = "File identifier")),
    responses(
        (status = 200, description = "Retry started", body = RetryResponse),
        (status = 404, description = "File not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Job already in progress", body = crate::error::ErrorResponse)
    ),
    tag = "Admin"
)]
pub async fn retry_file(
    Path(file_id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<RetryResponse>, GatewayError> {
    let file = state
        .file_repo
        .find_by_id(file_id)
        .await?
        .ok_or(GatewayError::FileNotFound(file_id))?;

    let contexts = state.file_repo.list_context_ids_by_file(file_id).await?;
    let ContextPipelineRef {
        context_id,
        pipeline_id,
    } = contexts.first().cloned().ok_or(GatewayError::Conflict(
        "file is not attached to any context".into(),
    ))?;

    if let Some(job_id) = file.mir_job_id.as_ref() {
        let in_flight = state
            .job_repo
            .get(*job_id)
            .await?
            .map(|job| matches!(job.status, JobStatus::Pending | JobStatus::InProgress))
            .unwrap_or(false);
        if in_flight {
            return Err(GatewayError::Conflict(format!(
                "file {file_id} already processing via job {job_id}"
            )));
        }
    }

    let pending_file = state.file_repo.reset_pending(file_id).await?;
    let (job_id, _updated) =
        job_actions::start_mir_job(&state, &pending_file, context_id, pipeline_id, None, false)
            .await?;

    Ok(Json(RetryResponse {
        file_id,
        context_id,
        pipeline_id,
        job_id,
        job_mode: JobLaunchMode::Started,
    }))
}

#[derive(serde::Serialize, ToSchema)]
pub struct RetryResponse {
    pub file_id: i64,
    pub context_id: i32,
    pub pipeline_id: uuid::Uuid,
    pub job_id: Uuid,
    pub job_mode: JobLaunchMode,
}

#[derive(serde::Serialize, ToSchema)]
pub struct FileDetailsResponse {
    pub file_id: i64,
    pub sha256_hex: String,
    pub size_bytes: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    pub storage_bucket: String,
    pub storage_key: String,
    pub status: FileStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mir_job_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mir_artifact_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mir_artifact_sha256_hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mir_processed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub contexts: Vec<FileContextAttachment>,
}

#[derive(serde::Serialize, ToSchema)]
pub struct FileContextAttachment {
    pub context_file_id: i64,
    pub context_id: i32,
    pub pipeline_id: uuid::Uuid,
    pub origin: FileOrigin,
    pub original_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    pub attached_at: DateTime<Utc>,
}

impl FileContextAttachment {
    fn from_record(record: ContextFileRecord) -> Self {
        Self {
            context_file_id: record.id,
            context_id: record.context_id,
            pipeline_id: record.pipeline_id,
            origin: record.origin,
            original_name: record.original_name,
            original_path: record.original_path,
            media_type: record.media_type,
            attached_at: record.attached_at,
        }
    }
}

impl FileDetailsResponse {
    fn from_record(record: FileRecord, contexts: Vec<FileContextAttachment>) -> Self {
        Self {
            file_id: record.id,
            sha256_hex: hex::encode(record.sha256),
            size_bytes: record.size_bytes,
            mime: record.mime,
            storage_bucket: record.storage_bucket,
            storage_key: record.storage_key,
            status: record.status,
            mir_job_id: record.mir_job_id,
            mir_artifact_uri: record.mir_artifact_uri,
            mir_artifact_sha256_hex: record.mir_artifact_sha256.map(hex::encode),
            mir_processed_at: record.mir_processed_at,
            created_at: record.created_at,
            updated_at: record.updated_at,
            contexts,
        }
    }
}
