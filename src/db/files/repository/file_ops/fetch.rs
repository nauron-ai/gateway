use std::collections::HashMap;
use uuid::Uuid;

use super::super::{CreateFileParams, FileRecord, FileRepository, FileStatus};

impl FileRepository {
    pub async fn create_or_get_by_hash(
        &self,
        params: CreateFileParams<'_>,
    ) -> Result<FileRecord, sqlx::Error> {
        let inserted = sqlx::query_as!(
            FileRecord,
            r#"
            INSERT INTO files (sha256, size_bytes, mime, storage_bucket, storage_key)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (sha256) DO NOTHING
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
            params.sha256,
            params.size_bytes,
            params.mime,
            params.storage_bucket,
            params.storage_key
        )
        .fetch_optional(self.pool())
        .await?;

        if let Some(record) = inserted {
            return Ok(record);
        }

        self.find_by_hash(params.sha256)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn find_by_hash(&self, sha256: &[u8]) -> Result<Option<FileRecord>, sqlx::Error> {
        sqlx::query_as!(
            FileRecord,
            r#"
            SELECT
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
            FROM files
            WHERE sha256 = $1
            "#,
            sha256
        )
        .fetch_optional(self.pool())
        .await
    }

    pub async fn find_by_id(&self, file_id: i64) -> Result<Option<FileRecord>, sqlx::Error> {
        sqlx::query_as!(
            FileRecord,
            r#"
            SELECT
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
            FROM files
            WHERE id = $1
            "#,
            file_id
        )
        .fetch_optional(self.pool())
        .await
    }

    pub async fn find_many_by_ids(
        &self,
        ids: &[i64],
    ) -> Result<HashMap<i64, FileRecord>, sqlx::Error> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let records = sqlx::query_as!(
            FileRecord,
            r#"
            SELECT
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
            FROM files
            WHERE id = ANY($1)
            "#,
            ids
        )
        .fetch_all(self.pool())
        .await?;
        Ok(records
            .into_iter()
            .map(|record| (record.id, record))
            .collect())
    }

    pub async fn find_by_doc_id(&self, doc_id: Uuid) -> Result<Option<FileRecord>, sqlx::Error> {
        sqlx::query_as!(
            FileRecord,
            r#"
            SELECT
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
            FROM files
            WHERE doc_id = $1
            "#,
            doc_id
        )
        .fetch_optional(self.pool())
        .await
    }
}
