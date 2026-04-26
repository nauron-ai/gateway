use crate::error::GatewayError;

pub(super) fn resolve_limit(raw: Option<i64>) -> Result<i64, GatewayError> {
    const DEFAULT_LIMIT: i64 = 50;
    const MAX_LIMIT: i64 = 100;
    match raw {
        None => Ok(DEFAULT_LIMIT),
        Some(value) if (1..=MAX_LIMIT).contains(&value) => Ok(value),
        _ => Err(GatewayError::InvalidField {
            field: "limit".into(),
            message: "must be between 1 and 100".into(),
        }),
    }
}
