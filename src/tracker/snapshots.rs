use chrono::{DateTime, Utc};
use nauron_contracts::{IngestResult, MirResult, RdfResult};
use uuid::Uuid;

use crate::db::jobs::{JobEngine, JobKind, JobRecord, JobStage, JobStatus};

#[derive(Clone)]
pub struct JobSnapshot {
    pub job_id: Uuid,
    pub context_id: i32,
    pub pipeline_id: Uuid,
    pub file_id: Option<i64>,
    pub engine: JobEngine,
    pub kind: Option<JobKind>,
    pub status: JobStatus,
    pub stage: Option<JobStage>,
    pub percent: Option<u8>,
    pub stage_current: Option<u32>,
    pub stage_total: Option<u32>,
    pub stage_percent: Option<u8>,
    pub message: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub result: Option<JobResultPayload>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum JobResultPayload {
    Mir(MirResult),
    Rdf(RdfResult),
    Ingest(IngestResult),
    Conditions(ConditionsEvaluateResultPayload),
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ConditionsEvaluateResultPayload {
    Success {
        response: nauron_contracts::conditions::ConditionEvaluationResponse,
    },
    Failure {
        error: nauron_contracts::conditions::ConditionErrorResponse,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum TrackerError {
    #[error("kafka error: {0}")]
    Kafka(#[from] rdkafka::error::KafkaError),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("publisher error: {0}")]
    Publisher(#[from] crate::kafka::KafkaError),
    #[error("job not registered: {0}")]
    UnknownJob(String),
    #[error("stage parse error: {0}")]
    StageParse(#[from] nauron_contracts::StageParseError),
}

impl TryFrom<JobRecord> for JobSnapshot {
    type Error = TrackerError;

    fn try_from(record: JobRecord) -> Result<Self, Self::Error> {
        let percent = record.progress_pct.map(|pct| pct.clamp(0, 100) as u8);
        let stage_current = record
            .stage_progress_current
            .and_then(|value| u32::try_from(value).ok());
        let stage_total = record
            .stage_progress_total
            .and_then(|value| u32::try_from(value).ok());
        let stage_percent = record.stage_progress_pct.map(|pct| pct.clamp(0, 100) as u8);
        let result = match (record.engine, record.result_json) {
            (JobEngine::Mir, Some(value)) => {
                Some(JobResultPayload::Mir(serde_json::from_value(value)?))
            }
            (JobEngine::Rdf, Some(value)) => {
                Some(JobResultPayload::Rdf(serde_json::from_value(value)?))
            }
            (JobEngine::Ingest, Some(value)) => {
                Some(JobResultPayload::Ingest(serde_json::from_value(value)?))
            }
            (JobEngine::Conditions, Some(value)) => {
                Some(JobResultPayload::Conditions(serde_json::from_value(value)?))
            }
            _ => None,
        };

        Ok(Self {
            job_id: record.job_id,
            context_id: record.context_id,
            pipeline_id: record.pipeline_id,
            file_id: record.file_id,
            engine: record.engine,
            kind: record.kind,
            status: record.status,
            stage: record.stage,
            percent,
            stage_current,
            stage_total,
            stage_percent,
            message: record.message,
            updated_at: record.updated_at,
            result,
        })
    }
}
