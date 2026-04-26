use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
use chrono::{DateTime, Utc};
use utoipa::ToSchema;

use crate::{
    db::files::{ContextFileRecord, FileOrigin, FileStatus},
    error::GatewayError,
    state::AppState,
};

#[utoipa::path(
    get,
    path = "/admin/contexts/{context_id}/files",
    operation_id = "admin_list_context_files",
    summary = "List context files (admin)",
    description = "Returns all files attached to a context with full details. Admin only. \
Bypasses ownership checks for administrative access.",
    params(("context_id" = i32, Path, description = "Context identifier")),
    responses(
        (status = 200, description = "Context files", body = ContextFilesResponse),
        (status = 404, description = "Context not found", body = crate::error::ErrorResponse)
    ),
    tag = "Admin"
)]
pub async fn list_context_files(
    Path(context_id): Path<i32>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ContextFilesResponse>, GatewayError> {
    let records = state.file_repo.list_by_context(context_id).await?;
    if records.is_empty() && state.context_repo.get(context_id).await?.is_none() {
        return Err(GatewayError::ContextNotFound(context_id));
    }
    let files = records
        .into_iter()
        .map(ContextFileEntry::from_record)
        .collect();
    Ok(Json(ContextFilesResponse { context_id, files }))
}

#[derive(serde::Serialize, ToSchema)]
pub struct ContextFilesResponse {
    pub context_id: i32,
    pub files: Vec<ContextFileEntry>,
}

#[derive(serde::Serialize, ToSchema)]
pub struct ContextFileEntry {
    pub context_file_id: i64,
    pub file_id: i64,
    pub pipeline_id: uuid::Uuid,
    pub origin: FileOrigin,
    pub original_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    pub attached_at: DateTime<Utc>,
    pub status: FileStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256_hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mir_artifact_uri: Option<String>,
}

impl ContextFileEntry {
    fn from_record(record: ContextFileRecord) -> Self {
        Self {
            context_file_id: record.id,
            file_id: record.file_id,
            pipeline_id: record.pipeline_id,
            origin: record.origin,
            original_name: record.original_name,
            original_path: record.original_path,
            media_type: record.media_type,
            attached_at: record.attached_at,
            status: record.file_status,
            sha256_hex: Some(hex::encode(record.file_sha256)),
            mir_artifact_uri: record.mir_artifact_uri,
        }
    }
}
