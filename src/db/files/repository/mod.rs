mod admin_ops;
mod context_ops;
mod file_ops;

pub use admin_ops::{AdminFileCursor, AdminFileListParams, AdminFileRecord};
pub use context_ops::{ContextFileListCursor, ContextFileListParams, ContextPipelineRef};

use sqlx::PgPool;

use super::{
    AttachContextFileParams, ContextFileRecord, CreateFileParams, FileOrigin, FileRecord,
    FileStatus,
};

#[derive(Clone)]
pub struct FileRepository {
    pool: PgPool,
}

impl FileRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl FileRepository {
    pub(super) fn pool(&self) -> &PgPool {
        &self.pool
    }
}
