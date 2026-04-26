use super::super::FileStatus;
use super::{
    AttachContextFileParams, ContextFileListCursor, ContextFileListParams, ContextFileRecord,
    FileOrigin, FileRepository,
};

impl FileRepository {
    pub async fn attach_to_context(
        &self,
        params: AttachContextFileParams<'_>,
    ) -> Result<ContextFileRecord, sqlx::Error> {
        let inserted_id = sqlx::query_scalar!(
            r#"
            INSERT INTO context_files (
                context_id,
                file_id,
                pipeline_id,
                origin,
                original_name,
                original_path,
                media_type
            )
            VALUES ($1, $2, $3, $4::file_origin, $5, $6, $7)
            ON CONFLICT (context_id, file_id) DO NOTHING
            RETURNING id
            "#,
            params.context_id,
            params.file_id,
            params.pipeline_id,
            params.origin as FileOrigin,
            params.original_name,
            params.original_path,
            params.media_type
        )
        .fetch_optional(self.pool())
        .await?;

        let target_id = if let Some(id) = inserted_id {
            id
        } else {
            sqlx::query_scalar!(
                r#"
                SELECT id
                FROM context_files
                WHERE context_id = $1 AND file_id = $2
                "#,
                params.context_id,
                params.file_id
            )
            .fetch_one(self.pool())
            .await?
        };

        self.fetch_context_file_by_id(target_id).await
    }

    pub async fn list_by_context(
        &self,
        context_id: i32,
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
            WHERE cf.context_id = $1
            ORDER BY cf.attached_at DESC, cf.id DESC
            "#,
            context_id
        )
        .fetch_all(self.pool())
        .await
    }

    pub async fn list_by_context_with_cursor(
        &self,
        params: ContextFileListParams,
    ) -> Result<Vec<ContextFileRecord>, sqlx::Error> {
        if let Some(ContextFileListCursor {
            attached_at,
            context_file_id,
        }) = params.cursor
        {
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
                WHERE cf.context_id = $1
                  AND (cf.attached_at, cf.id) < ($2, $3)
                ORDER BY cf.attached_at DESC, cf.id DESC
                LIMIT $4
                "#,
                params.context_id,
                attached_at,
                context_file_id,
                params.limit
            )
            .fetch_all(self.pool())
            .await
        } else {
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
                WHERE cf.context_id = $1
                ORDER BY cf.attached_at DESC, cf.id DESC
                LIMIT $2
                "#,
                params.context_id,
                params.limit
            )
            .fetch_all(self.pool())
            .await
        }
    }
}
