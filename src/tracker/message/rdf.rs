use nauron_contracts::{RdfEvent, RdfProgress, RdfResult, RdfStage};
use uuid::Uuid;

use crate::{
    db::jobs::{JobEngine, JobRecord, JobRepository, JobSnapshotUpsert, JobStatus},
    tracker::TrackerError,
};

use super::lookup_job;

pub(super) fn parse_rdf_event(payload: &str) -> Option<RdfEvent> {
    serde_json::from_str::<RdfEvent>(payload)
        .ok()
        .or_else(|| {
            serde_json::from_str::<RdfResult>(payload)
                .ok()
                .map(RdfEvent::Result)
        })
        .or_else(|| {
            serde_json::from_str::<RdfProgress>(payload)
                .ok()
                .map(RdfEvent::Progress)
        })
}

pub(super) async fn handle_rdf_event(
    job_repo: &JobRepository,
    event: RdfEvent,
) -> Result<(), TrackerError> {
    match event {
        RdfEvent::Progress(progress) => {
            let current = lookup_job(job_repo, progress.job_id).await?;
            job_repo
                .upsert_snapshot(build_rdf_progress_upsert(progress, &current)?)
                .await?;
        }
        RdfEvent::Result(result) => {
            let current = lookup_job(job_repo, rdf_job_id(&result)).await?;
            job_repo
                .upsert_snapshot(build_rdf_result_upsert(result, &current)?)
                .await?;
        }
    }
    Ok(())
}

fn build_rdf_progress_upsert(
    progress: RdfProgress,
    current: &JobRecord,
) -> Result<JobSnapshotUpsert, TrackerError> {
    let terminal = is_terminal_status(current.status);
    let status = if terminal {
        current.status
    } else {
        JobStatus::InProgress
    };
    let message = if terminal {
        current.message.clone()
    } else {
        progress.message
    };
    let result_json = if terminal {
        current.result_json.clone()
    } else {
        None
    };

    Ok(JobSnapshotUpsert {
        job_id: progress.job_id,
        context_id: progress.context_id,
        file_id: current.file_id,
        pipeline_id: Some(current.pipeline_id),
        source_job_id: current.source_job_id,
        engine: JobEngine::Rdf,
        kind: None,
        status,
        stage: Some(progress.stage.into()),
        progress_pct: Some(progress.percent.into()),
        stage_progress_current: progress
            .stage_current
            .and_then(|value| i32::try_from(value).ok()),
        stage_progress_total: progress
            .stage_total
            .and_then(|value| i32::try_from(value).ok()),
        stage_progress_pct: progress.stage_percent.map(i16::from),
        message,
        result_json,
        updated_at: progress.timestamp,
    })
}

fn is_terminal_status(status: JobStatus) -> bool {
    matches!(
        status,
        JobStatus::Success | JobStatus::Failure | JobStatus::Retryable | JobStatus::Retired
    )
}

fn build_rdf_result_upsert(
    result: RdfResult,
    current: &JobRecord,
) -> Result<JobSnapshotUpsert, TrackerError> {
    let (job_id, context_id, status, updated_at, stage, message) = match &result {
        RdfResult::Success {
            job_id,
            context_id,
            completed_at,
            ..
        } => (
            *job_id,
            *context_id,
            JobStatus::Success,
            *completed_at,
            RdfStage::Completed.into(),
            None,
        ),
        RdfResult::Failure {
            job_id,
            context_id,
            stage,
            message,
            occurred_at,
            ..
        } => (
            *job_id,
            *context_id,
            JobStatus::Failure,
            *occurred_at,
            (*stage).into(),
            Some(message.clone()),
        ),
        RdfResult::Retryable {
            job_id,
            context_id,
            stage,
            message,
            occurred_at,
            ..
        } => (
            *job_id,
            *context_id,
            JobStatus::Retryable,
            *occurred_at,
            (*stage).into(),
            Some(message.clone()),
        ),
    };

    Ok(JobSnapshotUpsert {
        job_id,
        context_id,
        file_id: current.file_id,
        pipeline_id: Some(current.pipeline_id),
        source_job_id: current.source_job_id,
        engine: JobEngine::Rdf,
        kind: None,
        status,
        stage: Some(stage),
        progress_pct: Some(100),
        stage_progress_current: None,
        stage_progress_total: None,
        stage_progress_pct: None,
        message,
        result_json: Some(serde_json::to_value(&result)?),
        updated_at,
    })
}

fn rdf_job_id(result: &RdfResult) -> Uuid {
    match result {
        RdfResult::Success { job_id, .. }
        | RdfResult::Failure { job_id, .. }
        | RdfResult::Retryable { job_id, .. } => *job_id,
    }
}

#[cfg(test)]
mod tests;
