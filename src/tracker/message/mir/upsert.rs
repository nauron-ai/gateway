use nauron_contracts::{MirProgress, MirResult, MirStage};
use uuid::Uuid;

use crate::{
    db::jobs::{JobEngine, JobRecord, JobSnapshotUpsert, JobStatus},
    tracker::TrackerError,
};

pub(super) fn build_mir_progress_upsert(
    progress: MirProgress,
    current: &JobRecord,
) -> Result<JobSnapshotUpsert, TrackerError> {
    Ok(JobSnapshotUpsert {
        job_id: progress.job_id,
        context_id: current.context_id,
        file_id: current.file_id,
        pipeline_id: Some(current.pipeline_id),
        source_job_id: current.source_job_id,
        engine: JobEngine::Mir,
        kind: None,
        status: JobStatus::InProgress,
        stage: Some(progress.stage.into()),
        progress_pct: Some(progress.percent.into()),
        stage_progress_current: None,
        stage_progress_total: None,
        stage_progress_pct: None,
        message: progress.message,
        result_json: None,
        updated_at: progress.timestamp,
    })
}

pub(super) fn build_mir_result_upsert(
    result: MirResult,
    current: &JobRecord,
) -> Result<JobSnapshotUpsert, TrackerError> {
    let (job_id, status, timestamp, message) = match &result {
        MirResult::Success {
            job_id,
            completed_at,
            ..
        } => (*job_id, JobStatus::Success, *completed_at, None),
        MirResult::Failure {
            job_id,
            message,
            occurred_at,
            ..
        } => (
            *job_id,
            JobStatus::Failure,
            *occurred_at,
            Some(message.clone()),
        ),
        MirResult::Retryable {
            job_id,
            message,
            occurred_at,
            ..
        } => (
            *job_id,
            JobStatus::Retryable,
            *occurred_at,
            Some(message.clone()),
        ),
    };

    let result_json = Some(serde_json::to_value(&result)?);
    Ok(JobSnapshotUpsert {
        job_id,
        context_id: current.context_id,
        file_id: current.file_id,
        pipeline_id: Some(current.pipeline_id),
        source_job_id: current.source_job_id,
        engine: JobEngine::Mir,
        kind: None,
        status,
        stage: Some(MirStage::Completed.into()),
        progress_pct: Some(100),
        stage_progress_current: None,
        stage_progress_total: None,
        stage_progress_pct: None,
        message,
        result_json,
        updated_at: timestamp,
    })
}

pub(super) fn result_job_id(result: &MirResult) -> Uuid {
    match result {
        MirResult::Success { job_id, .. }
        | MirResult::Failure { job_id, .. }
        | MirResult::Retryable { job_id, .. } => *job_id,
    }
}
