mod conditions;
mod ingest;
mod mir;
mod rdf;

use nauron_contracts::IngestEvent;
use nauron_contracts::MirEvent;
use nauron_contracts::conditions::ConditionsEvaluateEvent;
use rdkafka::Message;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::BorrowedMessage;
use uuid::Uuid;

use crate::db::files::FileRepository;
use crate::db::jobs::{JobRecord, JobRepository};
use crate::kafka::KafkaPublisher;
use crate::metrics::GatewayMetrics;

use self::mir::handle_mir_event;
use self::rdf::{handle_rdf_event, parse_rdf_event};
use super::TrackerError;
use conditions::handle_conditions_event;
use ingest::handle_ingest_event;

#[derive(Debug, Clone, Copy)]
pub(super) enum TopicKind {
    Mir,
    Rdf,
    Ingest,
    Conditions,
}

pub async fn handle_message(
    kind: TopicKind,
    job_repo: &JobRepository,
    file_repo: &FileRepository,
    rdf_publisher: &KafkaPublisher,
    consumer: &StreamConsumer,
    metrics: &GatewayMetrics,
    message: &BorrowedMessage<'_>,
) -> Result<(), TrackerError> {
    let payload = match message.payload_view::<str>() {
        Some(Ok(text)) => text,
        _ => {
            consumer.commit_message(message, CommitMode::Async)?;
            return Ok(());
        }
    };

    match kind {
        TopicKind::Mir => {
            if let Ok(event) = serde_json::from_str::<MirEvent>(payload) {
                handle_mir_event(job_repo, file_repo, rdf_publisher, metrics, event).await?;
            }
        }
        TopicKind::Rdf => {
            if let Some(event) = parse_rdf_event(payload) {
                handle_rdf_event(job_repo, event).await?;
            } else {
                metrics.record_rdf_invalid_payload(message.topic());
                tracing::warn!(
                    topic = message.topic(),
                    "tracker dropping invalid rdf payload"
                );
            }
        }
        TopicKind::Ingest => {
            if let Ok(event) = serde_json::from_str::<IngestEvent>(payload) {
                handle_ingest_event(job_repo, metrics, event).await?;
            }
        }
        TopicKind::Conditions => {
            if let Ok(event) = serde_json::from_str::<ConditionsEvaluateEvent>(payload) {
                handle_conditions_event(job_repo, metrics, event).await?;
            }
        }
    }
    consumer.commit_message(message, CommitMode::Async)?;
    Ok(())
}

pub(super) async fn lookup_job(
    job_repo: &JobRepository,
    job_id: Uuid,
) -> Result<JobRecord, TrackerError> {
    job_repo
        .get(job_id)
        .await?
        .ok_or_else(|| TrackerError::UnknownJob(job_id.to_string()))
}
