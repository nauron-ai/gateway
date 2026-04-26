use tokio::time::{Duration, interval};

use crate::db::jobs::JobRepository;

const CLEANUP_INTERVAL_SECS: u64 = 3600;

pub fn spawn_ingest_job_cleanup(job_repo: JobRepository) {
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(CLEANUP_INTERVAL_SECS));
        loop {
            ticker.tick().await;
            let _ = job_repo.delete_expired_ingest_jobs().await;
        }
    });
}
