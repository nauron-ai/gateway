use super::{JobEngine, JobRepository, JobStatus};

#[derive(Debug, Clone)]
pub struct JobStatusAggregate {
    pub status: JobStatus,
    pub count: i64,
}

#[derive(Debug, Clone)]
pub struct JobEngineAggregate {
    pub engine: JobEngine,
    pub count: i64,
}

impl JobRepository {
    pub async fn count_by_status(
        &self,
        context_id: i32,
    ) -> Result<Vec<JobStatusAggregate>, sqlx::Error> {
        sqlx::query_as!(
            JobStatusAggregate,
            r#"
            SELECT
                status as "status: JobStatus",
                COUNT(*)::bigint as "count!"
            FROM jobs
            WHERE context_id = $1
            GROUP BY status
            "#,
            context_id
        )
        .fetch_all(self.pool())
        .await
    }

    pub async fn count_by_engine(
        &self,
        context_id: i32,
    ) -> Result<Vec<JobEngineAggregate>, sqlx::Error> {
        sqlx::query_as!(
            JobEngineAggregate,
            r#"
            SELECT
                engine as "engine: JobEngine",
                COUNT(*)::bigint as "count!"
            FROM jobs
            WHERE context_id = $1
            GROUP BY engine
            "#,
            context_id
        )
        .fetch_all(self.pool())
        .await
    }
}
