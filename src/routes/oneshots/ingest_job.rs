use axum::http::HeaderMap;
use serde_json::Value;
use uuid::Uuid;

use crate::error::GatewayError;
use crate::idempotency::build_deterministic_job_id;

pub(super) fn default_ingest_type_spec() -> Value {
    Value::String("string".to_string())
}

pub(super) fn validate_ingest_type_spec(type_spec: Option<&Value>) -> Result<(), GatewayError> {
    let Some(type_spec) = type_spec else {
        return Ok(());
    };
    match type_spec {
        Value::String(_) | Value::Object(_) => Ok(()),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::Array(_) => {
            Err(GatewayError::InvalidField {
                field: "schema.type".to_string(),
                message: "schema field type must be a string or object".to_string(),
            })
        }
    }
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub(crate) struct CreateIngestJobResponse {
    pub job_id: Uuid,
    pub job_status_url: String,
}

impl CreateIngestJobResponse {
    pub(super) fn new(job_id: Uuid) -> Self {
        Self {
            job_status_url: format!("/v1/jobs/{job_id}"),
            job_id,
        }
    }
}

pub(super) fn build_ingest_job_id(context_id: i32, user_id: &str, headers: &HeaderMap) -> Uuid {
    let key = headers
        .get(super::IDEMPOTENCY_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match key {
        Some(key) => {
            build_deterministic_job_id(super::INGEST_IDEMPOTENCY_PREFIX, context_id, user_id, key)
        }
        None => Uuid::new_v4(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::validate_ingest_type_spec;

    #[test]
    fn rejects_invalid_type_spec() {
        let value = json!(["string"]);
        let result = validate_ingest_type_spec(Some(&value));

        assert!(result.is_err());
    }
}
