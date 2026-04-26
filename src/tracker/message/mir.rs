mod result;
#[cfg(test)]
mod tests;
mod upsert;

use nauron_contracts::MirEvent;

use crate::{
    db::{files::FileRepository, jobs::JobRepository},
    kafka::KafkaPublisher,
    metrics::GatewayMetrics,
    tracker::TrackerError,
};

use super::lookup_job;
use result::handle_result_event;
use upsert::{build_mir_progress_upsert, build_mir_result_upsert, result_job_id};

pub(super) async fn handle_mir_event(
    job_repo: &JobRepository,
    file_repo: &FileRepository,
    rdf_publisher: &KafkaPublisher,
    metrics: &GatewayMetrics,
    event: MirEvent,
) -> Result<(), TrackerError> {
    let record = match &event {
        MirEvent::Progress(progress) => {
            let current = lookup_job(job_repo, progress.job_id).await?;
            job_repo
                .upsert_snapshot(build_mir_progress_upsert(progress.clone(), &current)?)
                .await?
        }
        MirEvent::Result(result) => {
            let current = lookup_job(job_repo, result_job_id(result)).await?;
            job_repo
                .upsert_snapshot(build_mir_result_upsert(result.clone(), &current)?)
                .await?
        }
    };
    if record.source_job_id.is_none() {
        job_repo.sync_linked_to_source(&record).await?;
    }

    match event {
        MirEvent::Progress(_) => {
            if let Some(file_id) = record.file_id {
                file_repo.mark_processing(file_id, record.job_id).await?;
            }
        }
        MirEvent::Result(result) => {
            handle_result_event(job_repo, file_repo, rdf_publisher, metrics, record, result)
                .await?;
        }
    }
    Ok(())
}
