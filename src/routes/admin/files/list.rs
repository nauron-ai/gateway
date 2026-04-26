use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    db::files::{AdminFileCursor, AdminFileListParams, AdminFileRecord, FileStatus},
    error::GatewayError,
    routes::pagination::resolve_limit,
    state::AppState,
};

#[utoipa::path(
    get,
    path = "/admin/files",
    summary = "List all files (admin)",
    description = "Returns paginated list of all files in the system. Admin only. Can filter by status, context, or SHA256 prefix. \
Shows processing status, job associations, and context attachments.",
    params(AdminFilesQuery),
    responses(
        (status = 200, description = "List all files", body = AdminFilesResponse)
    ),
    tag = "Admin"
)]
pub async fn list_files(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AdminFilesQuery>,
) -> Result<impl IntoResponse, GatewayError> {
    let limit = resolve_limit(query.limit)?;
    let cursor = build_cursor(query.cursor_updated_at, query.cursor_file_id)?;
    let sha_filter = build_sha_filter(&query.sha256_hex)?;

    let records = state
        .file_repo
        .list_files_for_admin(AdminFileListParams {
            limit,
            status: query.status,
            context_id: query.context_id,
            sha256_prefix: sha_filter,
            cursor,
        })
        .await?;

    let next_cursor = if (records.len() as i64) == limit {
        records.last().map(|record| AdminFilesCursor {
            updated_at: record.updated_at,
            file_id: record.file_id,
        })
    } else {
        None
    };

    let files = records
        .into_iter()
        .map(AdminFileEntry::from_record)
        .collect();

    Ok(Json(AdminFilesResponse { files, next_cursor }))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct AdminFilesQuery {
    pub limit: Option<i64>,
    pub cursor_updated_at: Option<DateTime<Utc>>,
    pub cursor_file_id: Option<i64>,
    pub status: Option<FileStatus>,
    pub sha256_hex: Option<String>,
    pub context_id: Option<i32>,
}

#[derive(Serialize, ToSchema)]
pub struct AdminFilesResponse {
    pub files: Vec<AdminFileEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<AdminFilesCursor>,
}

#[derive(Serialize, ToSchema)]
pub struct AdminFileEntry {
    pub file_id: i64,
    pub sha256_hex: String,
    pub size_bytes: i64,
    pub status: FileStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mir_job_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mir_artifact_uri: Option<String>,
    pub contexts_count: i64,
    pub contexts: Vec<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct AdminFilesCursor {
    pub updated_at: DateTime<Utc>,
    pub file_id: i64,
}

impl AdminFileEntry {
    fn from_record(record: AdminFileRecord) -> Self {
        Self {
            file_id: record.file_id,
            sha256_hex: hex::encode(record.sha256),
            size_bytes: record.size_bytes,
            status: record.status,
            mir_job_id: record.mir_job_id,
            mir_artifact_uri: record.mir_artifact_uri,
            contexts_count: record.contexts_count,
            contexts: record.contexts,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

fn build_cursor(
    updated_at: Option<DateTime<Utc>>,
    file_id: Option<i64>,
) -> Result<Option<AdminFileCursor>, GatewayError> {
    match (updated_at, file_id) {
        (Some(ts), Some(id)) if id > 0 => Ok(Some(AdminFileCursor {
            updated_at: ts,
            file_id: id,
        })),
        (None, None) => Ok(None),
        _ => Err(GatewayError::InvalidField {
            field: "cursor".into(),
            message: "provide both cursor_updated_at and cursor_file_id".into(),
        }),
    }
}

fn build_sha_filter(raw: &Option<String>) -> Result<Option<String>, GatewayError> {
    match raw {
        None => Ok(None),
        Some(prefix) if prefix.is_empty() => Ok(None),
        Some(prefix) => {
            if prefix.len() > 64 || !prefix.chars().all(|ch| ch.is_ascii_hexdigit()) {
                return Err(GatewayError::InvalidField {
                    field: "sha256_hex".into(),
                    message: "must be a hex prefix up to 64 chars".into(),
                });
            }
            Ok(Some(format!("{}%", prefix.to_lowercase())))
        }
    }
}
