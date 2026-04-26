use super::super::model::{JobEngine, JobKind, JobRecord, JobRow, JobSnapshotUpsert, JobStatus};
use super::JobRepository;
use nauron_contracts::conditions::ConditionsEvaluateStage;
use nauron_contracts::{IngestStage, MirStage, RdfStage};

impl JobRepository {
    pub async fn upsert_linked_snapshot(
        &self,
        snapshot: JobSnapshotUpsert,
    ) -> Result<JobRecord, sqlx::Error> {
        let stage = snapshot
            .stage
            .map(|stage| stage.columns())
            .unwrap_or_default();

        let row = sqlx::query_as!(
            JobRow,
            r#"
            INSERT INTO jobs (
                job_id,
                context_id,
                file_id,
                pipeline_id,
                source_job_id,
                engine,
                kind,
                status,
                mir_stage,
                rdf_stage,
                ingest_stage,
                conditions_stage,
                progress_pct,
                stage_progress_current,
                stage_progress_total,
                stage_progress_pct,
                message,
                result_json,
                updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8,
                $9, $10,
                $11, $12,
                $13, $14, $15, $16, $17, $18, $19
            )
            ON CONFLICT (pipeline_id, file_id, source_job_id)
            WHERE kind = 'mir_linked'::job_kind AND status <> 'retired' AND source_job_id IS NOT NULL
            DO UPDATE
            SET context_id = EXCLUDED.context_id,
                engine = EXCLUDED.engine,
                kind = EXCLUDED.kind,
                status = EXCLUDED.status,
                mir_stage = EXCLUDED.mir_stage,
                rdf_stage = EXCLUDED.rdf_stage,
                ingest_stage = EXCLUDED.ingest_stage,
                conditions_stage = EXCLUDED.conditions_stage,
                progress_pct = EXCLUDED.progress_pct,
                stage_progress_current = EXCLUDED.stage_progress_current,
                stage_progress_total = EXCLUDED.stage_progress_total,
                stage_progress_pct = EXCLUDED.stage_progress_pct,
                message = EXCLUDED.message,
                result_json = EXCLUDED.result_json,
                updated_at = EXCLUDED.updated_at
            RETURNING
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
            "#,
            snapshot.job_id,
            snapshot.context_id,
            snapshot.file_id,
            snapshot.pipeline_id,
            snapshot.source_job_id,
            snapshot.engine as _,
            snapshot.kind as _,
            snapshot.status as _,
            stage.mir_stage as _,
            stage.rdf_stage as _,
            stage.ingest_stage as _,
            stage.conditions_stage as _,
            snapshot.progress_pct,
            snapshot.stage_progress_current,
            snapshot.stage_progress_total,
            snapshot.stage_progress_pct,
            snapshot.message,
            snapshot.result_json,
            snapshot.updated_at,
        )
        .fetch_one(&self.pool)
        .await?;

        JobRecord::try_from(row)
    }
}
