use std::sync::Arc;

use axum::Json;
use axum::extract::{Multipart, State};
use axum::response::IntoResponse;

use crate::auth::AuthUser;
use crate::error::{ErrorResponse, GatewayError};
use crate::routes::contexts::ensure_context_write_access;
use crate::routes::load::response::{LoadContextForm, LoadContextResponse};
use crate::state::AppState;

mod form;
mod helpers;
mod ingest;
pub mod job_actions;
mod job_helpers;
mod job_rdf;
pub(crate) mod response;
mod service;

#[utoipa::path(
    post,
    path = "/v1/load/context",
    summary = "Upload documents to context",
    description = "Uploads one or more documents to a context for processing. Files are stored and queued for \
MIR extraction followed by optional RDF/knowledge graph processing based on context mode. Returns pipeline_id for tracking.",
    request_body(
        content = inline(LoadContextForm),
        content_type = "multipart/form-data",
        description = "Multipart payload with context metadata and files"
    ),
    responses(
        (status = 200, description = "File accepted for processing", body = LoadContextResponse),
        (status = 400, description = "Invalid multipart payload", body = ErrorResponse),
        (status = 502, description = "External dependency unavailable", body = ErrorResponse)
    ),
    tag = "Load"
)]
pub(crate) async fn load_context(
    State(state): State<Arc<AppState>>,
    axum::Extension(user): axum::Extension<AuthUser>,
    payload: Multipart,
) -> Result<impl IntoResponse, GatewayError> {
    let request = form::UploadRequest::parse(payload).await?;
    ensure_context_write_access(&state, request.form.context_id, &user).await?;
    let mut request = request;
    request.form.user_id = Some(user.id.to_string());

    // Touch the documented schema type so its fields are considered used for linting purposes.
    let form = &request.form;
    let _schema_hint = LoadContextForm {
        file: form.file.clone(),
        context_id: form.context_id,
        user_id: form.user_id.clone(),
        dry_run: form.dry_run,
    };
    let _ = _schema_hint;

    let response = service::process_upload(state, request).await?;
    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::form::UploadedFile;
    use super::helpers;
    use super::ingest::is_zip_upload;

    #[test]
    fn filename_sanitized() {
        assert_eq!(
            helpers::sanitize_filename("some weird/name.pdf"),
            "some_weird_name.pdf"
        );
    }

    #[test]
    fn detects_regular_zip_upload_by_magic_bytes() {
        let file = UploadedFile {
            bytes: Bytes::from_static(b"PK\x03\x04\x14\x00"),
            name: Some("archive.zip".into()),
            content_type: Some("application/octet-stream".into()),
        };
        assert!(is_zip_upload(&file));
    }

    #[test]
    fn rejects_docx_even_when_magic_bytes_match_zip() {
        let file = UploadedFile {
            bytes: Bytes::from_static(b"PK\x03\x04\x14\x00"),
            name: Some("document.docx".into()),
            content_type: Some(
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document".into(),
            ),
        };
        assert!(!is_zip_upload(&file));
    }

    #[test]
    fn rejects_docx_when_mime_is_zip_but_extension_is_ooxml() {
        let file = UploadedFile {
            bytes: Bytes::from_static(b"PK\x03\x04\x14\x00"),
            name: Some("document.docx".into()),
            content_type: Some("application/zip".into()),
        };
        assert!(!is_zip_upload(&file));
    }

    #[test]
    fn rejects_zip_extension_when_mime_is_ooxml() {
        let file = UploadedFile {
            bytes: Bytes::from_static(b"PK\x03\x04\x14\x00"),
            name: Some("document.zip".into()),
            content_type: Some(
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document".into(),
            ),
        };
        assert!(!is_zip_upload(&file));
    }

    #[test]
    fn rejects_pptx_by_extension_when_mime_missing() {
        let file = UploadedFile {
            bytes: Bytes::from_static(b"PK\x03\x04\x14\x00"),
            name: Some("slides.PPTX".into()),
            content_type: None,
        };
        assert!(!is_zip_upload(&file));
    }

    #[test]
    fn rejects_docm_by_extension() {
        let file = UploadedFile {
            bytes: Bytes::from_static(b"PK\x03\x04\x14\x00"),
            name: Some("template.docm".into()),
            content_type: None,
        };
        assert!(!is_zip_upload(&file));
    }
}
