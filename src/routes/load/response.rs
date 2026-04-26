use serde::Serialize;
use uuid::Uuid;

use crate::job_mode::JobLaunchMode;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub(crate) struct LoadContextResponse {
    pub pipeline_id: Uuid,
    pub job_id: Uuid,
    pub doc_id: Uuid,
    pub context_id: i32,
    pub file_id: i64,
    pub sha256_hex: String,
    pub deduplicated: bool,
    pub job_mode: JobLaunchMode,
    pub source: UploadedLocation,
    pub topics: TopicInfo,
    pub status_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_status_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_path: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<LoadEntry>,
}

impl LoadContextResponse {
    pub fn from_entries(entries: Vec<LoadEntry>) -> Self {
        let primary = entries
            .first()
            .cloned()
            .unwrap_or_else(LoadEntry::placeholder);
        Self {
            pipeline_id: primary.pipeline_id,
            job_id: primary.job_id,
            doc_id: primary.doc_id,
            context_id: primary.context_id,
            file_id: primary.file_id,
            sha256_hex: primary.sha256_hex.clone(),
            deduplicated: primary.deduplicated,
            job_mode: primary.job_mode,
            source: primary.source.clone(),
            topics: primary.topics.clone(),
            status_url: primary.pipeline_status_url.clone(),
            job_status_url: Some(primary.job_status_url()),
            original_path: primary.original_path.clone(),
            entries,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct LoadEntry {
    pub job_id: Uuid,
    pub doc_id: Uuid,
    pub pipeline_id: Uuid,
    pub context_id: i32,
    pub file_id: i64,
    pub sha256_hex: String,
    pub deduplicated: bool,
    pub job_mode: JobLaunchMode,
    pub source: UploadedLocation,
    pub topics: TopicInfo,
    pub pipeline_status_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_status_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_path: Option<String>,
}

impl LoadEntry {
    fn placeholder() -> Self {
        Self {
            job_id: Uuid::nil(),
            doc_id: Uuid::nil(),
            pipeline_id: Uuid::nil(),
            context_id: 0,
            file_id: 0,
            sha256_hex: String::new(),
            deduplicated: false,
            job_mode: JobLaunchMode::Started,
            source: UploadedLocation {
                bucket: String::new(),
                key: String::new(),
            },
            topics: TopicInfo {
                progress: String::new(),
                result: String::new(),
                rdf_progress: String::new(),
                rdf_result: String::new(),
            },
            pipeline_status_url: String::new(),
            job_status_url: None,
            original_path: None,
        }
    }

    pub fn job_status_url(&self) -> String {
        format!("/v1/jobs/{}", self.job_id)
    }
}

#[cfg(test)]
mod tests {
    use super::{LoadContextResponse, LoadEntry, TopicInfo, UploadedLocation};
    use crate::job_mode::JobLaunchMode;
    use uuid::Uuid;

    #[test]
    fn from_entries_promotes_doc_id_from_primary_entry() {
        let job_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").expect("uuid");
        let doc_id = Uuid::parse_str("22222222-2222-2222-2222-222222222222").expect("uuid");
        let response = LoadContextResponse::from_entries(vec![LoadEntry {
            job_id,
            doc_id,
            pipeline_id: Uuid::nil(),
            context_id: 7,
            file_id: 11,
            sha256_hex: "abc".into(),
            deduplicated: false,
            job_mode: JobLaunchMode::Started,
            source: UploadedLocation {
                bucket: "bucket".into(),
                key: "key".into(),
            },
            topics: TopicInfo {
                progress: "progress".into(),
                result: "result".into(),
                rdf_progress: "rdf-progress".into(),
                rdf_result: "rdf-result".into(),
            },
            pipeline_status_url: "/v1/pipelines/1".into(),
            job_status_url: Some(format!("/v1/jobs/{job_id}")),
            original_path: None,
        }]);

        assert_eq!(response.job_id, job_id);
        assert_eq!(response.doc_id, doc_id);
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct UploadedLocation {
    pub bucket: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct TopicInfo {
    pub progress: String,
    pub result: String,
    pub rdf_progress: String,
    pub rdf_result: String,
}

#[derive(Debug, Clone, ToSchema)]
pub(crate) struct LoadContextForm {
    /// Plik do przetworzenia.
    #[schema(value_type = String, format = Binary)]
    pub file: String,
    /// Target context identifier.
    #[schema(value_type = i32, minimum = 1)]
    pub context_id: i32,
    /// Optional user identifier.
    #[schema(nullable = true, example = "user-42")]
    pub user_id: Option<String>,
    /// When `true`, only preliminary stages run without full RDF processing.
    #[schema(nullable = true, example = false)]
    pub dry_run: Option<bool>,
}
