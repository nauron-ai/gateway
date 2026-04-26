use std::sync::Arc;

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    config::AuthSettings,
    db::users::{UserRecord, UserRole},
    error::GatewayError,
    state::AppState,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
    pub role: UserRole,
}

impl From<UserRecord> for AuthUser {
    fn from(value: UserRecord) -> Self {
        Self {
            id: value.id,
            email: value.email,
            role: value.role,
        }
    }
}

pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default().hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, password_hash: &str) -> Result<bool, GatewayError> {
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|_| GatewayError::Unauthorized("invalid credentials".into()))?;
    let verified = Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok();
    Ok(verified)
}

pub fn generate_token(
    user_id: Uuid,
    settings: &AuthSettings,
) -> Result<(String, DateTime<Utc>), GatewayError> {
    let expires_at = Utc::now()
        .checked_add_signed(Duration::seconds(settings.jwt_ttl_seconds))
        .ok_or_else(|| GatewayError::Unauthorized("could not compute token expiry".into()))?;
    let claims = Claims {
        sub: user_id,
        exp: expires_at.timestamp(),
    };
    let token = jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(settings.jwt_secret.as_bytes()),
    )
    .map_err(|_| GatewayError::Unauthorized("failed to generate token".into()))?;
    Ok((token, expires_at))
}

fn decode_token(token: &str, settings: &AuthSettings) -> Result<Claims, GatewayError> {
    let validation = Validation::new(Algorithm::HS256);
    jsonwebtoken::decode::<Claims>(
        token,
        &DecodingKey::from_secret(settings.jwt_secret.as_bytes()),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|err| match err.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
            GatewayError::Unauthorized("token expired".into())
        }
        _ => GatewayError::Unauthorized("invalid token".into()),
    })
}

fn extract_bearer_token(headers: &HeaderMap) -> Result<String, GatewayError> {
    let header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| GatewayError::Unauthorized("missing bearer token".into()))?;
    let parts: Vec<&str> = header.split_whitespace().collect();
    if parts.len() == 2 && parts[0].eq_ignore_ascii_case("bearer") {
        Ok(parts[1].to_string())
    } else {
        Err(GatewayError::Unauthorized(
            "invalid authorization header".into(),
        ))
    }
}

pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, GatewayError> {
    let token = extract_bearer_token(req.headers())?;
    let claims = decode_token(&token, &state.config.auth)?;

    let user = state
        .user_repo
        .find_by_id(claims.sub)
        .await?
        .ok_or_else(|| GatewayError::Unauthorized("user not found".into()))?;
    if user.blocked {
        return Err(GatewayError::Forbidden("user is blocked".into()));
    }

    req.extensions_mut().insert(AuthUser::from(user));
    Ok(next.run(req).await)
}

pub async fn require_admin(req: Request, next: Next) -> Result<Response, GatewayError> {
    let user = req
        .extensions()
        .get::<AuthUser>()
        .ok_or_else(|| GatewayError::Unauthorized("unauthenticated".into()))?;

    if user.role != UserRole::Admin {
        return Err(GatewayError::Forbidden("admin role required".into()));
    }

    Ok(next.run(req).await)
}
