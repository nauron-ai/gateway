use thiserror::Error;

#[derive(Debug, Error)]
pub enum VectorApiClientError {
    #[error("url parse error: {0}")]
    UrlParse(#[from] url::ParseError),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("unexpected status: {0}")]
    UnexpectedStatus(reqwest::StatusCode),
}
