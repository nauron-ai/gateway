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
    Ok(JobSnapshotUpsert {
        job_id: progress.job_id,
        context_id: progress.context_id,
        file_id: current.file_id,
        pipeline_id: Some(current.pipeline_id),
        source_job_id: current.source_job_id,
        engine: JobEngine::Rdf,
        kind: None,
        status: JobStatus::InProgress,
        stage: Some(progress.stage.into()),
        progress_pct: Some(progress.percent.into()),
        stage_progress_current: progress
            .stage_current
            .and_then(|value| i32::try_from(value).ok()),
        stage_progress_total: progress
            .stage_total
            .and_then(|value| i32::try_from(value).ok()),
        stage_progress_pct: progress.stage_percent.map(i16::from),
        message: progress.message,
        result_json: None,
        updated_at: progress.timestamp,
    })
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
mod tests {
    use chrono::Utc;
    use nauron_contracts::{FailureKind, RdfResult, SchemaVersion};
    use uuid::Uuid;

    use super::*;
    use crate::test_utils::parse_uuid;

    fn sample_job() -> JobRecord {
        JobRecord {
            job_id: parse_uuid("22222222-2222-2222-2222-222222222222"),
            context_id: 7,
            file_id: Some(11),
            pipeline_id: Uuid::nil(),
            source_job_id: None,
            engine: JobEngine::Rdf,
            kind: None,
            status: JobStatus::InProgress,
            stage: None,
            progress_pct: Some(40),
            stage_progress_current: Some(2),
            stage_progress_total: Some(5),
            stage_progress_pct: Some(40),
            message: None,
            result_json: None,
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn retryable_result_maps_to_retryable_job_status() {
        let result = RdfResult::Retryable {
            schema_version: SchemaVersion::V1,
            job_id: parse_uuid("22222222-2222-2222-2222-222222222222"),
            doc_id: parse_uuid("33333333-3333-3333-3333-333333333333"),
            context_id: 7,
            stage: RdfStage::Persist,
            kind: FailureKind::Upstream,
            message: "provider throttled request".into(),
            details: Some("provider=azure status=429".into()),
            retry_after_seconds: Some(31),
            occurred_at: Utc::now(),
        };

        let upsert = build_rdf_result_upsert(result, &sample_job()).expect("valid upsert");

        assert_eq!(upsert.status, JobStatus::Retryable);
        assert_eq!(
            upsert.message.as_deref(),
            Some("provider throttled request")
        );
        assert_eq!(upsert.stage_progress_current, None);
        assert_eq!(upsert.stage_progress_total, None);
        assert_eq!(upsert.stage_progress_pct, None);
        let payload = upsert.result_json.expect("serialized result");
        assert_eq!(payload["retry_after_seconds"], 31);
        assert_eq!(payload["kind"], "upstream");
        assert_eq!(payload["details"], "provider=azure status=429");
    }

    #[test]
    fn progress_upsert_copies_stage_progress_fields() {
        let progress = RdfProgress {
            schema_version: SchemaVersion::V1,
            job_id: parse_uuid("22222222-2222-2222-2222-222222222222"),
            doc_id: parse_uuid("33333333-3333-3333-3333-333333333333"),
            context_id: 7,
            stage: RdfStage::InformationExtraction,
            percent: 63,
            stage_current: Some(5),
            stage_total: Some(8),
            stage_percent: Some(62),
            message: Some("extracting relations".into()),
            timestamp: Utc::now(),
        };

        let upsert = build_rdf_progress_upsert(progress, &sample_job()).expect("valid upsert");

        assert_eq!(upsert.progress_pct, Some(63));
        assert_eq!(upsert.stage_progress_current, Some(5));
        assert_eq!(upsert.stage_progress_total, Some(8));
        assert_eq!(upsert.stage_progress_pct, Some(62));
    }
}
