use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::AuthUser, db::users::UserRole, error::GatewayError,
    routes::contexts::ensure_context_write_access, state::AppState,
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateShareRequest {
    pub user_id: Uuid,
    #[serde(default)]
    pub role: Option<UserRole>,
}

#[utoipa::path(
    post,
    path = "/v1/contexts/{context_id}/shares",
    summary = "Share context with user",
    description = "Grants another user access to this context. Specify role: 'viewer' (read-only) or 'user' (read/write). \
Admin role cannot be granted via sharing. Only context owner can share.",
    request_body(content = CreateShareRequest, example = json!({
        "user_id": "550e8400-e29b-41d4-a716-446655440000",
        "role": "viewer"
    })),
    responses(
        (status = 201, description = "Context shared"),
        (status = 403, description = "Forbidden", body = crate::error::ErrorResponse),
        (status = 404, description = "Context not found", body = crate::error::ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Contexts"
)]
pub async fn add_share(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthUser>,
    Path(context_id): Path<i32>,
    Json(payload): Json<CreateShareRequest>,
) -> Result<(StatusCode, ()), GatewayError> {
    let context = ensure_context_write_access(&state, context_id, &user).await?;
    let role = payload.role.unwrap_or(UserRole::Viewer);

    if role == UserRole::Admin {
        return Err(GatewayError::Forbidden("cannot grant admin role".into()));
    }

    state
        .share_repo
        .create_share(context.id, payload.user_id, role)
        .await?;

    Ok((StatusCode::CREATED, ()))
}

#[utoipa::path(
    delete,
    path = "/v1/contexts/{context_id}/shares/{user_id}",
    summary = "Remove context share",
    description = "Revokes a user's access to this context. Only context owner can remove shares.",
    params(
        ("context_id" = i32, Path, description = "Context identifier"),
        ("user_id" = Uuid, Path, description = "User identifier")
    ),
    responses(
        (status = 204, description = "Share removed"),
        (status = 403, description = "Forbidden", body = crate::error::ErrorResponse),
        (status = 404, description = "Context not found", body = crate::error::ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Contexts"
)]
pub async fn remove_share(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthUser>,
    Path((context_id, shared_user_id)): Path<(i32, Uuid)>,
) -> Result<(StatusCode, ()), GatewayError> {
    let context = ensure_context_write_access(&state, context_id, &user).await?;
    state
        .share_repo
        .remove_share(context.id, shared_user_id)
        .await?;
    Ok((StatusCode::NO_CONTENT, ()))
}
