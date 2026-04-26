use super::super::model::JobRecord;
use super::JobRepository;

impl JobRepository {
    pub async fn sync_linked_to_source(&self, source: &JobRecord) -> Result<u64, sqlx::Error> {
        let stage = source
            .stage
            .map(|stage| stage.columns())
            .unwrap_or_default();

        let result = sqlx::query!(
            r#"
            UPDATE jobs
            SET status = $1,
                mir_stage = $2,
                rdf_stage = $3,
                ingest_stage = $4,
                conditions_stage = $5,
                progress_pct = $6,
                stage_progress_current = $7,
                stage_progress_total = $8,
                stage_progress_pct = $9,
                message = $10,
                result_json = $11,
                updated_at = $12
            WHERE source_job_id = $13
              AND kind = 'mir_linked'::job_kind
            "#,
            source.status as _,
            stage.mir_stage as _,
            stage.rdf_stage as _,
            stage.ingest_stage as _,
            stage.conditions_stage as _,
            source.progress_pct,
            source.stage_progress_current,
            source.stage_progress_total,
            source.stage_progress_pct,
            source.message.clone(),
            source.result_json.clone(),
            source.updated_at,
            source.job_id,
        )
        .execute(self.pool())
        .await?;
        Ok(result.rows_affected())
    }
}
