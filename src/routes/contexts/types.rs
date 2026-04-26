use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::db::{
    contexts::{ContextMode, ContextRecord},
    files::{ContextFileRecord, FileOrigin, FileStatus},
};

#[derive(Serialize, ToSchema)]
pub struct ContextResponse {
    pub id: i32,
    pub title: Option<String>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<Uuid>,
    pub files_count: i64,
    pub mode: ContextMode,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateContextRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub mode: Option<crate::db::contexts::ContextMode>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateContextRequest {
    #[serde(default)]
    pub mode: Option<crate::db::contexts::ContextMode>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ContextsQuery {
    pub limit: Option<i64>,
    pub cursor_created_at: Option<DateTime<Utc>>,
    pub cursor_id: Option<i32>,
}

#[derive(Serialize, ToSchema)]
pub struct ContextsResponse {
    pub contexts: Vec<ContextResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<ContextCursor>,
}

#[derive(Serialize, ToSchema)]
pub struct ContextCursor {
    pub created_at: DateTime<Utc>,
    pub id: i32,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct ContextFilesQuery {
    pub limit: Option<i64>,
    pub cursor_attached_at: Option<DateTime<Utc>>,
    pub cursor_id: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct ContextFilesResponse {
    pub files: Vec<ContextFileEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<ContextFileCursor>,
}

#[derive(Serialize, ToSchema)]
pub struct ContextFileEntry {
    pub context_file_id: i64,
    pub pipeline_id: uuid::Uuid,
    pub doc_id: Option<Uuid>,
    pub original_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    pub status: FileStatus,
    pub sha256_hex: String,
    pub origin: FileOrigin,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mir_artifact_uri: Option<String>,
    pub attached_at: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct ContextFileCursor {
    pub attached_at: DateTime<Utc>,
    pub context_file_id: i64,
}

impl From<ContextRecord> for ContextResponse {
    fn from(record: ContextRecord) -> Self {
        Self {
            id: record.id,
            title: record.title,
            description: record.description,
            created_at: record.created_at,
            updated_at: record.updated_at,
            owner_id: record.owner_id,
            files_count: record.files_count.unwrap_or(0),
            mode: record.mode,
        }
    }
}

impl ContextFileEntry {
    pub fn from_record(record: ContextFileRecord) -> Self {
        let sha256_hex = hex::encode(&record.file_sha256);
        Self {
            context_file_id: record.id,
            pipeline_id: record.pipeline_id,
            doc_id: record.doc_id,
            original_name: record.original_name,
            media_type: record.media_type,
            status: record.file_status,
            sha256_hex,
            origin: record.origin,
            mir_artifact_uri: record.mir_artifact_uri,
            attached_at: record.attached_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use super::ContextFileEntry;
    use crate::db::files::{ContextFileRecord, FileOrigin, FileStatus};

    #[test]
    fn from_record_preserves_doc_id() {
        let entry = ContextFileEntry::from_record(ContextFileRecord {
            id: 1,
            context_id: 7,
            file_id: 11,
            pipeline_id: Uuid::nil(),
            origin: FileOrigin::Upload,
            original_name: "contract.pdf".into(),
            original_path: None,
            media_type: Some("application/pdf".into()),
            attached_at: Utc::now(),
            file_sha256: vec![0xAB, 0xCD],
            file_status: FileStatus::Success,
            mir_artifact_uri: Some("s3://bucket/artifact.json".into()),
            doc_id: Some(Uuid::parse_str("33333333-3333-3333-3333-333333333333").expect("uuid")),
        });

        assert_eq!(
            entry.doc_id,
            Some(Uuid::parse_str("33333333-3333-3333-3333-333333333333").expect("uuid"))
        );
        assert_eq!(entry.sha256_hex, "abcd");
    }
}
