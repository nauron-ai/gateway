use chrono::Utc;
use nauron_contracts::{FailureKind, MirProgress, MirResult, MirStage, SchemaVersion};
use uuid::Uuid;

use crate::db::jobs::{JobEngine, JobRecord, JobStatus};
use crate::test_utils::parse_uuid;

use super::upsert::{build_mir_progress_upsert, build_mir_result_upsert};

fn sample_job() -> JobRecord {
    JobRecord {
        job_id: parse_uuid("11111111-1111-1111-1111-111111111111"),
        context_id: 7,
        file_id: Some(11),
        pipeline_id: Uuid::nil(),
        source_job_id: None,
        engine: JobEngine::Mir,
        kind: None,
        status: JobStatus::InProgress,
        stage: None,
        progress_pct: Some(40),
        stage_progress_current: None,
        stage_progress_total: None,
        stage_progress_pct: None,
        message: None,
        result_json: None,
        updated_at: Utc::now(),
    }
}

#[test]
fn failure_result_maps_to_failure_job_status() {
    let result = MirResult::Failure {
        schema_version: SchemaVersion::V1,
        job_id: parse_uuid("11111111-1111-1111-1111-111111111111"),
        context_id: 7,
        kind: FailureKind::Input,
        message: "bad input".into(),
        details: Some("textract=invalid_image_type".into()),
        occurred_at: Utc::now(),
    };

    let upsert = build_mir_result_upsert(result, &sample_job()).expect("valid upsert");

    assert_eq!(upsert.status, JobStatus::Failure);
    assert_eq!(upsert.message.as_deref(), Some("bad input"));
}

#[test]
fn retryable_result_maps_to_retryable_job_status() {
    let result = MirResult::Retryable {
        schema_version: SchemaVersion::V1,
        job_id: parse_uuid("11111111-1111-1111-1111-111111111111"),
        context_id: 7,
        kind: FailureKind::Upstream,
        message: "throttled".into(),
        details: Some("retry".into()),
        occurred_at: Utc::now(),
    };

    let upsert = build_mir_result_upsert(result, &sample_job()).expect("valid upsert");

    assert_eq!(upsert.status, JobStatus::Retryable);
    assert_eq!(upsert.message.as_deref(), Some("throttled"));
}

#[test]
fn progress_upsert_uses_current_context_id() {
    let upsert = build_mir_progress_upsert(
        MirProgress {
            schema_version: SchemaVersion::V1,
            job_id: parse_uuid("11111111-1111-1111-1111-111111111111"),
            context_id: 99,
            stage: MirStage::Detect,
            percent: 55,
            message: Some("halfway".into()),
            timestamp: Utc::now(),
        },
        &sample_job(),
    )
    .expect("valid upsert");

    assert_eq!(upsert.context_id, 7);
}

#[test]
fn result_upsert_uses_current_context_id() {
    let result = MirResult::Success {
        schema_version: SchemaVersion::V1,
        job_id: parse_uuid("11111111-1111-1111-1111-111111111111"),
        context_id: 99,
        artifacts: vec![],
        stats: None,
        completed_at: Utc::now(),
    };

    let upsert = build_mir_result_upsert(result, &sample_job()).expect("valid upsert");

    assert_eq!(upsert.context_id, 7);
}
