use thiserror::Error;

#[derive(Debug, Error)]
pub enum InferencerClientError {
    #[error("url parse error: {0}")]
    UrlParse(#[from] url::ParseError),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("unexpected status: {0}")]
    UnexpectedStatus(reqwest::StatusCode),
    #[error("unexpected status with body: {0}, {1}")]
    UnexpectedStatusBody(reqwest::StatusCode, String),
    #[error("decode error: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("circuit breaker open: inferencer unavailable")]
    CircuitOpen,
}
