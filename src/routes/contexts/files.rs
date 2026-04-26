use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::{
        HeaderValue,
        header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};

use crate::{
    auth::AuthUser,
    db::files::ContextFileListParams,
    error::{ErrorResponse, GatewayError},
    routes::download_headers::attachment_disposition,
    state::AppState,
};

use super::utils::ensure_context_owner;
use super::{ContextFileCursor, ContextFileEntry, ContextFilesQuery, ContextFilesResponse, utils};
use crate::routes::pagination::resolve_limit;

#[utoipa::path(
    get,
    path = "/v1/contexts/{context_id}/files",
    summary = "List files in context",
    description = "Returns paginated list of files uploaded to this context. Includes file metadata, processing status, and attachment timestamps.",
    params(
        ("context_id" = i32, Path, description = "Context identifier"),
        ContextFilesQuery
    ),
    responses(
        (status = 200, description = "List of files within the context", body = ContextFilesResponse),
        (status = 404, description = "Context not found", body = ErrorResponse)
    ),
    tag = "Contexts"
)]
pub async fn list_context_files(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthUser>,
    Path(context_id): Path<i32>,
    Query(query): Query<ContextFilesQuery>,
) -> Result<impl IntoResponse, GatewayError> {
    let limit = resolve_limit(query.limit)?;
    let cursor = utils::build_file_cursor(query.cursor_attached_at, query.cursor_id)?;
    let _context = ensure_context_owner(&state, context_id, &user).await?;
    let records = state
        .file_repo
        .list_by_context_with_cursor(ContextFileListParams {
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
        records.last().map(|record| ContextFileCursor {
            attached_at: record.attached_at,
            context_file_id: record.id,
        })
    } else {
        None
    };

    let files = records
        .into_iter()
        .map(ContextFileEntry::from_record)
        .collect();

    Ok(Json(ContextFilesResponse { files, next_cursor }))
}

#[utoipa::path(
    get,
    path = "/v1/contexts/{context_id}/files/{context_file_id}/download",
    summary = "Download original file",
    description = "Downloads the original uploaded file from storage. Returns the file with appropriate Content-Type and Content-Disposition headers.",
    params(
        ("context_id" = i32, Path, description = "Context identifier"),
        ("context_file_id" = i64, Path, description = "Context file identifier"),
    ),
    responses(
        (status = 200, description = "Original uploaded file", content_type = "application/octet-stream", body = String),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Context or file not found", body = ErrorResponse)
    ),
    tag = "Contexts"
)]
pub async fn download_context_file(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthUser>,
    Path((context_id, context_file_id)): Path<(i32, i64)>,
) -> Result<Response, GatewayError> {
    let _context = ensure_context_owner(&state, context_id, &user).await?;

    let record = state
        .file_repo
        .find_context_file_by_id(context_file_id)
        .await?
        .ok_or(GatewayError::FileNotFound(context_file_id))?;

    if record.context_id != context_id {
        return Err(GatewayError::FileNotFound(context_file_id));
    }

    let file = state
        .file_repo
        .find_by_id(record.file_id)
        .await?
        .ok_or(GatewayError::FileNotFound(context_file_id))?;

    let bytes = state
        .storage
        .download(file.storage_bucket.as_str(), file.storage_key.as_str())
        .await?;

    let mut response = Response::new(bytes.into());

    if let Some(ct) = record.media_type.as_deref().or(file.mime.as_deref())
        && let Ok(value) = HeaderValue::from_str(ct)
    {
        response.headers_mut().insert(CONTENT_TYPE, value);
    }
    if !response.headers().contains_key(CONTENT_TYPE) {
        response.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/octet-stream"),
        );
    }

    if let Some(value) = attachment_disposition(&record.original_name) {
        response.headers_mut().insert(CONTENT_DISPOSITION, value);
    }

    Ok(response)
}
