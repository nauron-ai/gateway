use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::{AttachContextFileParams, ContextFileRecord, FileOrigin, FileRepository};

mod attach;
mod lookup;

#[derive(Debug, Clone)]
pub struct ContextFileListParams {
    pub context_id: i32,
    pub limit: i64,
    pub cursor: Option<ContextFileListCursor>,
}

#[derive(Debug, Clone)]
pub struct ContextFileListCursor {
    pub attached_at: DateTime<Utc>,
    pub context_file_id: i64,
}

#[derive(Debug, Clone)]
pub struct ContextPipelineRef {
    pub context_id: i32,
    pub pipeline_id: Uuid,
}
