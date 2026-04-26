use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::HeaderValue;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::response::Response;
use nauron_contracts::{ArtifactRef, MirResult};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::artifacts::{select_archive_artifact, select_document_artifact};
use crate::error::{ErrorResponse, GatewayError};
use crate::routes::download_headers::attachment_disposition;
use crate::state::AppState;
use crate::tracker::JobResultPayload;

use super::status::load_snapshot;

#[utoipa::path(
    get,
    path = "/v1/jobs/{job_id}/result",
    summary = "Download job artifact",
    description = "Downloads processing artifact from a completed MIR job. Use 'artifact' query param to select \
'document' (parsed content) or 'archive' (all outputs). Returns 409 if job not yet complete.",
    params(
        ("job_id" = Uuid, Path, description = "Job identifier"),
        DownloadQuery
    ),
    responses(
        (status = 200, description = "Downloaded artifact", content_type = "application/octet-stream", body = String),
        (status = 404, description = "Job not found", body = ErrorResponse),
        (status = 409, description = "Result not yet available", body = ErrorResponse)
    ),
    tag = "Jobs"
)]
pub(crate) async fn download_result(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
    Query(query): Query<DownloadQuery>,
) -> Result<Response, GatewayError> {
    let snapshot = load_snapshot(&state, job_id).await?;
    let mir_result = match snapshot.result.as_ref() {
        Some(JobResultPayload::Mir(result)) => result,
        _ => return Err(GatewayError::ResultUnavailable(job_id.to_string())),
    };
    let artifact = select_artifact(mir_result, query.artifact.as_deref()).ok_or_else(|| {
        GatewayError::ArtifactMissing {
            job_id: job_id.to_string(),
        }
    })?;
    let bytes = state
        .storage
        .download(artifact.bucket.as_str(), artifact.key.as_str())
        .await?;
    let mut response = Response::new(bytes.into());
    if let Some(ct) = artifact.content_type.as_deref()
        && let Ok(value) = HeaderValue::from_str(ct)
    {
        response.headers_mut().insert(CONTENT_TYPE, value);
    }
    if !response.headers().contains_key(CONTENT_TYPE) {
        response.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/octet-stream"),
        );
    }
    if let Some(filename) = artifact.key.as_str().rsplit('/').next()
        && let Some(value) = attachment_disposition(filename)
    {
        response.headers_mut().insert(CONTENT_DISPOSITION, value);
    }
    Ok(response)
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub(crate) struct DownloadQuery {
    /// Preferred artifact: `document` (default) or `archive`.
    artifact: Option<String>,
}

pub(crate) fn select_artifact<'a>(
    result: &'a MirResult,
    preference: Option<&str>,
) -> Option<&'a ArtifactRef> {
    let artifacts = match result {
        MirResult::Success { artifacts, .. } => artifacts,
        _ => return None,
    };
    if artifacts.is_empty() {
        return None;
    }

    let prefer_archive = matches!(preference, Some(value) if value.eq_ignore_ascii_case("archive"));

    if prefer_archive {
        select_archive_artifact(artifacts)
            .or_else(|| select_document_artifact(artifacts))
            .or_else(|| artifacts.first())
    } else {
        select_document_artifact(artifacts)
            .or_else(|| select_archive_artifact(artifacts))
            .or_else(|| artifacts.first())
    }
}
