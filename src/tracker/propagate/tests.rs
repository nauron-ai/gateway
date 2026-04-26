use chrono::Utc;
use uuid::Uuid;

use crate::db::jobs::{JobEngine, JobKind, JobRecord, JobStatus};
use crate::job_mode::JobLaunchMode;
use crate::test_utils::parse_uuid;

use super::{MirSyncTarget, select_synced_mir_target};

fn mir_job(job_id: &str, status: JobStatus, kind: Option<JobKind>) -> JobRecord {
    JobRecord {
        job_id: parse_uuid(job_id),
        context_id: 7,
        file_id: Some(11),
        pipeline_id: Uuid::nil(),
        source_job_id: (kind == JobLaunchMode::Linked.as_kind())
            .then(|| parse_uuid("cccccccc-cccc-cccc-cccc-cccccccccccc")),
        engine: JobEngine::Mir,
        kind,
        status,
        stage: None,
        progress_pct: None,
        stage_progress_current: None,
        stage_progress_total: None,
        stage_progress_pct: None,
        message: None,
        result_json: None,
        updated_at: Utc::now(),
    }
}

#[test]
fn synced_target_skips_retired_rows() {
    let retired_job_id = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    let target = select_synced_mir_target(&[mir_job(retired_job_id, JobStatus::Retired, None)]);

    assert_eq!(target.kind, Some(JobKind::Reused));
    assert_ne!(target.job_id, parse_uuid(retired_job_id));
    assert_eq!(target.source_job_id, None);
}

#[test]
fn synced_target_preserves_linked_identity_when_only_linked_job_exists() {
    let linked_job_id = parse_uuid("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb");
    let source_job_id = parse_uuid("cccccccc-cccc-cccc-cccc-cccccccccccc");
    let target = select_synced_mir_target(&[mir_job(
        "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
        JobStatus::Pending,
        JobLaunchMode::Linked.as_kind(),
    )]);

    assert_eq!(
        target,
        MirSyncTarget {
            job_id: linked_job_id,
            kind: Some(JobKind::MirLinked),
            source_job_id: Some(source_job_id),
        }
    );
}

#[test]
fn synced_target_prefers_active_canonical_job_over_linked_snapshot() {
    let canonical_job_id = parse_uuid("dddddddd-dddd-dddd-dddd-dddddddddddd");
    let target = select_synced_mir_target(&[
        mir_job(
            "dddddddd-dddd-dddd-dddd-dddddddddddd",
            JobStatus::InProgress,
            None,
        ),
        mir_job(
            "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
            JobStatus::Pending,
            JobLaunchMode::Linked.as_kind(),
        ),
    ]);

    assert_eq!(
        target,
        MirSyncTarget {
            job_id: canonical_job_id,
            kind: Some(JobKind::Reused),
            source_job_id: None,
        }
    );
}
