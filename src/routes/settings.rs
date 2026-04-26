use std::sync::Arc;

use axum::extract::State;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    auth::AuthUser,
    db::settings::{ChatMode, UpdateUserSettingsParams, UserTheme},
    error::GatewayError,
    state::AppState,
};

#[derive(Debug, Serialize, ToSchema, Default)]
pub struct UserSettingsResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_chat_mode: Option<ChatMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<UserTheme>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUserSettingsRequest {
    #[serde(default)]
    pub default_chat_mode: Option<ChatMode>,
    #[serde(default)]
    pub default_k: Option<i32>,
    #[serde(default)]
    pub default_lang: Option<String>,
    #[serde(default)]
    pub theme: Option<UserTheme>,
}

impl From<crate::db::settings::UserSettings> for UserSettingsResponse {
    fn from(value: crate::db::settings::UserSettings) -> Self {
        let _ = (value.user_id, value.created_at, value.updated_at);
        Self {
            default_chat_mode: value.default_chat_mode,
            default_k: value.default_k,
            default_lang: value.default_lang,
            theme: value.theme,
        }
    }
}

#[utoipa::path(
    get,
    path = "/v1/settings",
    summary = "Get user settings",
    description = "Returns current user's preferences including default chat mode, search parameters (k), language, and UI theme.",
    responses(
        (status = 200, description = "Current user settings", body = UserSettingsResponse)
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Settings"
)]
pub async fn get_settings(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthUser>,
) -> Result<Json<UserSettingsResponse>, GatewayError> {
    let settings = state
        .settings_repo
        .get(user.id)
        .await?
        .map(UserSettingsResponse::from)
        .unwrap_or_default();

    Ok(Json(settings))
}

#[utoipa::path(
    patch,
    path = "/v1/settings",
    summary = "Update user settings",
    description = "Updates user preferences. All fields are optional - only provided fields are updated. \
Settings include: default_chat_mode, default_k (search results count), default_lang, theme.",
    request_body(content = UpdateUserSettingsRequest, example = json!({
        "default_chat_mode": "rdf-emb",
        "default_k": 15,
        "default_lang": "pl",
        "theme": "dark"
    })),
    responses(
        (status = 200, description = "Updated settings", body = UserSettingsResponse),
        (status = 400, description = "Invalid settings", body = crate::error::ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Settings"
)]
pub async fn update_settings(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthUser>,
    Json(payload): Json<UpdateUserSettingsRequest>,
) -> Result<Json<UserSettingsResponse>, GatewayError> {
    if let Some(k) = payload.default_k
        && k <= 0
    {
        return Err(GatewayError::InvalidField {
            field: "default_k".into(),
            message: "must be greater than zero".into(),
        });
    }

    let params = UpdateUserSettingsParams {
        default_chat_mode: payload.default_chat_mode,
        default_k: payload.default_k,
        default_lang: payload.default_lang,
        theme: payload.theme,
    };

    let updated = state.settings_repo.upsert(user.id, params).await?;

    Ok(Json(UserSettingsResponse::from(updated)))
}
