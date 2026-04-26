use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct FileRecord {
    pub id: i64,
    pub doc_id: Option<Uuid>,
    pub sha256: Vec<u8>,
    pub size_bytes: i64,
    pub mime: Option<String>,
    pub storage_bucket: String,
    pub storage_key: String,
    pub status: FileStatus,
    pub mir_job_id: Option<Uuid>,
    pub mir_artifact_uri: Option<String>,
    pub mir_artifact_sha256: Option<Vec<u8>>,
    pub mir_processed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ContextFileRecord {
    pub id: i64,
    pub context_id: i32,
    pub file_id: i64,
    pub pipeline_id: Uuid,
    pub origin: FileOrigin,
    pub original_name: String,
    pub original_path: Option<String>,
    pub media_type: Option<String>,
    pub attached_at: DateTime<Utc>,
    pub file_sha256: Vec<u8>,
    pub file_status: FileStatus,
    pub mir_artifact_uri: Option<String>,
    pub doc_id: Option<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "file_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum FileStatus {
    Pending,
    Processing,
    Success,
    Failure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "file_origin", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum FileOrigin {
    Upload,
    ArchiveEntry,
}

#[derive(Debug)]
pub struct CreateFileParams<'a> {
    pub sha256: &'a [u8],
    pub size_bytes: i64,
    pub mime: Option<&'a str>,
    pub storage_bucket: &'a str,
    pub storage_key: &'a str,
}

#[derive(Debug)]
pub struct AttachContextFileParams<'a> {
    pub context_id: i32,
    pub file_id: i64,
    pub pipeline_id: Uuid,
    pub origin: FileOrigin,
    pub original_name: &'a str,
    pub original_path: Option<&'a str>,
    pub media_type: Option<&'a str>,
}
