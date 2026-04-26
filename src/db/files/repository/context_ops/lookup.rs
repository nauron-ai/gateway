use super::super::FileStatus;
use super::{ContextFileRecord, ContextPipelineRef, FileOrigin, FileRepository};

impl FileRepository {
    pub async fn list_context_ids_by_file(
        &self,
        file_id: i64,
    ) -> Result<Vec<ContextPipelineRef>, sqlx::Error> {
        let rows = sqlx::query!(
            r#"
            SELECT context_id, pipeline_id
            FROM context_files
            WHERE file_id = $1
            "#,
            file_id
        )
        .fetch_all(self.pool())
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| ContextPipelineRef {
                context_id: row.context_id,
                pipeline_id: row.pipeline_id,
            })
            .collect())
    }

    pub async fn list_context_records_by_file(
        &self,
        file_id: i64,
    ) -> Result<Vec<ContextFileRecord>, sqlx::Error> {
        sqlx::query_as!(
            ContextFileRecord,
            r#"
            SELECT
                cf.id,
                cf.context_id,
                cf.file_id,
                cf.pipeline_id,
                cf.origin as "origin: FileOrigin",
                cf.original_name,
                cf.original_path,
                cf.media_type,
                cf.attached_at,
                f.sha256 as file_sha256,
                f.status as "file_status: FileStatus",
                f.mir_artifact_uri,
                f.doc_id as "doc_id?"
            FROM context_files cf
            INNER JOIN files f ON f.id = cf.file_id
            WHERE cf.file_id = $1
            ORDER BY cf.attached_at DESC, cf.id DESC
            "#,
            file_id
        )
        .fetch_all(self.pool())
        .await
    }

    pub async fn find_by_pipeline_id(
        &self,
        pipeline_id: uuid::Uuid,
    ) -> Result<Option<ContextFileRecord>, sqlx::Error> {
        sqlx::query_as!(
            ContextFileRecord,
            r#"
            SELECT
                cf.id,
                cf.context_id,
                cf.file_id,
                cf.pipeline_id,
                cf.origin as "origin: FileOrigin",
                cf.original_name,
                cf.original_path,
                cf.media_type,
                cf.attached_at,
                f.sha256 as file_sha256,
                f.status as "file_status: FileStatus",
                f.mir_artifact_uri,
                f.doc_id as "doc_id?"
            FROM context_files cf
            INNER JOIN files f ON f.id = cf.file_id
            WHERE cf.pipeline_id = $1
            "#,
            pipeline_id
        )
        .fetch_optional(self.pool())
        .await
    }

    pub async fn find_context_file_by_id(
        &self,
        id: i64,
    ) -> Result<Option<ContextFileRecord>, sqlx::Error> {
        sqlx::query_as!(
            ContextFileRecord,
            r#"
            SELECT
                cf.id,
                cf.context_id,
                cf.file_id,
                cf.pipeline_id,
                cf.origin as "origin: FileOrigin",
                cf.original_name,
                cf.original_path,
                cf.media_type,
                cf.attached_at,
                f.sha256 as file_sha256,
                f.status as "file_status: FileStatus",
                f.mir_artifact_uri,
                f.doc_id as "doc_id?"
            FROM context_files cf
            INNER JOIN files f ON f.id = cf.file_id
            WHERE cf.id = $1
            "#,
            id
        )
        .fetch_optional(self.pool())
        .await
    }
}

impl FileRepository {
    pub(super) async fn fetch_context_file_by_id(
        &self,
        id: i64,
    ) -> Result<ContextFileRecord, sqlx::Error> {
        self.find_context_file_by_id(id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }
}
