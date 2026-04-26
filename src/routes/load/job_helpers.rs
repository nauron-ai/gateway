use std::sync::Arc;

use chrono::Utc;
use nauron_contracts::{MirRequest, MirResult, MirStage, OutputTarget, SchemaVersion, SourceRef};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    artifacts::select_document_artifact,
    config::AppConfig,
    db::{
        files::FileRecord,
        jobs::{JobEngine, JobKind, JobSnapshotUpsert, JobStatus},
    },
    error::GatewayError,
    job_mode::JobLaunchMode,
    state::AppState,
    tracker::rdf::build_rdf_start_for_context,
};

pub(crate) async fn reuse_mir_result(
    state: &Arc<AppState>,
    file: &FileRecord,
    context_id: i32,
    pipeline_id: Uuid,
) -> Result<Option<Uuid>, GatewayError> {
    let source_job_id = match &file.mir_job_id {
        Some(id) => id,
        None => return Ok(None),
    };
    let Some(template) = state.job_repo.get(*source_job_id).await? else {
        warn!(
            file_id = file.id,
            job_id = %source_job_id,
            "cannot reuse MIR result: original job missing"
        );
        return Ok(None);
    };
    let Some(value) = template.result_json.clone() else {
        warn!(
            file_id = file.id,
            job_id = %source_job_id,
            "cannot reuse MIR result: job missing payload"
        );
        return Ok(None);
    };
    let mut result: MirResult = serde_json::from_value(value)?;
    match &mut result {
        MirResult::Success {
            job_id,
            context_id: ctx,
            artifacts,
            ..
        } => {
            let new_job_id = Uuid::new_v4();
            let artifacts_copy = artifacts.clone();
            *job_id = new_job_id;
            *ctx = context_id;
            let snapshot = JobSnapshotUpsert {
                job_id: new_job_id,
                context_id,
                file_id: Some(file.id),
                engine: JobEngine::Mir,
                pipeline_id: Some(pipeline_id),
                source_job_id: None,
                kind: JobLaunchMode::Reused.as_kind(),
                status: JobStatus::Success,
                stage: Some(MirStage::Completed.into()),
                progress_pct: Some(100),
                stage_progress_current: None,
                stage_progress_total: None,
                stage_progress_pct: None,
                message: Some("mir result reused".into()),
                result_json: Some(serde_json::to_value(&result)?),
                updated_at: file.mir_processed_at.unwrap_or_else(Utc::now),
            };
            state.job_repo.upsert_snapshot(snapshot).await?;
            state.metrics.record_mir_reused(file.id, context_id);
            info!(
                file_id = file.id,
                context_id,
                job_id = %new_job_id,
                source_job_id = %source_job_id,
                "reused MIR artifacts"
            );

            if !state
                .job_repo
                .exists_for_file_engine_context(file.id, context_id, JobEngine::Rdf)
                .await?
            {
                if let Some(artifact) = select_document_artifact(&artifacts_copy) {
                    if let Some(payload) = build_rdf_start_for_context(
                        &result,
                        context_id,
                        file.doc_id.unwrap_or(*source_job_id),
                        artifact,
                    ) {
                        state
                            .job_repo
                            .upsert_snapshot(JobSnapshotUpsert {
                                job_id: payload.job_id,
                                context_id,
                                file_id: Some(file.id),
                                pipeline_id: Some(pipeline_id),
                                source_job_id: Some(new_job_id),
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
                                updated_at: Utc::now(),
                            })
                            .await?;

                        state
                            .rdf_publisher
                            .publish_json(payload.job_id, &payload)
                            .await?;

                        info!(
                            file_id = file.id,
                            context_id,
                            job_id = %payload.job_id,
                            source_job_id = %source_job_id,
                            "started RDF job for reused MIR result"
                        );
                    }
                } else {
                    warn!(
                        file_id = file.id,
                        context_id,
                        job_id = %new_job_id,
                        "skipping RDF fanout: no document artifact in reused MIR result"
                    );
                }
            }

            Ok(Some(new_job_id))
        }
        _ => {
            warn!(
                file_id = file.id,
                job_id = %source_job_id,
                "cannot reuse MIR result: source job not successful"
            );
            Ok(None)
        }
    }
}

pub(crate) async fn create_linked_job(
    state: &Arc<AppState>,
    file: &FileRecord,
    context_id: i32,
    pipeline_id: Uuid,
) -> Result<Option<Uuid>, GatewayError> {
    let source_job_id = match &file.mir_job_id {
        Some(id) => id,
        None => return Ok(None),
    };
    let Some(template) = state.job_repo.get(*source_job_id).await? else {
        warn!(
            file_id = file.id,
            job_id = %source_job_id,
            "cannot link MIR job: original job missing"
        );
        return Ok(None);
    };
    let job_id = Uuid::new_v4();
    let snapshot = JobSnapshotUpsert {
        job_id,
        context_id,
        file_id: Some(file.id),
        engine: JobEngine::Mir,
        pipeline_id: Some(pipeline_id),
        source_job_id: Some(*source_job_id),
        kind: JobLaunchMode::Linked.as_kind(),
        status: template.status,
        stage: template.stage,
        progress_pct: template.progress_pct,
        stage_progress_current: template.stage_progress_current,
        stage_progress_total: template.stage_progress_total,
        stage_progress_pct: template.stage_progress_pct,
        message: template.message.clone(),
        result_json: template.result_json.clone(),
        updated_at: template.updated_at,
    };
    let record = state.job_repo.upsert_linked_snapshot(snapshot).await?;
    if record.job_id == job_id {
        state.metrics.record_mir_linked(file.id, context_id);
    }
    info!(
        file_id = file.id,
        context_id,
        job_id = %record.job_id,
        source_job_id = %source_job_id,
        "linked MIR job to existing progress"
    );
    Ok(Some(record.job_id))
}

pub(crate) fn build_request(
    config: &AppConfig,
    job_id: Uuid,
    bucket: String,
    key: String,
    context_id: i32,
    user_id: Option<String>,
    dry_run: bool,
) -> MirRequest {
    MirRequest {
        schema_version: SchemaVersion::V1,
        job_id,
        context_id,
        user_id,
        source: SourceRef::S3 {
            bucket,
            key,
            version_id: None,
        },
        output: OutputTarget {
            bucket: config.output_bucket.clone(),
            prefix: Some(format!("{}/{}", config.output_prefix, job_id)),
        },
        dry_run,
        attempt: 1,
        submitted_at: Some(Utc::now()),
    }
}
