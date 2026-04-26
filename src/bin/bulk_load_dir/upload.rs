use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use thiserror::Error;
use uuid::Uuid;

use crate::gateway_client::GatewayClient;

#[derive(Debug, Clone)]
pub struct UploadConfig {
    pub token: String,
    pub context_id: i32,
    pub max_bytes: u64,
    pub dry_run: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoadResponse {
    pub pipeline_id: Uuid,
    pub job_id: Uuid,
    pub context_id: i32,
    pub file_id: i64,
    pub sha256_hex: String,
    pub deduplicated: bool,
    pub status_url: String,
}

#[derive(Debug, Clone)]
pub enum UploadOutcome {
    Uploaded(LoadResponse),
    SkippedTooLarge { size_bytes: u64, max_bytes: u64 },
    Failed(String),
}

#[derive(Debug, Error)]
pub enum UploadError {
    #[error("failed to read file: {0}")]
    Read(#[from] std::io::Error),
    #[error("file too large: {size_bytes} > {max_bytes}")]
    TooLarge { size_bytes: u64, max_bytes: u64 },
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
}

pub async fn upload_one(
    client: &GatewayClient,
    config: &UploadConfig,
    path: &Path,
) -> Result<LoadResponse, UploadError> {
    let size_bytes = fs::metadata(path)?.len();
    if size_bytes > config.max_bytes {
        return Err(UploadError::TooLarge {
            size_bytes,
            max_bytes: config.max_bytes,
        });
    }

    let file_name = match path.file_name().and_then(OsStr::to_str) {
        Some(value) => value.to_string(),
        None => DEFAULT_FILE_NAME.to_string(),
    };
    let bytes = fs::read(path)?;

    let mut part = Part::bytes(bytes).file_name(file_name);
    if let Some(mime) = mime_for_path(path) {
        part = part.mime_str(mime).map_err(UploadError::Http)?;
    }

    let form = Form::new()
        .text("context_id", config.context_id.to_string())
        .text("dry_run", config.dry_run.to_string())
        .part("file", part);

    let url = format!("{}/v1/load/context", client.base_url());
    let response = client
        .http()
        .post(url)
        .bearer_auth(&config.token)
        .multipart(form)
        .send()
        .await?
        .error_for_status()?;
    let body: LoadResponse = response.json().await?;
    Ok(body)
}

fn mime_for_path(path: &Path) -> Option<&'static str> {
    let ext = path
        .extension()
        .and_then(OsStr::to_str)?
        .to_ascii_lowercase();
    match ext.as_str() {
        "pdf" => Some("application/pdf"),
        "txt" => Some("text/plain"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        _ => None,
    }
}

const DEFAULT_FILE_NAME: &str = "file.bin";

impl From<UploadError> for UploadOutcome {
    fn from(value: UploadError) -> Self {
        match value {
            UploadError::TooLarge {
                size_bytes,
                max_bytes,
            } => UploadOutcome::SkippedTooLarge {
                size_bytes,
                max_bytes,
            },
            other => UploadOutcome::Failed(other.to_string()),
        }
    }
}
