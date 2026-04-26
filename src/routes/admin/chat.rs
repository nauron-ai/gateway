use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use uuid::Uuid;

use crate::{error::GatewayError, state::AppState};
use nauron_contracts::chat::{
    AdminSessionsQuery, ReasoningQuery, ReasoningResponse, SessionDetailResponse, SessionsResponse,
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/chat/sessions", get(list_sessions))
        .route("/chat/sessions/{session_id}", get(get_session))
        .route("/chat/{session_id}/reasoning", get(get_reasoning))
}

#[utoipa::path(
    get,
    path = "/admin/chat/sessions",
    operation_id = "admin_list_chat_sessions",
    summary = "List all chat sessions (admin)",
    description = "Returns paginated list of all chat sessions across all users. Admin only. \
Can filter by context_id or user_id. Bypasses ownership checks.",
    params(AdminSessionsQuery),
    responses(
        (status = 200, description = "List of all chat sessions (admin)", body = SessionsResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Admin"
)]
pub(crate) async fn list_sessions(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AdminSessionsQuery>,
) -> Result<Json<SessionsResponse>, GatewayError> {
    let sessions = state.inferencer_client.list_sessions_admin(&query).await?;

    Ok(Json(sessions))
}

#[utoipa::path(
    get,
    path = "/admin/chat/sessions/{session_id}",
    operation_id = "admin_get_chat_session",
    summary = "Get chat session (admin)",
    description = "Retrieves full session details including all messages. Admin only. Bypasses ownership checks for any session.",
    params(
        ("session_id" = Uuid, Path, description = "Session identifier")
    ),
    responses(
        (status = 200, description = "Session details with messages (admin)", body = SessionDetailResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 404, description = "Session not found", body = crate::error::ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Admin"
)]
pub(crate) async fn get_session(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<SessionDetailResponse>, GatewayError> {
    let session = state
        .inferencer_client
        .get_session_admin(session_id)
        .await?;

    Ok(Json(session))
}

#[utoipa::path(
    get,
    path = "/admin/chat/{session_id}/reasoning",
    operation_id = "admin_get_chat_reasoning",
    summary = "Get reasoning trace (admin)",
    description = "Retrieves internal reasoning trace for a message. Admin only. Shows step-by-step thought process for debugging.",
    params(
        ("session_id" = Uuid, Path, description = "Session identifier"),
        ReasoningQuery
    ),
    responses(
        (status = 200, description = "Reasoning trace for message (admin)", body = ReasoningResponse),
        (status = 400, description = "Missing message_id", body = crate::error::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Admin"
)]
pub(crate) async fn get_reasoning(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<Uuid>,
    Query(query): Query<ReasoningQuery>,
) -> Result<Json<ReasoningResponse>, GatewayError> {
    let reasoning = state
        .inferencer_client
        .get_reasoning_admin(session_id, query.message_id)
        .await?;

    Ok(Json(reasoning))
}
