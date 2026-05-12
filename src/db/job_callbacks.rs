use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct JobCallbackUpsert {
    pub job_id: Uuid,
    pub url: String,
    pub secret: Option<String>,
}

#[derive(Debug, Clone)]
pub struct JobCallbackRecord {
    pub job_id: Uuid,
    pub url: String,
    pub secret: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct JobCallbackRepository {
    pool: PgPool,
}

impl JobCallbackRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert(&self, upsert: JobCallbackUpsert) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO job_callbacks (job_id, callback_url, callback_secret, updated_at)
            VALUES ($1, $2, $3, now())
            ON CONFLICT (job_id)
            DO UPDATE SET
                callback_url = EXCLUDED.callback_url,
                callback_secret = EXCLUDED.callback_secret,
                updated_at = now()
            "#,
        )
        .bind(upsert.job_id)
        .bind(upsert.url)
        .bind(upsert.secret)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get(&self, job_id: Uuid) -> Result<Option<JobCallbackRecord>, sqlx::Error> {
        let row = sqlx::query_as::<_, (Uuid, String, Option<String>, DateTime<Utc>)>(
            r#"
            SELECT job_id, callback_url, callback_secret, updated_at
            FROM job_callbacks
            WHERE job_id = $1
            "#,
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(
            row.map(|(job_id, url, secret, updated_at)| JobCallbackRecord {
                job_id,
                url,
                secret,
                updated_at,
            }),
        )
    }
}
