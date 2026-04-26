use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    response::Response,
    routing::{get, post},
};
use futures_util::StreamExt;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::{error::GatewayError, state::AppState};
use nauron_contracts::chat::ChatMessageRequest;
use nauron_contracts::chat::{
    ReasoningQuery, ReasoningResponse, SessionDetailResponse, SessionsQuery, SessionsResponse,
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/emb", post(chat_emb))
        .route("/rdf-emb", post(chat_rdf_emb))
        .route("/bn", post(chat_bn))
        .route("/sessions", get(list_sessions))
        .route("/sessions/{session_id}", get(get_session))
        .route("/{session_id}/reasoning", get(get_reasoning))
}

#[utoipa::path(
    post,
    path = "/v1/chat/emb",
    summary = "Chat with embedding-based RAG",
    description = "Chat using vector embedding search. Returns SSE stream. Uses semantic similarity for context retrieval.",
    request_body(content = ChatMessageRequest, example = json!({"context_id": 1, "message": "What are the key findings?", "k": 10})),
    responses(
        (status = 200, description = "SSE stream with embeddings chat", content_type = "text/event-stream", body = nauron_contracts::chat::SseExample),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Chat"
)]
async fn chat_emb(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthUser>,
    Json(request): Json<ChatMessageRequest>,
) -> Result<Response, GatewayError> {
    let user_id = user.id.to_string();

    let response = state.inferencer_client.chat_emb(&request, &user_id).await?;

    stream_response(response).await
}

#[utoipa::path(
    post,
    path = "/v1/chat/rdf-emb",
    summary = "Chat with RDF + embedding hybrid RAG",
    description = "Hybrid retrieval combining RDF knowledge graph with vector embeddings. Returns SSE stream.",
    request_body(content = ChatMessageRequest, example = json!({"context_id": 1, "message": "Who are the stakeholders?", "k": 15})),
    responses(
        (status = 200, description = "SSE stream with RDF embeddings chat", content_type = "text/event-stream", body = nauron_contracts::chat::SseExample),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Chat"
)]
async fn chat_rdf_emb(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthUser>,
    Json(request): Json<ChatMessageRequest>,
) -> Result<Response, GatewayError> {
    let user_id = user.id.to_string();

    let response = state
        .inferencer_client
        .chat_rdf_emb(&request, &user_id)
        .await?;

    stream_response(response).await
}

#[utoipa::path(
    post,
    path = "/v1/chat/bn",
    summary = "Agentic chat with Bayesian Network reasoning",
    description = "Agentic chat with BN reasoning, RDF graph and embeddings. Multi-step reasoning with tools. Returns SSE.",
    request_body(content = ChatMessageRequest, example = json!({"context_id": 1, "message": "Analyze risk factors", "k": 20})),
    responses(
        (status = 200, description = "SSE stream with BN+RDF+embeddings agentic chat", content_type = "text/event-stream", body = nauron_contracts::chat::SseExample),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Chat"
)]
async fn chat_bn(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthUser>,
    Json(request): Json<ChatMessageRequest>,
) -> Result<Response, GatewayError> {
    let user_id = user.id.to_string();

    let response = state.inferencer_client.chat_bn(&request, &user_id).await?;

    stream_response(response).await
}

async fn stream_response(response: reqwest::Response) -> Result<Response, GatewayError> {
    let status = response.status();
    let headers = response.headers().clone();

    let stream = response.bytes_stream();
    let body =
        axum::body::Body::from_stream(stream.map(|result| result.map_err(std::io::Error::other)));

    let mut axum_response = Response::new(body);
    *axum_response.status_mut() = status;

    for (key, value) in headers.iter() {
        if let (Ok(header_name), Ok(header_value)) = (
            axum::http::HeaderName::try_from(key.as_str()),
            axum::http::HeaderValue::try_from(value.as_bytes()),
        ) {
            axum_response
                .headers_mut()
                .insert(header_name, header_value);
        }
    }

    Ok(axum_response)
}

#[utoipa::path(
    get,
    path = "/v1/chat/sessions",
    summary = "List user's chat sessions",
    description = "Paginated list of user's sessions ordered by last update. Cursor-based pagination.",
    params(SessionsQuery),
    responses(
        (status = 200, description = "List of chat sessions", body = SessionsResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Chat"
)]
async fn list_sessions(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthUser>,
    Query(query): Query<SessionsQuery>,
) -> Result<Json<SessionsResponse>, GatewayError> {
    let user_id = user.id.to_string();

    let sessions = state
        .inferencer_client
        .list_sessions(&user_id, &query)
        .await?;

    Ok(Json(sessions))
}

#[utoipa::path(
    get,
    path = "/v1/chat/sessions/{session_id}",
    summary = "Get chat session details",
    description = "Full session details with all messages. Only accessible by session owner.",
    params(
        ("session_id" = Uuid, Path, description = "Session identifier")
    ),
    responses(
        (status = 200, description = "Session details with messages", body = SessionDetailResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Chat"
)]
async fn get_session(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthUser>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<SessionDetailResponse>, GatewayError> {
    let user_id = user.id.to_string();

    let session = state
        .inferencer_client
        .get_session(&user_id, session_id)
        .await?;

    Ok(Json(session))
}

#[utoipa::path(
    get,
    path = "/v1/chat/{session_id}/reasoning",
    summary = "Get reasoning trace for a message",
    description = "Internal reasoning trace showing thought process, tool calls, and decision rationale.",
    params(
        ("session_id" = Uuid, Path, description = "Session identifier"),
        ReasoningQuery
    ),
    responses(
        (status = 200, description = "Reasoning trace for message", body = ReasoningResponse),
        (status = 400, description = "Missing message_id", body = crate::error::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Chat"
)]
async fn get_reasoning(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthUser>,
    Path(session_id): Path<Uuid>,
    Query(query): Query<ReasoningQuery>,
) -> Result<Json<ReasoningResponse>, GatewayError> {
    let user_id = user.id.to_string();

    let reasoning = state
        .inferencer_client
        .get_reasoning(&user_id, session_id, query.message_id)
        .await?;

    Ok(Json(reasoning))
}
