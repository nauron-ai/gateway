use uuid::Uuid;

use super::super::model::{
    JobEngine, JobKind, JobListCursor, JobListParams, JobRecord, JobRow, JobStatus,
};
use super::JobRepository;
use nauron_contracts::conditions::ConditionsEvaluateStage;
use nauron_contracts::{IngestStage, MirStage, RdfStage};

impl JobRepository {
    pub async fn get(&self, job_id: Uuid) -> Result<Option<JobRecord>, sqlx::Error> {
        let row = sqlx::query_as!(
            JobRow,
            r#"
            SELECT
                job_id,
                context_id,
                file_id,
                pipeline_id,
                source_job_id,
                engine as "engine: JobEngine",
                kind as "kind: JobKind",
                status as "status: JobStatus",
                mir_stage as "mir_stage: MirStage",
                rdf_stage as "rdf_stage: RdfStage",
                ingest_stage as "ingest_stage: IngestStage",
                conditions_stage as "conditions_stage: ConditionsEvaluateStage",
                progress_pct,
                stage_progress_current,
                stage_progress_total,
                stage_progress_pct,
                message,
                result_json,
                updated_at
            FROM jobs
            WHERE job_id = $1
            "#,
            job_id
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(JobRecord::try_from).transpose()
    }

    pub async fn list_by_context(
        &self,
        params: JobListParams,
    ) -> Result<Vec<JobRecord>, sqlx::Error> {
        if let Some(JobListCursor { updated_at, job_id }) = params.cursor {
            let rows = sqlx::query_as!(
                JobRow,
                r#"
                SELECT
                    job_id,
                    context_id,
                    file_id,
                    pipeline_id,
                    source_job_id,
                    engine as "engine: JobEngine",
                    kind as "kind: JobKind",
                    status as "status: JobStatus",
                    mir_stage as "mir_stage: MirStage",
                    rdf_stage as "rdf_stage: RdfStage",
                    ingest_stage as "ingest_stage: IngestStage",
                    conditions_stage as "conditions_stage: ConditionsEvaluateStage",
                    progress_pct,
                    stage_progress_current,
                    stage_progress_total,
                    stage_progress_pct,
                    message,
                    result_json,
                    updated_at
                FROM jobs
                WHERE context_id = $1
                  AND (updated_at, job_id) < ($2, $3)
                ORDER BY updated_at DESC, job_id DESC
                LIMIT $4
                "#,
                params.context_id,
                updated_at,
                job_id,
                params.limit
            )
            .fetch_all(&self.pool)
            .await?;
            rows.into_iter().map(JobRecord::try_from).collect()
        } else {
            let rows = sqlx::query_as!(
                JobRow,
                r#"
                SELECT
                    job_id,
                    context_id,
                    file_id,
                    pipeline_id,
                    source_job_id,
                    engine as "engine: JobEngine",
                    kind as "kind: JobKind",
                    status as "status: JobStatus",
                    mir_stage as "mir_stage: MirStage",
                    rdf_stage as "rdf_stage: RdfStage",
                    ingest_stage as "ingest_stage: IngestStage",
                    conditions_stage as "conditions_stage: ConditionsEvaluateStage",
                    progress_pct,
                    stage_progress_current,
                    stage_progress_total,
                    stage_progress_pct,
                    message,
                    result_json,
                    updated_at
                FROM jobs
                WHERE context_id = $1
                ORDER BY updated_at DESC, job_id DESC
                LIMIT $2
                "#,
                params.context_id,
                params.limit
            )
            .fetch_all(&self.pool)
            .await?;
            rows.into_iter().map(JobRecord::try_from).collect()
        }
    }

    pub async fn list_by_pipeline(&self, pipeline_id: Uuid) -> Result<Vec<JobRecord>, sqlx::Error> {
        let rows = sqlx::query_as!(
            JobRow,
            r#"
            SELECT
                job_id,
                context_id,
                file_id,
                pipeline_id,
                source_job_id,
                engine as "engine: JobEngine",
                kind as "kind: JobKind",
                status as "status: JobStatus",
                mir_stage as "mir_stage: MirStage",
                rdf_stage as "rdf_stage: RdfStage",
                ingest_stage as "ingest_stage: IngestStage",
                conditions_stage as "conditions_stage: ConditionsEvaluateStage",
                progress_pct,
                stage_progress_current,
                stage_progress_total,
                stage_progress_pct,
                message,
                result_json,
                updated_at
            FROM jobs
            WHERE pipeline_id = $1
            ORDER BY updated_at DESC, job_id DESC
            "#,
            pipeline_id
        )
        .fetch_all(self.pool())
        .await?;
        rows.into_iter().map(JobRecord::try_from).collect()
    }

    pub async fn exists_for_file_engine_context(
        &self,
        file_id: i64,
        context_id: i32,
        engine: JobEngine,
    ) -> Result<bool, sqlx::Error> {
        let exists = sqlx::query_scalar!(
            r#"
            SELECT 1
            FROM jobs
            WHERE file_id = $1
              AND context_id = $2
              AND engine = $3
            LIMIT 1
            "#,
            file_id,
            context_id,
            engine as _,
        )
        .fetch_optional(self.pool())
        .await?
        .is_some();
        Ok(exists)
    }

    pub async fn delete_expired_ingest_jobs(&self) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            DELETE FROM jobs
            WHERE engine = 'ingest'::job_engine
              AND status IN ('success', 'failure', 'retryable', 'retired')
              AND updated_at < (now() - interval '6 months')
            "#
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }
}
