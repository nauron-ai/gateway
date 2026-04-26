use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use nauron_contracts::conditions::ConditionsEvaluateStage;
use nauron_contracts::{IngestStage, MirStage, RdfStage};

#[derive(Debug, Clone, FromRow)]
pub struct JobRecord {
    pub job_id: Uuid,
    pub context_id: i32,
    pub file_id: Option<i64>,
    pub pipeline_id: Uuid,
    pub source_job_id: Option<Uuid>,
    pub engine: JobEngine,
    pub kind: Option<JobKind>,
    pub status: JobStatus,
    pub stage: Option<super::stage::JobStage>,
    pub progress_pct: Option<i16>,
    pub stage_progress_current: Option<i32>,
    pub stage_progress_total: Option<i32>,
    pub stage_progress_pct: Option<i16>,
    pub message: Option<String>,
    pub result_json: Option<Value>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub(crate) struct JobRow {
    pub job_id: Uuid,
    pub context_id: i32,
    pub file_id: Option<i64>,
    pub pipeline_id: Uuid,
    pub source_job_id: Option<Uuid>,
    pub engine: JobEngine,
    pub kind: Option<JobKind>,
    pub status: JobStatus,
    pub mir_stage: Option<MirStage>,
    pub rdf_stage: Option<RdfStage>,
    pub ingest_stage: Option<IngestStage>,
    pub conditions_stage: Option<ConditionsEvaluateStage>,
    pub progress_pct: Option<i16>,
    pub stage_progress_current: Option<i32>,
    pub stage_progress_total: Option<i32>,
    pub stage_progress_pct: Option<i16>,
    pub message: Option<String>,
    pub result_json: Option<Value>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<JobRow> for JobRecord {
    type Error = sqlx::Error;

    fn try_from(row: JobRow) -> Result<Self, Self::Error> {
        let stage = super::stage::JobStage::from_columns(
            row.engine,
            row.mir_stage,
            row.rdf_stage,
            row.ingest_stage,
            row.conditions_stage,
        );

        Ok(Self {
            job_id: row.job_id,
            context_id: row.context_id,
            file_id: row.file_id,
            pipeline_id: row.pipeline_id,
            source_job_id: row.source_job_id,
            engine: row.engine,
            kind: row.kind,
            status: row.status,
            stage,
            progress_pct: row.progress_pct,
            stage_progress_current: row.stage_progress_current,
            stage_progress_total: row.stage_progress_total,
            stage_progress_pct: row.stage_progress_pct,
            message: row.message,
            result_json: row.result_json,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, Clone)]
pub struct JobSnapshotUpsert {
    pub job_id: Uuid,
    pub context_id: i32,
    pub file_id: Option<i64>,
    pub pipeline_id: Option<Uuid>,
    pub source_job_id: Option<Uuid>,
    pub engine: JobEngine,
    pub kind: Option<JobKind>,
    pub status: JobStatus,
    pub stage: Option<super::stage::JobStage>,
    pub progress_pct: Option<i16>,
    pub stage_progress_current: Option<i32>,
    pub stage_progress_total: Option<i32>,
    pub stage_progress_pct: Option<i16>,
    pub message: Option<String>,
    pub result_json: Option<Value>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "job_engine", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum JobEngine {
    Mir,
    Rdf,
    Lpg,
    Bayessian,
    Ingest,
    Conditions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "job_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    Reused,
    MirLinked,
    Fanout,
    Retry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "job_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    InProgress,
    Success,
    Failure,
    Retryable,
    Retired,
}

#[derive(Debug, Clone)]
pub struct JobListParams {
    pub context_id: i32,
    pub limit: i64,
    pub cursor: Option<JobListCursor>,
}

#[derive(Debug, Clone)]
pub struct JobListCursor {
    pub updated_at: DateTime<Utc>,
    pub job_id: Uuid,
}
