use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::super::{FileRecord, FileRepository, FileStatus};

impl FileRepository {
    pub async fn mark_processing(
        &self,
        file_id: i64,
        mir_job_id: Uuid,
    ) -> Result<FileRecord, sqlx::Error> {
        sqlx::query_as!(
            FileRecord,
            r#"
            UPDATE files
            SET status = 'processing'::file_status,
                mir_job_id = $2,
                doc_id = COALESCE(doc_id, $2),
                updated_at = now()
            WHERE id = $1
            RETURNING
                id,
                doc_id,
                sha256,
                size_bytes,
                mime,
                storage_bucket,
                storage_key,
                status as "status: FileStatus",
                mir_job_id,
                mir_artifact_uri,
                mir_artifact_sha256,
                mir_processed_at,
                created_at,
                updated_at
            "#,
            file_id,
            mir_job_id
        )
        .fetch_one(self.pool())
        .await
    }

    pub async fn mark_success(
        &self,
        file_id: i64,
        artifact_uri: &str,
        artifact_sha256: Option<&[u8]>,
        processed_at: DateTime<Utc>,
    ) -> Result<FileRecord, sqlx::Error> {
        sqlx::query_as!(
            FileRecord,
            r#"
            UPDATE files
            SET status = 'success'::file_status,
                mir_artifact_uri = $2,
                mir_artifact_sha256 = $3,
                mir_processed_at = $4,
                updated_at = now()
            WHERE id = $1
            RETURNING
                id,
                doc_id,
                sha256,
                size_bytes,
                mime,
                storage_bucket,
                storage_key,
                status as "status: FileStatus",
                mir_job_id,
                mir_artifact_uri,
                mir_artifact_sha256,
                mir_processed_at,
                created_at,
                updated_at
            "#,
            file_id,
            artifact_uri,
            artifact_sha256,
            processed_at
        )
        .fetch_one(self.pool())
        .await
    }

    pub async fn mark_failure(&self, file_id: i64) -> Result<FileRecord, sqlx::Error> {
        sqlx::query_as!(
            FileRecord,
            r#"
            UPDATE files
            SET status = 'failure'::file_status,
                updated_at = now()
            WHERE id = $1
            RETURNING
                id,
                doc_id,
                sha256,
                size_bytes,
                mime,
                storage_bucket,
                storage_key,
                status as "status: FileStatus",
                mir_job_id,
                mir_artifact_uri,
                mir_artifact_sha256,
                mir_processed_at,
                created_at,
                updated_at
            "#,
            file_id
        )
        .fetch_one(self.pool())
        .await
    }

    pub async fn reset_pending(&self, file_id: i64) -> Result<FileRecord, sqlx::Error> {
        sqlx::query_as!(
            FileRecord,
            r#"
            UPDATE files
            SET status = 'pending'::file_status,
                mir_job_id = NULL,
                updated_at = now()
            WHERE id = $1
            RETURNING
                id,
                doc_id,
                sha256,
                size_bytes,
                mime,
                storage_bucket,
                storage_key,
                status as "status: FileStatus",
                mir_job_id,
                mir_artifact_uri,
                mir_artifact_sha256,
                mir_processed_at,
                created_at,
                updated_at
            "#,
            file_id
        )
        .fetch_one(self.pool())
        .await
    }
}
