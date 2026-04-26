use nauron_contracts::{ArtifactRef, MirResult, RdfStart, SchemaVersion};
use uuid::Uuid;

pub fn build_rdf_start_for_context(
    result: &MirResult,
    context_id: i32,
    doc_id: Uuid,
    artifact: &ArtifactRef,
) -> Option<RdfStart> {
    let MirResult::Success {
        job_id: _,
        completed_at,
        ..
    } = result
    else {
        return None;
    };

    Some(RdfStart {
        schema_version: SchemaVersion::V1,
        job_id: Uuid::new_v4(),
        doc_id,
        context_id,
        text_uri: format!("s3://{}/{}", artifact.bucket, artifact.key),
        source_id: artifact.key.clone(),
        requested_at: Some(*completed_at),
    })
}

pub fn mir_result_job_id(result: &MirResult) -> Uuid {
    match result {
        MirResult::Success { job_id, .. }
        | MirResult::Failure { job_id, .. }
        | MirResult::Retryable { job_id, .. } => *job_id,
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use nauron_contracts::{ArtifactRef, MirResult};
    use uuid::Uuid;

    use super::build_rdf_start_for_context;

    #[test]
    fn uses_canonical_doc_id_for_rdf_start() {
        let job_id = Uuid::parse_str("88888888-8888-8888-8888-888888888888").expect("uuid");
        let doc_id = Uuid::parse_str("99999999-9999-9999-9999-999999999999").expect("uuid");
        let result = MirResult::Success {
            schema_version: nauron_contracts::SchemaVersion::V1,
            job_id,
            context_id: 7,
            artifacts: Vec::new(),
            stats: None,
            completed_at: Utc::now(),
        };
        let artifact = ArtifactRef {
            bucket: "mir-output".into(),
            key: "jobs/job-1/document.md".into(),
            content_type: Some("text/markdown".into()),
            size_bytes: None,
        };

        let payload = build_rdf_start_for_context(&result, 42, doc_id, &artifact).expect("payload");

        assert_eq!(payload.doc_id, doc_id);
        assert_eq!(payload.context_id, 42);
    }
}
