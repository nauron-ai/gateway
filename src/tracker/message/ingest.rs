use nauron_contracts::{IngestEvent, IngestProgress, IngestResult, IngestStage};
use uuid::Uuid;

use crate::db::jobs::{JobEngine, JobRecord, JobRepository, JobSnapshotUpsert, JobStatus};
use crate::metrics::GatewayMetrics;
use crate::tracker::TrackerError;

use super::lookup_job;

pub(super) async fn handle_ingest_event(
    job_repo: &JobRepository,
    _metrics: &GatewayMetrics,
    event: IngestEvent,
) -> Result<(), TrackerError> {
    match event {
        IngestEvent::Progress(progress) => {
            let current = lookup_job(job_repo, progress.job_id).await?;
            job_repo
                .upsert_snapshot(build_ingest_progress_upsert(progress, &current)?)
                .await?;
        }
        IngestEvent::Result(result) => {
            let current = lookup_job(job_repo, ingest_job_id(&result)).await?;
            job_repo
                .upsert_snapshot(build_ingest_result_upsert(result, &current)?)
                .await?;
        }
    }
    Ok(())
}

fn build_ingest_progress_upsert(
    progress: IngestProgress,
    current: &JobRecord,
) -> Result<JobSnapshotUpsert, TrackerError> {
    Ok(JobSnapshotUpsert {
        job_id: progress.job_id,
        context_id: progress.context_id,
        file_id: current.file_id,
        pipeline_id: Some(current.pipeline_id),
        source_job_id: current.source_job_id,
        engine: JobEngine::Ingest,
        kind: None,
        status: JobStatus::InProgress,
        stage: Some(progress.stage.into()),
        progress_pct: Some((progress.percent as i16).clamp(0, 100)),
        stage_progress_current: progress.stage_progress_current.map(|value| value as i32),
        stage_progress_total: progress.stage_progress_total.map(|value| value as i32),
        stage_progress_pct: progress.stage_progress_pct.map(i16::from),
        message: progress.message,
        result_json: None,
        updated_at: progress.timestamp,
    })
}

fn build_ingest_result_upsert(
    result: IngestResult,
    current: &JobRecord,
) -> Result<JobSnapshotUpsert, TrackerError> {
    let (job_id, context_id, status, updated_at, message) = match &result {
        IngestResult::Success {
            job_id,
            context_id,
            completed_at,
            ..
        } => (
            *job_id,
            *context_id,
            JobStatus::Success,
            *completed_at,
            None,
        ),
        IngestResult::Failure {
            job_id,
            context_id,
            message,
            occurred_at,
            ..
        } => (
            *job_id,
            *context_id,
            JobStatus::Failure,
            *occurred_at,
            Some(message.clone()),
        ),
    };

    Ok(JobSnapshotUpsert {
        job_id,
        context_id,
        file_id: current.file_id,
        pipeline_id: Some(current.pipeline_id),
        source_job_id: current.source_job_id,
        engine: JobEngine::Ingest,
        kind: None,
        status,
        stage: Some(IngestStage::Completed.into()),
        progress_pct: Some(100),
        stage_progress_current: None,
        stage_progress_total: None,
        stage_progress_pct: None,
        message,
        result_json: Some(serde_json::to_value(&result)?),
        updated_at,
    })
}

fn ingest_job_id(result: &IngestResult) -> Uuid {
    match result {
        IngestResult::Success { job_id, .. } | IngestResult::Failure { job_id, .. } => *job_id,
    }
}
