use std::sync::Arc;

use tracing::info;
use uuid::Uuid;

use crate::db::files::FileRecord;
use crate::error::GatewayError;
use crate::job_mode::JobLaunchMode;
use crate::state::AppState;

use super::job_helpers::{build_request, create_linked_job, reuse_mir_result};
pub use super::job_rdf::start_rdf_job;

pub async fn decide_job_action(
    state: &Arc<AppState>,
    file_record: &mut FileRecord,
    context_id: i32,
    pipeline_id: Uuid,
    user_id: Option<String>,
    dry_run: bool,
) -> Result<(Uuid, JobLaunchMode), GatewayError> {
    if state.config.files_dedup_enabled {
        if file_record.status == crate::db::files::FileStatus::Success {
            if let Some(job_id) =
                reuse_mir_result(state, file_record, context_id, pipeline_id).await?
            {
                return Ok((job_id, JobLaunchMode::Reused));
            }
        } else if let Some(linked) =
            create_linked_job(state, file_record, context_id, pipeline_id).await?
        {
            return Ok((linked, JobLaunchMode::Linked));
        }
    }

    let (job_id, refreshed_file) = start_mir_job(
        state,
        file_record,
        context_id,
        pipeline_id,
        user_id,
        dry_run,
    )
    .await?;
    *file_record = refreshed_file;
    Ok((job_id, JobLaunchMode::Started))
}

pub async fn start_mir_job(
    state: &Arc<AppState>,
    file: &FileRecord,
    context_id: i32,
    pipeline_id: Uuid,
    user_id: Option<String>,
    dry_run: bool,
) -> Result<(Uuid, FileRecord), GatewayError> {
    let job_id = Uuid::new_v4();
    state
        .tracker
        .register_pending(
            job_id,
            context_id,
            pipeline_id,
            Some(file.id),
            JobLaunchMode::Started,
        )
        .await?;
    let updated_file = state.file_repo.mark_processing(file.id, job_id).await?;
    state.metrics.record_mir_started(file.id, context_id);
    info!(
        file_id = file.id,
        context_id,
        job_id = %job_id,
        "started MIR job"
    );
    let request = build_request(
        &state.config,
        job_id,
        updated_file.storage_bucket.clone(),
        updated_file.storage_key.clone(),
        context_id,
        user_id,
        dry_run,
    );
    state.mir_publisher.publish_request(&request).await?;
    Ok((job_id, updated_file))
}
