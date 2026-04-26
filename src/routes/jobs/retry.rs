use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path, State};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::db::jobs::{JobEngine, JobRecord, JobSnapshotUpsert, JobStatus};
use crate::error::{ErrorResponse, GatewayError};
use crate::job_mode::JobLaunchMode;
use crate::routes::contexts::ensure_context_write_access;
use crate::routes::jobs::download::select_artifact;
use crate::routes::load::job_actions;
use crate::state::AppState;
use crate::tracker::JobResultPayload;

#[derive(Deserialize, ToSchema, IntoParams)]
pub(crate) struct RetryRequest {
    #[serde(default)]
    target: Option<JobEngine>,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct JobRetryResponse {
    original_job_id: Uuid,
    new_job_id: Uuid,
    job_mode: JobLaunchMode,
}

#[utoipa::path(
    post,
    path = "/v1/jobs/{job_id}/retry",
    summary = "Retry a failed job",
    description = "Retries a failed or retryable job. Creates a new job and marks the original as retired. \
Use 'target' to specify which stage to retry: 'mir' (document extraction) or 'rdf' (knowledge graph extraction).",
    params(
        ("job_id" = Uuid, Path, description = "Job identifier")
    ),
    request_body(content = RetryRequest, example = json!({
        "target": "mir"
    })),
    responses(
        (status = 200, description = "Job retried successfully", body = JobRetryResponse),
        (status = 404, description = "Job not found", body = ErrorResponse),
        (status = 409, description = "Job cannot be retried", body = ErrorResponse)
    ),
    tag = "Jobs"
)]
pub(crate) async fn retry_job(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthUser>,
    Path(job_id): Path<Uuid>,
    Json(request): Json<RetryRequest>,
) -> Result<Json<JobRetryResponse>, GatewayError> {
    let job = state
        .job_repo
        .get(job_id)
        .await?
        .ok_or_else(|| GatewayError::NotFound(job_id.to_string()))?;

    let Some(file_id) = job.file_id else {
        return Err(GatewayError::Conflict(
            "Job is not associated with a file".into(),
        ));
    };

    let file = state
        .file_repo
        .find_by_id(file_id)
        .await?
        .ok_or(GatewayError::FileNotFound(file_id))?;

    let target_engine = match request.target {
        Some(engine) => engine,
        None => JobEngine::Mir,
    };
    ensure_context_write_access(&state, job.context_id, &user).await?;

    match target_engine {
        JobEngine::Mir => {
            // Check if there is already an active job for this file/context
            if let Some(current_job_id) = file.mir_job_id.as_ref() {
                let in_flight = state
                    .job_repo
                    .get(*current_job_id)
                    .await?
                    .map(|j| matches!(j.status, JobStatus::Pending | JobStatus::InProgress))
                    .is_some_and(|value| value);

                if in_flight {
                    // If the in-flight job is the one we are retrying, we allow it (force retry).
                    // Otherwise, it's a conflict with another job.
                    if *current_job_id != job_id {
                        return Err(GatewayError::Conflict(format!(
                            "File {file_id} is already being processed by job {current_job_id}"
                        )));
                    }
                }
            }

            // Reset file to pending state to allow processing
            let pending_file = state.file_repo.reset_pending(file_id).await?;

            let (new_job_id, _updated_file) = job_actions::start_mir_job(
                &state,
                &pending_file,
                job.context_id,
                job.pipeline_id,
                None,
                false,
            )
            .await?;

            mark_job_retried(&state, &job, new_job_id).await?;

            Ok(Json(JobRetryResponse {
                original_job_id: job_id,
                new_job_id,
                job_mode: JobLaunchMode::Started,
            }))
        }
        JobEngine::Rdf => {
            // For RDF retry, we need the MIR result.
            // If the current job is MIR and successful, use it.
            // If the current job is RDF, find the MIR job for the file.

            let mir_job = if job.engine == JobEngine::Mir && job.status == JobStatus::Success {
                job.clone()
            } else {
                // Pick the most recent successful MIR job in the same pipeline.
                state
                    .job_repo
                    .list_by_pipeline(job.pipeline_id)
                    .await?
                    .into_iter()
                    .find(|candidate| {
                        candidate.engine == JobEngine::Mir && candidate.status == JobStatus::Success
                    })
                    .ok_or_else(|| {
                        GatewayError::ResultUnavailable(
                            "No successful MIR job found for this pipeline".into(),
                        )
                    })?
            };

            let Some(JobResultPayload::Mir(mir_result)) = mir_job
                .result_json
                .clone()
                .and_then(|v| serde_json::from_value(v).ok())
            else {
                return Err(GatewayError::ResultUnavailable(
                    "MIR result missing or invalid".into(),
                ));
            };

            let artifact = select_artifact(&mir_result, None).ok_or_else(|| {
                GatewayError::ArtifactMissing {
                    job_id: mir_job.job_id.to_string(),
                }
            })?;

            let new_job_id = job_actions::start_rdf_job(
                &state,
                &file,
                job.context_id,
                job.pipeline_id,
                &mir_result,
                artifact,
            )
            .await?;

            mark_job_retried(&state, &job, new_job_id).await?;

            Ok(Json(JobRetryResponse {
                original_job_id: job_id,
                new_job_id,
                job_mode: JobLaunchMode::Started,
            }))
        }
        _ => Err(GatewayError::InvalidField {
            field: "target".into(),
            message: "Unsupported retry target".into(),
        }),
    }
}

async fn mark_job_retried(
    state: &AppState,
    job: &JobRecord,
    new_job_id: Uuid,
) -> Result<(), GatewayError> {
    state
        .job_repo
        .upsert_snapshot(JobSnapshotUpsert {
            job_id: job.job_id,
            context_id: job.context_id,
            file_id: job.file_id,
            pipeline_id: Some(job.pipeline_id),
            source_job_id: job.source_job_id,
            engine: job.engine,
            kind: job.kind,
            status: JobStatus::Retired,
            stage: job.stage,
            progress_pct: job.progress_pct,
            stage_progress_current: None,
            stage_progress_total: None,
            stage_progress_pct: None,
            message: Some(format!("superseded by retry {new_job_id}")),
            result_json: job.result_json.clone(),
            updated_at: Utc::now(),
        })
        .await?;
    Ok(())
}
