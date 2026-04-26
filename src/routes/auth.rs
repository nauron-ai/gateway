use std::sync::Arc;

use axum::{Json, extract::State, response::IntoResponse};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::routes::admin::users::UserResponse;
use crate::{auth, error::GatewayError, state::AppState};

#[utoipa::path(
    post,
    path = "/auth/login",
    summary = "Authenticate user",
    description = "Authenticates user with email and password. Returns JWT bearer token for API authorization. \
Token must be included in Authorization header for protected endpoints.",
    request_body(content = LoginRequest, example = json!({
        "email": "user@example.com",
        "password": "secretpassword123"
    })),
    responses(
        (status = 200, description = "Login success", body = LoginResponse),
        (status = 401, description = "Invalid credentials", body = crate::error::ErrorResponse)
    ),
    security([]),
    tag = "Auth"
)]
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, GatewayError> {
    let user = state
        .user_repo
        .find_by_email(&payload.email)
        .await?
        .ok_or_else(|| GatewayError::Unauthorized("invalid credentials".into()))?;

    if user.blocked {
        return Err(GatewayError::Forbidden("user is blocked".into()));
    }

    let valid = auth::verify_password(&payload.password, &user.password_hash)?;
    if !valid {
        return Err(GatewayError::Unauthorized("invalid credentials".into()));
    }

    let (token, expires_at) = auth::generate_token(user.id, &state.config.auth)?;

    Ok(Json(LoginResponse {
        token,
        token_type: "bearer".into(),
        expires_at,
        user: UserResponse::from(user),
    }))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    pub token: String,
    pub token_type: String,
    pub expires_at: DateTime<Utc>,
    pub user: UserResponse,
}
