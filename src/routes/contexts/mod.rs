use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::{
    auth::AuthUser,
    db::contexts::{ContextListParams, UpdateContextParams},
    error::{ErrorResponse, GatewayError},
    state::AppState,
};

use super::pagination::resolve_limit;
use utils::build_context_cursor;

pub mod conditions;
mod files;
pub(crate) mod graph;
pub mod jobs;
pub mod stats;
mod types;
mod utils;
pub use conditions::__path_create_evaluate_conditions_job;
pub use conditions::create_evaluate_conditions_job;
pub use files::{
    __path_download_context_file, __path_list_context_files, download_context_file,
    list_context_files,
};
pub use graph::{get_context_entities, get_context_graph};
pub use types::{
    ContextCursor, ContextFileCursor, ContextFileEntry, ContextFilesQuery, ContextFilesResponse,
    ContextResponse, ContextsQuery, ContextsResponse, CreateContextRequest, UpdateContextRequest,
};
pub(crate) use utils::{ensure_context_owner, ensure_context_write_access};

#[utoipa::path(
    get,
    path = "/v1/contexts",
    summary = "List user's contexts",
    description = "Returns paginated list of contexts owned by or shared with the authenticated user. \
Contexts are document collections used for RAG-based chat and condition evaluation.",
    params(ContextsQuery),
    responses(
        (status = 200, description = "List of contexts", body = ContextsResponse),
        (status = 400, description = "Invalid pagination params", body = ErrorResponse)
    ),
    tag = "Contexts"
)]
pub async fn list_contexts(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthUser>,
    Query(query): Query<ContextsQuery>,
) -> Result<impl IntoResponse, GatewayError> {
    let limit = resolve_limit(query.limit)?;
    let cursor = build_context_cursor(query.cursor_created_at, query.cursor_id)?;
    let records = state
        .context_repo
        .list(ContextListParams {
            cursor,
            limit,
            owner_id: Some(user.id),
        })
        .await?;

    let next_cursor = if (records.len() as i64) == limit {
        records.last().map(|record| ContextCursor {
            created_at: record.created_at,
            id: record.id,
        })
    } else {
        None
    };

    let contexts = records.into_iter().map(ContextResponse::from).collect();
    Ok(Json(ContextsResponse {
        contexts,
        next_cursor,
    }))
}

#[utoipa::path(
    post,
    path = "/v1/contexts",
    summary = "Create a new context",
    description = "Creates a new document context for the authenticated user. \
Context mode determines processing pipeline: 'rdf' extracts knowledge graph, 'emb' uses pure embeddings, 'lpg' uses labeled property graph.",
    request_body(content = CreateContextRequest, example = json!({
        "mode": "rdf"
    })),
    responses(
        (status = 201, description = "Context created", body = ContextResponse),
        (status = 502, description = "Dependency communication failure", body = ErrorResponse)
    ),
    tag = "Contexts"
)]
pub async fn create_context(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthUser>,
    Json(payload): Json<CreateContextRequest>,
) -> Result<impl IntoResponse, GatewayError> {
    if user.role == crate::db::users::UserRole::Viewer {
        return Err(GatewayError::Forbidden(
            "viewer role cannot create contexts".into(),
        ));
    }
    let record = state
        .context_repo
        .create(
            Some(user.id),
            payload
                .mode
                .unwrap_or(crate::db::contexts::ContextMode::Rdf),
        )
        .await?;
    Ok((StatusCode::CREATED, Json(ContextResponse::from(record))))
}

#[utoipa::path(
    patch,
    path = "/v1/contexts/{context_id}",
    summary = "Update context metadata",
    description = "Updates context title, description, or processing mode. Only the context owner or users with write access can update.",
    params(
        ("context_id" = i32, Path, description = "Context identifier")
    ),
    request_body(content = UpdateContextRequest, example = json!({
        "title": "Q4 2024 Financial Reports",
        "description": "Quarterly financial statements and audit reports"
    })),
    responses(
        (status = 200, description = "Context updated", body = ContextResponse),
        (status = 404, description = "Context not found", body = ErrorResponse)
    ),
    tag = "Contexts"
)]
pub async fn update_context(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthUser>,
    Path(context_id): Path<i32>,
    Json(payload): Json<UpdateContextRequest>,
) -> Result<impl IntoResponse, GatewayError> {
    let context = ensure_context_write_access(&state, context_id, &user).await?;

    let record = state
        .context_repo
        .update(
            context.id,
            UpdateContextParams {
                title: payload.title,
                description: payload.description,
                mode: payload.mode,
            },
        )
        .await?
        .ok_or(GatewayError::ContextNotFound(context_id))?;

    Ok(Json(ContextResponse::from(record)))
}

#[utoipa::path(
    delete,
    path = "/v1/contexts/{context_id}",
    summary = "Delete a context",
    description = "Permanently deletes a context and all associated data including files, jobs, and knowledge graph. This action cannot be undone.",
    params(
        ("context_id" = i32, Path, description = "Context identifier")
    ),
    responses(
        (status = 204, description = "Context deleted"),
        (status = 404, description = "Context not found", body = ErrorResponse)
    ),
    tag = "Contexts"
)]
pub async fn delete_context(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthUser>,
    Path(context_id): Path<i32>,
) -> Result<impl IntoResponse, GatewayError> {
    let context = ensure_context_write_access(&state, context_id, &user).await?;

    let deleted = state.context_repo.delete(context.id).await?;
    if deleted {
        Ok((StatusCode::NO_CONTENT, ()))
    } else {
        Err(GatewayError::ContextNotFound(context_id))
    }
}
