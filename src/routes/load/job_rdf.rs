use std::sync::Arc;

use chrono::Utc;
use nauron_contracts::{ArtifactRef, MirResult};
use uuid::Uuid;

use crate::{
    db::{
        files::FileRecord,
        jobs::{JobEngine, JobKind, JobSnapshotUpsert, JobStatus},
    },
    error::GatewayError,
    state::AppState,
    tracker::rdf::{build_rdf_start_for_context, mir_result_job_id},
};

pub async fn start_rdf_job(
    state: &Arc<AppState>,
    file: &FileRecord,
    context_id: i32,
    pipeline_id: Uuid,
    mir_result: &MirResult,
    artifact: &ArtifactRef,
) -> Result<Uuid, GatewayError> {
    let payload = build_rdf_start_for_context(
        mir_result,
        context_id,
        file.doc_id.unwrap_or_else(|| mir_result_job_id(mir_result)),
        artifact,
    )
    .ok_or_else(|| GatewayError::ResultUnavailable("MIR result invalid for RDF".into()))?;

    let job_id = payload.job_id;

    let snapshot = JobSnapshotUpsert {
        job_id,
        context_id,
        file_id: Some(file.id),
        pipeline_id: Some(pipeline_id),
        source_job_id: Some(mir_result_job_id(mir_result)),
        engine: JobEngine::Rdf,
        kind: Some(JobKind::Retry),
        status: JobStatus::Pending,
        stage: None,
        progress_pct: None,
        stage_progress_current: None,
        stage_progress_total: None,
        stage_progress_pct: None,
        message: Some("rdf retried".into()),
        result_json: None,
        updated_at: Utc::now(),
    };
    state.job_repo.upsert_snapshot(snapshot).await?;

    state.rdf_publisher.publish_json(job_id, &payload).await?;

    Ok(job_id)
}
