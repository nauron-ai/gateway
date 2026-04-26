use std::collections::HashSet;

use nauron_contracts::{ArtifactRef, MirResult, MirStage};
use uuid::Uuid;

use crate::db::files::{ContextPipelineRef, FileRecord, FileRepository};
use crate::db::jobs::{JobEngine, JobKind, JobRecord, JobRepository, JobSnapshotUpsert, JobStatus};
use crate::job_mode::JobLaunchMode;
use crate::kafka::KafkaPublisher;

use super::{
    TrackerError,
    rdf::{build_rdf_start_for_context, mir_result_job_id},
};

pub async fn propagate_success(
    job_repo: &JobRepository,
    file_repo: &FileRepository,
    rdf_publisher: &KafkaPublisher,
    file: &FileRecord,
    record: &JobRecord,
    result: &MirResult,
    artifact: &ArtifactRef,
) -> Result<(), TrackerError> {
    let contexts = file_repo.list_context_ids_by_file(file.id).await?;
    let canonical_doc_id = file.doc_id.unwrap_or_else(|| mir_result_job_id(result));
    let mut rdf_handled = HashSet::new();

    for ContextPipelineRef {
        context_id,
        pipeline_id,
    } in contexts
    {
        let mut synced_result = None;
        if context_id != record.context_id {
            synced_result = Some(
                sync_mir_job(job_repo, file.id, context_id, pipeline_id, record, result).await?,
            );
        }

        if rdf_handled.insert(context_id) {
            let mir_for_rdf = synced_result.as_ref().unwrap_or(result);
            ensure_rdf_job(
                job_repo,
                rdf_publisher,
                file,
                RdfFanoutContext {
                    context_id,
                    pipeline_id,
                    canonical_doc_id,
                },
                mir_for_rdf,
                artifact,
            )
            .await?;
        }
    }

    Ok(())
}

struct RdfFanoutContext {
    context_id: i32,
    pipeline_id: Uuid,
    canonical_doc_id: Uuid,
}

async fn sync_mir_job(
    job_repo: &JobRepository,
    file_id: i64,
    context_id: i32,
    pipeline_id: uuid::Uuid,
    source_record: &JobRecord,
    result: &MirResult,
) -> Result<MirResult, TrackerError> {
    let existing = job_repo.list_by_pipeline(pipeline_id).await?;
    let target = select_synced_mir_target(&existing);

    let synced_result = sync_mir_success(result, target.job_id, context_id);
    let result_json = Some(serde_json::to_value(&synced_result)?);

    job_repo
        .upsert_snapshot(JobSnapshotUpsert {
            job_id: target.job_id,
            context_id,
            file_id: Some(file_id),
            pipeline_id: Some(pipeline_id),
            source_job_id: target.source_job_id,
            engine: JobEngine::Mir,
            kind: target.kind,
            status: JobStatus::Success,
            stage: Some(MirStage::Completed.into()),
            progress_pct: Some(100),
            stage_progress_current: None,
            stage_progress_total: None,
            stage_progress_pct: None,
            message: Some("mir result reused".into()),
            result_json,
            updated_at: source_record.updated_at,
        })
        .await?;
    Ok(synced_result)
}

fn sync_mir_success(source: &MirResult, job_id: Uuid, context_id: i32) -> MirResult {
    match source {
        MirResult::Success {
            schema_version,
            artifacts,
            stats,
            completed_at,
            ..
        } => MirResult::Success {
            schema_version: *schema_version,
            job_id,
            context_id,
            artifacts: artifacts.clone(),
            stats: stats.clone(),
            completed_at: *completed_at,
        },
        other => other.clone(),
    }
}

async fn ensure_rdf_job(
    job_repo: &JobRepository,
    rdf_publisher: &KafkaPublisher,
    file: &FileRecord,
    context: RdfFanoutContext,
    result: &MirResult,
    artifact: &ArtifactRef,
) -> Result<(), TrackerError> {
    if job_repo
        .exists_for_file_engine_context(file.id, context.context_id, JobEngine::Rdf)
        .await?
    {
        return Ok(());
    }

    let Some(payload) = build_rdf_start_for_context(
        result,
        context.context_id,
        context.canonical_doc_id,
        artifact,
    ) else {
        tracing::warn!(
            file_id = file.id,
            context_id = context.context_id,
            pipeline_id = %context.pipeline_id,
            "Skipping RDF fanout: MIR result invalid for RDF"
        );
        return Ok(());
    };

    job_repo
        .upsert_snapshot(JobSnapshotUpsert {
            job_id: payload.job_id,
            context_id: context.context_id,
            file_id: Some(file.id),
            pipeline_id: Some(context.pipeline_id),
            source_job_id: Some(mir_result_job_id(result)),
            engine: JobEngine::Rdf,
            kind: Some(JobKind::Fanout),
            status: JobStatus::Pending,
            stage: None,
            progress_pct: None,
            stage_progress_current: None,
            stage_progress_total: None,
            stage_progress_pct: None,
            message: Some("rdf pending".into()),
            result_json: None,
            updated_at: chrono::Utc::now(),
        })
        .await?;

    rdf_publisher
        .publish_json(payload.job_id, &payload)
        .await
        .map_err(TrackerError::Publisher)?;

    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct MirSyncTarget {
    job_id: Uuid,
    kind: Option<JobKind>,
    source_job_id: Option<Uuid>,
}

fn select_synced_mir_target(existing: &[JobRecord]) -> MirSyncTarget {
    let active_mir = existing
        .iter()
        .find(|job| {
            job.engine == JobEngine::Mir && job.status != JobStatus::Retired && !is_linked_mir(job)
        })
        .or_else(|| {
            existing
                .iter()
                .find(|job| job.engine == JobEngine::Mir && job.status != JobStatus::Retired)
        });

    match active_mir {
        Some(job) if is_linked_mir(job) => MirSyncTarget {
            job_id: job.job_id,
            kind: job.kind,
            source_job_id: job.source_job_id,
        },
        Some(job) => MirSyncTarget {
            job_id: job.job_id,
            kind: JobLaunchMode::Reused.as_kind(),
            source_job_id: None,
        },
        None => MirSyncTarget {
            job_id: Uuid::new_v4(),
            kind: JobLaunchMode::Reused.as_kind(),
            source_job_id: None,
        },
    }
}

fn is_linked_mir(job: &JobRecord) -> bool {
    job.kind == JobLaunchMode::Linked.as_kind()
}

#[cfg(test)]
mod tests;
