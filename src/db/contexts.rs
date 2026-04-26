use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct ContextRepository {
    pool: PgPool,
}

pub struct ContextListParams {
    pub cursor: Option<ContextListCursor>,
    pub limit: i64,
    pub owner_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct ContextListCursor {
    pub created_at: DateTime<Utc>,
    pub id: i32,
}

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "context_mode", rename_all = "lowercase")]
#[derive(utoipa::ToSchema, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContextMode {
    Emb,
    Rdf,
    Lpg,
}

#[derive(Debug, Clone)]
pub struct ContextRecord {
    pub id: i32,
    pub title: Option<String>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub owner_id: Option<Uuid>,
    pub mode: ContextMode,
    pub files_count: Option<i64>,
}

pub struct UpdateContextParams {
    pub title: Option<String>,
    pub description: Option<String>,
    pub mode: Option<ContextMode>,
}

impl ContextRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        owner_id: Option<Uuid>,
        mode: ContextMode,
    ) -> Result<ContextRecord, sqlx::Error> {
        sqlx::query_as!(
            ContextRecord,
            r#"
            INSERT INTO contexts (owner_id, mode)
            VALUES ($1, $2)
            RETURNING id, title, description, created_at, updated_at, owner_id, mode as "mode: ContextMode", 0::bigint as "files_count"
            "#,
            owner_id,
            mode as ContextMode
        )
        .fetch_one(&self.pool)
        .await
    }

    pub async fn get(&self, id: i32) -> Result<Option<ContextRecord>, sqlx::Error> {
        sqlx::query_as!(
            ContextRecord,
            r#"
            SELECT 
                c.id, 
                c.title, 
                c.description, 
                c.created_at, 
                c.updated_at, 
                c.owner_id,
                c.mode as "mode: ContextMode",
                (SELECT COUNT(*) FROM context_files cf WHERE cf.context_id = c.id) as "files_count"
            FROM contexts c
            WHERE c.id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list(&self, params: ContextListParams) -> Result<Vec<ContextRecord>, sqlx::Error> {
        if let Some(ContextListCursor { created_at, id }) = params.cursor {
            sqlx::query_as!(
                ContextRecord,
                r#"
            SELECT 
                c.id, 
                c.title, 
                c.description, 
                c.created_at, 
                c.updated_at, 
                c.owner_id,
                c.mode as "mode: ContextMode",
                (SELECT COUNT(*) FROM context_files cf WHERE cf.context_id = c.id) as "files_count?"
            FROM contexts c
            WHERE (c.created_at, c.id) < ($2, $3)
            AND ($4::UUID IS NULL OR c.owner_id = $4)
            ORDER BY c.created_at DESC, c.id DESC
                LIMIT $1
                "#,
                params.limit,
                created_at,
                id,
                params.owner_id
            )
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as!(
                ContextRecord,
                r#"
                SELECT 
                    c.id, 
                    c.title, 
                    c.description, 
                    c.created_at, 
                    c.updated_at, 
                    c.owner_id,
                    c.mode as "mode: ContextMode",
                    (SELECT COUNT(*) FROM context_files cf WHERE cf.context_id = c.id) as "files_count?"
                FROM contexts c
                WHERE ($2::UUID IS NULL OR c.owner_id = $2)
                ORDER BY c.created_at DESC, c.id DESC
                LIMIT $1
                "#,
                params.limit,
                params.owner_id
            )
            .fetch_all(&self.pool)
            .await
        }
    }

    pub async fn update(
        &self,
        id: i32,
        params: UpdateContextParams,
    ) -> Result<Option<ContextRecord>, sqlx::Error> {
        sqlx::query_as!(
            ContextRecord,
            r#"
            UPDATE contexts
            SET 
                title = COALESCE($2, title),
                description = COALESCE($3, description),
                mode = COALESCE($4, mode),
                updated_at = NOW()
            WHERE id = $1
            RETURNING 
                id, 
                title, 
                description, 
                created_at, 
                updated_at, 
                owner_id,
                mode as "mode: ContextMode",
                (SELECT COUNT(*) FROM context_files cf WHERE cf.context_id = contexts.id) as "files_count"
            "#,
            id,
            params.title,
            params.description,
            params.mode as Option<ContextMode>
        )
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn delete(&self, id: i32) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            DELETE FROM contexts
            WHERE id = $1
            "#,
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn health_check(&self) -> Result<(), sqlx::Error> {
        let _ = sqlx::query_scalar!("SELECT 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(())
    }
}
