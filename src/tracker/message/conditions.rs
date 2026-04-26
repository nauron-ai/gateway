use nauron_contracts::conditions::{
    ConditionErrorResponse, ConditionEvaluationResponse, ConditionsEvaluateEvent,
    ConditionsEvaluateProgress, ConditionsEvaluateResult, ConditionsEvaluateStage,
};
use uuid::Uuid;

use crate::db::jobs::{JobEngine, JobRecord, JobRepository, JobSnapshotUpsert, JobStatus};
use crate::metrics::GatewayMetrics;
use crate::tracker::TrackerError;

use super::lookup_job;

pub(super) async fn handle_conditions_event(
    job_repo: &JobRepository,
    _metrics: &GatewayMetrics,
    event: ConditionsEvaluateEvent,
) -> Result<(), TrackerError> {
    match event {
        ConditionsEvaluateEvent::Progress(progress) => {
            let current = lookup_job(job_repo, progress.job_id).await?;
            job_repo
                .upsert_snapshot(build_conditions_progress_upsert(progress, &current)?)
                .await?;
        }
        ConditionsEvaluateEvent::Result(result) => {
            let current = lookup_job(job_repo, conditions_job_id(&result)).await?;
            job_repo
                .upsert_snapshot(build_conditions_result_upsert(result, &current)?)
                .await?;
        }
    }
    Ok(())
}

fn build_conditions_progress_upsert(
    progress: ConditionsEvaluateProgress,
    current: &JobRecord,
) -> Result<JobSnapshotUpsert, TrackerError> {
    Ok(JobSnapshotUpsert {
        job_id: progress.job_id,
        context_id: progress.context_id,
        file_id: current.file_id,
        pipeline_id: Some(current.pipeline_id),
        source_job_id: current.source_job_id,
        engine: JobEngine::Conditions,
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

fn build_conditions_result_upsert(
    result: ConditionsEvaluateResult,
    current: &JobRecord,
) -> Result<JobSnapshotUpsert, TrackerError> {
    let (job_id, context_id, status, updated_at, message, result_json) = match &result {
        ConditionsEvaluateResult::Success {
            job_id,
            context_id,
            completed_at,
            response,
            ..
        } => (
            *job_id,
            *context_id,
            JobStatus::Success,
            *completed_at,
            None,
            Some(serialize_success_payload(response)?),
        ),
        ConditionsEvaluateResult::Failure {
            job_id,
            context_id,
            occurred_at,
            error,
            ..
        } => (
            *job_id,
            *context_id,
            JobStatus::Failure,
            *occurred_at,
            Some(error.error.message.clone()),
            Some(serialize_failure_payload(error)?),
        ),
    };

    Ok(JobSnapshotUpsert {
        job_id,
        context_id,
        file_id: current.file_id,
        pipeline_id: Some(current.pipeline_id),
        source_job_id: current.source_job_id,
        engine: JobEngine::Conditions,
        kind: None,
        status,
        stage: Some(ConditionsEvaluateStage::Completed.into()),
        progress_pct: Some(100),
        stage_progress_current: None,
        stage_progress_total: None,
        stage_progress_pct: None,
        message,
        result_json,
        updated_at,
    })
}

fn serialize_success_payload(
    response: &ConditionEvaluationResponse,
) -> Result<serde_json::Value, TrackerError> {
    serde_json::to_value(&ConditionsResultPayload::Success {
        response: response.clone(),
    })
    .map_err(TrackerError::from)
}

fn serialize_failure_payload(
    error: &ConditionErrorResponse,
) -> Result<serde_json::Value, TrackerError> {
    serde_json::to_value(&ConditionsResultPayload::Failure {
        error: error.clone(),
    })
    .map_err(TrackerError::from)
}

#[derive(serde::Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum ConditionsResultPayload {
    Success {
        response: ConditionEvaluationResponse,
    },
    Failure {
        error: ConditionErrorResponse,
    },
}

fn conditions_job_id(result: &ConditionsEvaluateResult) -> Uuid {
    match result {
        ConditionsEvaluateResult::Success { job_id, .. }
        | ConditionsEvaluateResult::Failure { job_id, .. } => *job_id,
    }
}
