use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::db::jobs::{JobEngine, JobKind, JobRecord, JobStatus};
use crate::error::{ErrorResponse, GatewayError};
use crate::routes::jobs::JobStatusResponse;
use crate::state::AppState;
use crate::tracker::JobSnapshot;
use utoipa::ToSchema;

#[utoipa::path(
    get,
    path = "/v1/pipelines/{pipeline_id}",
    summary = "Get pipeline status",
    description = "Returns aggregated status of all jobs in a document processing pipeline. \
A pipeline tracks a single file through MIR extraction, RDF processing, and embedding stages. \
Returns overall status and individual job details.",
    params(
        ("pipeline_id" = Uuid, Path, description = "Pipeline identifier")
    ),
    responses(
        (status = 200, description = "Aggregated pipeline status", body = PipelineStatusResponse),
        (status = 404, description = "Pipeline not found", body = ErrorResponse)
    ),
    tag = "Pipelines"
)]
pub(crate) async fn pipeline_status(
    State(state): State<Arc<AppState>>,
    Path(pipeline_id): Path<Uuid>,
) -> Result<Json<PipelineStatusResponse>, GatewayError> {
    let context_file = state
        .file_repo
        .find_by_pipeline_id(pipeline_id)
        .await?
        .ok_or_else(|| GatewayError::NotFound(pipeline_id.to_string()))?;

    let file = state
        .file_repo
        .find_by_id(context_file.file_id)
        .await?
        .ok_or_else(|| GatewayError::FileNotFound(context_file.file_id))?;

    let records = state.job_repo.list_by_pipeline(pipeline_id).await?;
    if records.is_empty() {
        return Err(GatewayError::NotFound(format!(
            "no jobs found for pipeline {pipeline_id}"
        )));
    }

    let status = summarize_pipeline(&records);
    let snapshots = records
        .into_iter()
        .map(JobSnapshot::try_from)
        .collect::<Result<Vec<_>, _>>()?;
    let updated_at = snapshots
        .iter()
        .map(|snapshot| snapshot.updated_at)
        .max()
        .unwrap_or_else(Utc::now);

    let jobs = snapshots
        .into_iter()
        .map(|snapshot| JobStatusResponse::from_snapshot(snapshot, Some(&file)))
        .collect();

    Ok(Json(PipelineStatusResponse {
        pipeline_id,
        context_id: context_file.context_id,
        file_id: context_file.file_id,
        sha256_hex: hex::encode(&context_file.file_sha256),
        status,
        jobs,
        updated_at,
        mir_artifact_uri: context_file.mir_artifact_uri,
    }))
}

fn summarize_pipeline(records: &[JobRecord]) -> JobStatus {
    let active: Vec<&JobRecord> = records
        .iter()
        .filter(|record| record.status != JobStatus::Retired)
        .collect();

    let view = if active.is_empty() {
        records.iter().collect()
    } else {
        active
    };
    let filtered = filter_linked_mir(view);

    if filtered
        .iter()
        .any(|record| record.status == JobStatus::Failure)
    {
        JobStatus::Failure
    } else if filtered
        .iter()
        .any(|record| record.status == JobStatus::Retryable)
    {
        JobStatus::Retryable
    } else if filtered
        .iter()
        .all(|record| record.status == JobStatus::Success)
    {
        JobStatus::Success
    } else if filtered
        .iter()
        .any(|record| record.status == JobStatus::InProgress)
    {
        JobStatus::InProgress
    } else {
        JobStatus::Pending
    }
}

fn filter_linked_mir(records: Vec<&JobRecord>) -> Vec<&JobRecord> {
    let has_canonical_mir = records
        .iter()
        .any(|record| record.engine == JobEngine::Mir && !is_linked_mir(record));
    if !has_canonical_mir {
        return records;
    }

    let filtered = records
        .iter()
        .copied()
        .filter(|record| !is_linked_mir(record))
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        records
    } else {
        filtered
    }
}

fn is_linked_mir(record: &JobRecord) -> bool {
    record.engine == JobEngine::Mir && record.kind == Some(JobKind::MirLinked)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use super::*;

    fn sample_job(
        job_id: Uuid,
        engine: JobEngine,
        kind: Option<JobKind>,
        status: JobStatus,
    ) -> JobRecord {
        JobRecord {
            job_id,
            context_id: 7,
            file_id: Some(11),
            pipeline_id: Uuid::nil(),
            source_job_id: None,
            engine,
            kind,
            status,
            stage: None,
            progress_pct: None,
            stage_progress_current: None,
            stage_progress_total: None,
            stage_progress_pct: None,
            message: None,
            result_json: None,
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn summary_ignores_linked_mir_when_canonical_exists() {
        let records = vec![
            sample_job(
                Uuid::parse_str("44444444-4444-4444-4444-444444444444").expect("uuid"),
                JobEngine::Mir,
                Some(JobKind::MirLinked),
                JobStatus::Pending,
            ),
            sample_job(
                Uuid::parse_str("55555555-5555-5555-5555-555555555555").expect("uuid"),
                JobEngine::Mir,
                None,
                JobStatus::Success,
            ),
            sample_job(
                Uuid::parse_str("66666666-6666-6666-6666-666666666666").expect("uuid"),
                JobEngine::Rdf,
                None,
                JobStatus::Success,
            ),
        ];

        assert_eq!(summarize_pipeline(&records), JobStatus::Success);
    }

    #[test]
    fn summary_keeps_linked_mir_when_no_canonical_exists() {
        let records = vec![sample_job(
            Uuid::parse_str("77777777-7777-7777-7777-777777777777").expect("uuid"),
            JobEngine::Mir,
            Some(JobKind::MirLinked),
            JobStatus::Pending,
        )];

        assert_eq!(summarize_pipeline(&records), JobStatus::Pending);
    }
}

#[derive(Serialize, ToSchema)]
pub(crate) struct PipelineStatusResponse {
    pipeline_id: Uuid,
    context_id: i32,
    file_id: i64,
    sha256_hex: String,
    status: JobStatus,
    updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mir_artifact_uri: Option<String>,
    jobs: Vec<JobStatusResponse>,
}
