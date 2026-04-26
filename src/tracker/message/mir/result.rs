use nauron_contracts::{MirResult, MirStage};
use tracing::{info, warn};

use crate::{
    artifacts::select_document_artifact,
    db::{
        files::FileRepository,
        jobs::{JobEngine, JobRecord, JobRepository, JobSnapshotUpsert, JobStatus},
    },
    kafka::KafkaPublisher,
    metrics::GatewayMetrics,
    tracker::{TrackerError, propagate::propagate_success},
};

pub(super) async fn handle_result_event(
    job_repo: &JobRepository,
    file_repo: &FileRepository,
    rdf_publisher: &KafkaPublisher,
    metrics: &GatewayMetrics,
    record: JobRecord,
    result: MirResult,
) -> Result<(), TrackerError> {
    let Some(file_id) = record.file_id else {
        return Ok(());
    };

    match &result {
        MirResult::Success {
            artifacts,
            completed_at,
            ..
        } => {
            if let Some(artifact) = select_document_artifact(artifacts) {
                let artifact_uri = format!("s3://{}/{}", artifact.bucket, artifact.key);
                let updated = file_repo
                    .mark_success(file_id, &artifact_uri, None, *completed_at)
                    .await?;
                metrics.record_mir_success(file_id, record.context_id);
                info!(
                    file_id,
                    context_id = record.context_id,
                    job_id = %record.job_id,
                    artifact_uri = artifact_uri.as_str(),
                    sha256 = %hex::encode(&updated.sha256),
                    "MIR job succeeded"
                );
                propagate_success(
                    job_repo,
                    file_repo,
                    rdf_publisher,
                    &updated,
                    &record,
                    &result,
                    artifact,
                )
                .await?;
            } else {
                mark_missing_artifact_failure(
                    job_repo,
                    file_repo,
                    metrics,
                    &record,
                    &result,
                    file_id,
                    *completed_at,
                )
                .await?;
            }
        }
        MirResult::Failure { .. } => {
            let failed = file_repo.mark_failure(file_id).await?;
            metrics.record_mir_failure(file_id, record.context_id);
            info!(
                file_id,
                context_id = record.context_id,
                job_id = %record.job_id,
                sha256 = %hex::encode(&failed.sha256),
                "MIR job failed"
            );
        }
        MirResult::Retryable { .. } => {
            info!(
                file_id,
                context_id = record.context_id,
                job_id = %record.job_id,
                "MIR job marked retryable"
            );
        }
    }

    Ok(())
}

async fn mark_missing_artifact_failure(
    job_repo: &JobRepository,
    file_repo: &FileRepository,
    metrics: &GatewayMetrics,
    record: &JobRecord,
    result: &MirResult,
    file_id: i64,
    occurred_at: chrono::DateTime<chrono::Utc>,
) -> Result<(), TrackerError> {
    let updated_record = overwrite_mir_job_as_failure(
        job_repo,
        record,
        result,
        "mir success missing document artifact",
        occurred_at,
    )
    .await?;
    let failed = file_repo.mark_failure(file_id).await?;
    metrics.record_mir_failure(file_id, record.context_id);
    warn!(
        file_id = failed.id,
        context_id = record.context_id,
        job_id = %updated_record.job_id,
        sha256 = %hex::encode(&failed.sha256),
        "MIR success payload missing document artifact; recorded failure"
    );
    Ok(())
}

async fn overwrite_mir_job_as_failure(
    job_repo: &JobRepository,
    record: &JobRecord,
    result: &MirResult,
    message: &str,
    occurred_at: chrono::DateTime<chrono::Utc>,
) -> Result<JobRecord, TrackerError> {
    let updated = job_repo
        .upsert_snapshot(JobSnapshotUpsert {
            job_id: record.job_id,
            context_id: record.context_id,
            file_id: record.file_id,
            pipeline_id: Some(record.pipeline_id),
            source_job_id: record.source_job_id,
            engine: JobEngine::Mir,
            kind: record.kind,
            status: JobStatus::Failure,
            stage: Some(MirStage::Completed.into()),
            progress_pct: Some(100),
            stage_progress_current: record.stage_progress_current,
            stage_progress_total: record.stage_progress_total,
            stage_progress_pct: record.stage_progress_pct,
            message: Some(message.into()),
            result_json: Some(serde_json::to_value(result)?),
            updated_at: occurred_at,
        })
        .await?;
    if updated.source_job_id.is_none() {
        job_repo.sync_linked_to_source(&updated).await?;
    }
    Ok(updated)
}
