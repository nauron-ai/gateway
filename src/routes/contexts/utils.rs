use chrono::{DateTime, Utc};

use crate::{
    auth::AuthUser,
    db::{
        contexts::{ContextListCursor, ContextRecord},
        files::ContextFileListCursor,
        users::UserRole,
    },
    error::GatewayError,
    state::AppState,
};

pub fn build_file_cursor(
    attached_at: Option<DateTime<Utc>>,
    context_file_id: Option<i64>,
) -> Result<Option<ContextFileListCursor>, GatewayError> {
    match (attached_at, context_file_id) {
        (Some(ts), Some(id)) if id > 0 => Ok(Some(ContextFileListCursor {
            attached_at: ts,
            context_file_id: id,
        })),
        (None, None) => Ok(None),
        _ => Err(GatewayError::InvalidField {
            field: "cursor".into(),
            message: "provide both cursor_attached_at and cursor_id".into(),
        }),
    }
}

pub fn build_context_cursor(
    created_at: Option<DateTime<Utc>>,
    context_id: Option<i32>,
) -> Result<Option<ContextListCursor>, GatewayError> {
    match (created_at, context_id) {
        (Some(ts), Some(id)) if id > 0 => Ok(Some(ContextListCursor { created_at: ts, id })),
        (None, None) => Ok(None),
        _ => Err(GatewayError::InvalidField {
            field: "cursor".into(),
            message: "provide both cursor_created_at and cursor_id".into(),
        }),
    }
}

pub async fn ensure_context_owner(
    state: &AppState,
    context_id: i32,
    user: &AuthUser,
) -> Result<ContextRecord, GatewayError> {
    let context = state
        .context_repo
        .get(context_id)
        .await?
        .ok_or(GatewayError::ContextNotFound(context_id))?;

    let owner_id = context.owner_id;
    let is_admin = user.role == UserRole::Admin;
    let is_owner = owner_id.map(|id| id == user.id).unwrap_or(false);
    let is_shared = state.share_repo.is_shared_with(context_id, user.id).await?;

    if is_admin || is_owner || is_shared {
        return Ok(context);
    }

    Err(GatewayError::Forbidden(
        "context does not belong to the authenticated user".into(),
    ))
}

pub async fn ensure_context_write_access(
    state: &AppState,
    context_id: i32,
    user: &AuthUser,
) -> Result<ContextRecord, GatewayError> {
    let context = state
        .context_repo
        .get(context_id)
        .await?
        .ok_or(GatewayError::ContextNotFound(context_id))?;

    let owner_id = context.owner_id;
    let is_admin = user.role == UserRole::Admin;
    let is_owner = owner_id.map(|id| id == user.id).unwrap_or(false);

    if is_admin || is_owner {
        return Ok(context);
    }

    Err(GatewayError::Forbidden(
        "write access requires context owner or admin".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_cursor_when_both_fields_present() {
        let ts = Utc::now();
        let cursor = build_file_cursor(Some(ts), Some(42))
            .unwrap()
            .expect("cursor expected");
        assert_eq!(cursor.context_file_id, 42);
    }

    #[test]
    fn errors_when_cursor_fields_missing() {
        let ts = Utc::now();
        let err = build_file_cursor(Some(ts), None).unwrap_err();
        assert!(matches!(err, GatewayError::InvalidField { .. }));
    }

    #[test]
    fn builds_context_cursor_when_both_fields_present() {
        let ts = Utc::now();
        let cursor = build_context_cursor(Some(ts), Some(7))
            .unwrap()
            .expect("cursor expected");
        assert_eq!(cursor.id, 7);
    }

    #[test]
    fn errors_when_context_cursor_fields_missing() {
        let ts = Utc::now();
        let err = build_context_cursor(Some(ts), None).unwrap_err();
        assert!(matches!(err, GatewayError::InvalidField { .. }));
    }
}
