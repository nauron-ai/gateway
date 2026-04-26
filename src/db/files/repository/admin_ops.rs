use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::{FileRepository, FileStatus};

#[derive(Debug, Clone)]
pub struct AdminFileListParams {
    pub limit: i64,
    pub status: Option<FileStatus>,
    pub context_id: Option<i32>,
    pub sha256_prefix: Option<String>,
    pub cursor: Option<AdminFileCursor>,
}

#[derive(Debug, Clone)]
pub struct AdminFileCursor {
    pub updated_at: DateTime<Utc>,
    pub file_id: i64,
}

#[derive(Debug, Clone)]
pub struct AdminFileRecord {
    pub file_id: i64,
    pub sha256: Vec<u8>,
    pub size_bytes: i64,
    pub status: FileStatus,
    pub mir_job_id: Option<Uuid>,
    pub mir_artifact_uri: Option<String>,
    pub contexts_count: i64,
    pub contexts: Vec<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl FileRepository {
    pub async fn list_files_for_admin(
        &self,
        params: AdminFileListParams,
    ) -> Result<Vec<AdminFileRecord>, sqlx::Error> {
        let cursor_updated_at = params.cursor.as_ref().map(|cursor| cursor.updated_at);
        let cursor_file_id = params.cursor.as_ref().map(|cursor| cursor.file_id);
        let sha_filter = params.sha256_prefix.as_deref();
        sqlx::query_as!(
            AdminFileRecord,
            r#"
            SELECT
                f.id AS file_id,
                f.sha256,
                f.size_bytes,
                f.status as "status: FileStatus",
                f.mir_job_id,
                f.mir_artifact_uri,
                f.created_at,
                f.updated_at,
                COALESCE((
                    SELECT COUNT(*)::bigint
                    FROM context_files cf_total
                    WHERE cf_total.file_id = f.id
                ), 0::bigint) as "contexts_count!",
                COALESCE((
                    SELECT ARRAY(
                        SELECT cf_preview.context_id
                        FROM context_files cf_preview
                        WHERE cf_preview.file_id = f.id
                        ORDER BY cf_preview.context_id DESC
                        LIMIT 10
                    )
                ), '{}'::int4[]) as "contexts!: Vec<i32>"
            FROM files f
            WHERE ($2::file_status IS NULL OR f.status = $2::file_status)
              AND ($3::text IS NULL OR encode(f.sha256, 'hex') ILIKE $3)
              AND ($4::int4 IS NULL OR EXISTS (
                    SELECT 1
                    FROM context_files cf_filter
                    WHERE cf_filter.file_id = f.id
                      AND cf_filter.context_id = $4
                ))
              AND (
                $5::timestamptz IS NULL
                OR (f.updated_at, f.id) < ($5, $6)
              )
            ORDER BY f.updated_at DESC, f.id DESC
            LIMIT $1
            "#,
            params.limit,
            params.status as Option<FileStatus>,
            sha_filter,
            params.context_id,
            cursor_updated_at,
            cursor_file_id
        )
        .fetch_all(self.pool())
        .await
    }
}
