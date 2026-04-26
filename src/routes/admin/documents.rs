use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderValue;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::response::Response;
use uuid::Uuid;

use crate::error::{ErrorResponse, GatewayError};
use crate::routes::download_headers::attachment_disposition;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/admin/documents/{doc_id}",
    summary = "Download document by doc_id (admin)",
    description = "Downloads the canonical MIR document artifact using the stable document identifier. Admin only.",
    params(("doc_id" = Uuid, Path, description = "Canonical document identifier")),
    responses(
        (status = 200, description = "Downloaded document", content_type = "application/octet-stream", body = String),
        (status = 404, description = "Document not found", body = ErrorResponse),
        (status = 409, description = "Document artifact not yet available", body = ErrorResponse)
    ),
    tag = "Admin"
)]
pub async fn download_document(
    State(state): State<Arc<AppState>>,
    Path(doc_id): Path<Uuid>,
) -> Result<Response, GatewayError> {
    let file = state
        .file_repo
        .find_by_doc_id(doc_id)
        .await?
        .ok_or(GatewayError::DocumentNotFound(doc_id))?;
    let artifact_uri = file
        .mir_artifact_uri
        .as_deref()
        .ok_or(GatewayError::DocumentResultUnavailable(doc_id))?;
    let (bucket, key) =
        parse_s3_uri(artifact_uri).ok_or(GatewayError::DocumentResultUnavailable(doc_id))?;
    let bytes = state.storage.download(bucket, key).await?;
    let mut response = Response::new(bytes.into());

    if let Some(content_type) = infer_content_type(key)
        && let Ok(value) = HeaderValue::from_str(content_type)
    {
        response.headers_mut().insert(CONTENT_TYPE, value);
    }
    if !response.headers().contains_key(CONTENT_TYPE) {
        response.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/octet-stream"),
        );
    }
    if let Some(filename) = key.rsplit('/').next()
        && let Some(value) = attachment_disposition(filename)
    {
        response.headers_mut().insert(CONTENT_DISPOSITION, value);
    }

    Ok(response)
}

fn parse_s3_uri(uri: &str) -> Option<(&str, &str)> {
    let value = uri.strip_prefix("s3://")?;
    let (bucket, key) = value.split_once('/')?;
    if bucket.is_empty() || key.is_empty() {
        return None;
    }
    Some((bucket, key))
}

fn infer_content_type(key: &str) -> Option<&'static str> {
    if key.ends_with(".md") {
        return Some("text/markdown; charset=utf-8");
    }
    if key.ends_with(".txt") {
        return Some("text/plain; charset=utf-8");
    }
    if key.ends_with(".json") {
        return Some("application/json");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{infer_content_type, parse_s3_uri};

    #[test]
    fn parses_s3_uri() {
        let (bucket, key) = parse_s3_uri("s3://mir-output/jobs/doc.md").expect("uri");
        assert_eq!(bucket, "mir-output");
        assert_eq!(key, "jobs/doc.md");
    }

    #[test]
    fn rejects_invalid_s3_uri() {
        assert!(parse_s3_uri("http://mir-output/jobs/doc.md").is_none());
        assert!(parse_s3_uri("s3://mir-output").is_none());
        assert!(parse_s3_uri("s3://bucket/").is_none());
        assert!(parse_s3_uri("s3:///key").is_none());
        assert!(parse_s3_uri("s3://").is_none());
    }

    #[test]
    fn infers_markdown_content_type() {
        assert_eq!(
            infer_content_type("jobs/document.md"),
            Some("text/markdown; charset=utf-8")
        );
        assert_eq!(infer_content_type("jobs/document.bin"), None);
    }
}
