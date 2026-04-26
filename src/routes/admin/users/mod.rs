mod types;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    auth,
    db::users::{CreateUserParams, UpdateUserParams, UserListCursor, UserListParams, UserRole},
    error::GatewayError,
    routes::pagination::resolve_limit,
    state::AppState,
};

pub use types::{
    CreateUserRequest, UpdateUserRequest, UserCursor, UserResponse, UsersQuery, UsersResponse,
};

#[utoipa::path(
    get,
    path = "/admin/users",
    summary = "List all users (admin)",
    description = "Paginated list of all users. Admin only. Includes roles, blocked status, timestamps.",
    params(UsersQuery),
    responses((status = 200, description = "List users", body = UsersResponse)),
    security(("bearerAuth" = [])),
    tag = "Users"
)]
pub async fn list_users(
    State(state): State<Arc<AppState>>,
    Query(query): Query<UsersQuery>,
) -> Result<impl IntoResponse, GatewayError> {
    let limit = resolve_limit(query.limit)?;
    let cursor = build_cursor(query.cursor_created_at, query.cursor_id)?;
    let records = state
        .user_repo
        .list(UserListParams { limit, cursor })
        .await?;
    let next_cursor = if (records.len() as i64) == limit {
        records.last().map(|r| UserCursor {
            created_at: r.created_at,
            user_id: r.id,
        })
    } else {
        None
    };
    let users = records.into_iter().map(UserResponse::from).collect();
    Ok(Json(UsersResponse { users, next_cursor }))
}

#[utoipa::path(
    post,
    path = "/admin/users",
    summary = "Create user (admin)",
    description = "Creates user with email/password. Admin only. Role defaults to 'user'.",
    request_body(content = CreateUserRequest, example = json!({"email": "user@example.com", "password": "pass123", "role": "user"})),
    responses(
        (status = 201, description = "User created", body = UserResponse),
        (status = 409, description = "Email exists", body = crate::error::ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "Users"
)]
pub async fn create_user(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<impl IntoResponse, GatewayError> {
    let role = payload.role.unwrap_or(UserRole::User);
    let password_hash =
        auth::hash_password(&payload.password).map_err(|e| GatewayError::InvalidField {
            field: "password".into(),
            message: e.to_string(),
        })?;
    let created = state
        .user_repo
        .create(CreateUserParams {
            email: &payload.email,
            password_hash: &password_hash,
            role,
            blocked: false,
        })
        .await
        .map_err(map_unique_email)?;
    Ok((StatusCode::CREATED, Json(UserResponse::from(created))))
}

#[utoipa::path(
    patch,
    path = "/admin/users/{user_id}",
    summary = "Update user (admin)",
    description = "Updates email, password, role, or blocked status. Admin only. At least one field required.",
    params(("user_id" = Uuid, Path, description = "User identifier")),
    request_body(content = UpdateUserRequest, example = json!({"role": "admin", "blocked": false})),
    responses(
        (status = 200, description = "User updated", body = UserResponse),
        (status = 404, description = "User not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Email exists", body = crate::error::ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "Users"
)]
pub async fn update_user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<impl IntoResponse, GatewayError> {
    if payload.email.is_none()
        && payload.password.is_none()
        && payload.role.is_none()
        && payload.blocked.is_none()
    {
        return Err(GatewayError::InvalidField {
            field: "body".into(),
            message: "provide at least one field".into(),
        });
    }
    let password_hash = match payload.password.as_deref() {
        Some(p) => Some(
            auth::hash_password(p).map_err(|e| GatewayError::InvalidField {
                field: "password".into(),
                message: e.to_string(),
            })?,
        ),
        None => None,
    };
    let updated = state
        .user_repo
        .update(
            user_id,
            UpdateUserParams {
                email: payload.email.as_deref(),
                password_hash: password_hash.as_deref(),
                role: payload.role,
                blocked: payload.blocked,
            },
        )
        .await;
    match updated {
        Ok(user) => Ok(Json(UserResponse::from(user))),
        Err(sqlx::Error::RowNotFound) => Err(GatewayError::UserNotFound(user_id)),
        Err(err) => Err(map_unique_email(err)),
    }
}

fn build_cursor(
    created_at: Option<DateTime<Utc>>,
    user_id: Option<Uuid>,
) -> Result<Option<UserListCursor>, GatewayError> {
    match (created_at, user_id) {
        (Some(ts), Some(id)) => Ok(Some(UserListCursor {
            created_at: ts,
            user_id: id,
        })),
        (None, None) => Ok(None),
        _ => Err(GatewayError::InvalidField {
            field: "cursor".into(),
            message: "provide both cursor_created_at and cursor_id".into(),
        }),
    }
}

fn map_unique_email(err: sqlx::Error) -> GatewayError {
    if let sqlx::Error::Database(db_err) = &err
        && db_err.constraint() == Some("users_email_key")
    {
        return GatewayError::Conflict("email already exists".into());
    }
    GatewayError::from(err)
}
