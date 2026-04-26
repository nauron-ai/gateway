use axum::Json;
use axum::extract::multipart::MultipartError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::tracker::TrackerError;

#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    #[error("multipart error: {0}")]
    Multipart(#[from] MultipartError),
    #[error("file field is required")]
    MissingFile,
    #[error("invalid field {field}: {message}")]
    InvalidField { field: String, message: String },
    #[error("storage error: {0}")]
    Storage(Box<crate::storage::StorageError>),
    #[error("kafka error: {0}")]
    Kafka(Box<crate::kafka::KafkaError>),
    #[error("context {0} not found")]
    ContextNotFound(i32),
    #[error("database error: {0}")]
    Database(Box<sqlx::Error>),
    #[error("tracker error: {0}")]
    Tracker(Box<TrackerError>),
    #[error("serialization error: {0}")]
    Serialization(Box<serde_json::Error>),
    #[error("archive error: {0}")]
    Archive(String),
    #[error("file {0} not found")]
    FileNotFound(i64),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("job {0} not found")]
    NotFound(String),
    #[error("document {0} not found")]
    DocumentNotFound(Uuid),
    #[error("job {0} has no completed result yet")]
    ResultUnavailable(String),
    #[error("document {0} has no downloadable result yet")]
    DocumentResultUnavailable(Uuid),
    #[error("job {job_id} has no downloadable artifacts")]
    ArtifactMissing { job_id: String },
    #[error("inferencer error: {0}")]
    Inferencer(Box<crate::inferencer::InferencerClientError>),
    #[error("vector api error: {0}")]
    VectorApi(Box<crate::vector_api::VectorApiClientError>),
    #[error("user {0} not found")]
    UserNotFound(Uuid),
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("forbidden: {0}")]
    Forbidden(String),
}

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let status = match self {
            GatewayError::MissingFile
            | GatewayError::InvalidField { .. }
            | GatewayError::Multipart(_) => StatusCode::BAD_REQUEST,
            GatewayError::NotFound(_)
            | GatewayError::DocumentNotFound(_)
            | GatewayError::ContextNotFound(_) => StatusCode::NOT_FOUND,
            GatewayError::ResultUnavailable(_)
            | GatewayError::DocumentResultUnavailable(_)
            | GatewayError::ArtifactMissing { .. } => StatusCode::CONFLICT,
            GatewayError::Storage(_)
            | GatewayError::Kafka(_)
            | GatewayError::Database(_)
            | GatewayError::Tracker(_)
            | GatewayError::Serialization(_)
            | GatewayError::VectorApi(_) => StatusCode::BAD_GATEWAY,
            GatewayError::Inferencer(ref err) => match err.as_ref() {
                crate::inferencer::InferencerClientError::CircuitOpen => {
                    StatusCode::SERVICE_UNAVAILABLE
                }
                crate::inferencer::InferencerClientError::UnexpectedStatus(code)
                    if code.is_client_error() || *code == StatusCode::NOT_FOUND =>
                {
                    *code
                }
                _ => StatusCode::BAD_GATEWAY,
            },
            GatewayError::Archive(_) => StatusCode::BAD_REQUEST,
            GatewayError::Conflict(_) => StatusCode::CONFLICT,
            GatewayError::FileNotFound(_) => StatusCode::NOT_FOUND,
            GatewayError::UserNotFound(_) => StatusCode::NOT_FOUND,
            GatewayError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            GatewayError::Forbidden(_) => StatusCode::FORBIDDEN,
        };

        let body = Json(ErrorResponse::new(self.to_string()));
        (status, body).into_response()
    }
}

impl From<crate::storage::StorageError> for GatewayError {
    fn from(value: crate::storage::StorageError) -> Self {
        GatewayError::Storage(Box::new(value))
    }
}

impl From<crate::kafka::KafkaError> for GatewayError {
    fn from(value: crate::kafka::KafkaError) -> Self {
        GatewayError::Kafka(Box::new(value))
    }
}

impl From<sqlx::Error> for GatewayError {
    fn from(value: sqlx::Error) -> Self {
        GatewayError::Database(Box::new(value))
    }
}

impl From<TrackerError> for GatewayError {
    fn from(value: TrackerError) -> Self {
        GatewayError::Tracker(Box::new(value))
    }
}

impl From<serde_json::Error> for GatewayError {
    fn from(value: serde_json::Error) -> Self {
        GatewayError::Serialization(Box::new(value))
    }
}

impl From<crate::inferencer::InferencerClientError> for GatewayError {
    fn from(value: crate::inferencer::InferencerClientError) -> Self {
        GatewayError::Inferencer(Box::new(value))
    }
}

impl From<crate::vector_api::VectorApiClientError> for GatewayError {
    fn from(value: crate::vector_api::VectorApiClientError) -> Self {
        GatewayError::VectorApi(Box::new(value))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GatewayInitError {
    #[error("invalid listen addr: {0}")]
    InvalidListenAddr(String),
    #[error("kafka tls settings must provide ca, cert and key")]
    KafkaTlsMismatch,
    #[error("database init failed: {0}")]
    Database(Box<sqlx::Error>),
    #[error("database migration failed: {0}")]
    Migration(Box<sqlx::migrate::MigrateError>),
    #[error("storage init failed: {0}")]
    Storage(Box<crate::storage::StorageError>),
    #[error("kafka init failed: {0}")]
    Kafka(Box<crate::kafka::KafkaError>),
    #[error("tracker init failed: {0}")]
    Tracker(Box<crate::tracker::TrackerError>),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

impl ErrorResponse {
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
        }
    }
}

impl From<sqlx::Error> for GatewayInitError {
    fn from(value: sqlx::Error) -> Self {
        GatewayInitError::Database(Box::new(value))
    }
}

impl From<sqlx::migrate::MigrateError> for GatewayInitError {
    fn from(value: sqlx::migrate::MigrateError) -> Self {
        GatewayInitError::Migration(Box::new(value))
    }
}

impl From<crate::storage::StorageError> for GatewayInitError {
    fn from(value: crate::storage::StorageError) -> Self {
        GatewayInitError::Storage(Box::new(value))
    }
}

impl From<crate::kafka::KafkaError> for GatewayInitError {
    fn from(value: crate::kafka::KafkaError) -> Self {
        GatewayInitError::Kafka(Box::new(value))
    }
}

impl From<crate::tracker::TrackerError> for GatewayInitError {
    fn from(value: crate::tracker::TrackerError) -> Self {
        GatewayInitError::Tracker(Box::new(value))
    }
}
