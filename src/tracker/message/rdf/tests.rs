use chrono::Utc;
use nauron_contracts::{FailureKind, RdfProgress, RdfResult, RdfStage, SchemaVersion};
use serde_json::json;
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

fn sample_progress() -> RdfProgress {
    RdfProgress {
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
    let upsert = build_rdf_progress_upsert(sample_progress(), &sample_job()).expect("valid upsert");

    assert_eq!(upsert.progress_pct, Some(63));
    assert_eq!(upsert.stage_progress_current, Some(5));
    assert_eq!(upsert.stage_progress_total, Some(8));
    assert_eq!(upsert.stage_progress_pct, Some(62));
}

#[test]
fn progress_upsert_does_not_downgrade_terminal_status() {
    let mut current = sample_job();
    current.status = JobStatus::Success;
    current.message = Some("done".into());
    current.result_json = Some(json!({"status": "success"}));

    let upsert = build_rdf_progress_upsert(sample_progress(), &current).expect("valid upsert");

    assert_eq!(upsert.status, JobStatus::Success);
    assert_eq!(upsert.message.as_deref(), Some("done"));
    assert_eq!(upsert.result_json, Some(json!({"status": "success"})));
}
