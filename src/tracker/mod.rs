mod consumer;
mod message;
mod propagate;
pub(crate) mod rdf;
mod snapshots;

use std::{sync::Arc, time::Duration};

use rdkafka::Message;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::db::files::FileRepository;
use crate::db::jobs::{JobEngine, JobRepository, JobSnapshotUpsert, JobStatus};
use crate::job_mode::JobLaunchMode;
use crate::kafka::KafkaPublisher;
use crate::kafka::topic_with_prefix;
use crate::metrics::GatewayMetrics;
use nauron_contracts::{
    CONDITIONS_EVALUATE_PROGRESS_TOPIC, CONDITIONS_EVALUATE_RESULT_TOPIC, INGEST_PROGRESS_TOPIC,
    INGEST_RESULT_TOPIC,
};

use message::TopicKind;

pub use snapshots::{JobResultPayload, JobSnapshot, TrackerError};

pub struct JobTracker {
    job_repo: JobRepository,
}

impl JobTracker {
    pub async fn spawn(
        config: &AppConfig,
        job_repo: JobRepository,
        file_repo: FileRepository,
        rdf_publisher: KafkaPublisher,
        metrics: Arc<GatewayMetrics>,
    ) -> Result<Self, TrackerError> {
        let consumer = consumer::create_consumer(&config.kafka, &config.status_group)?;
        let ingest_progress = topic_with_prefix(&config.queue_topic_prefix, INGEST_PROGRESS_TOPIC);
        let ingest_result = topic_with_prefix(&config.queue_topic_prefix, INGEST_RESULT_TOPIC);
        let conditions_progress = topic_with_prefix(
            &config.queue_topic_prefix,
            CONDITIONS_EVALUATE_PROGRESS_TOPIC,
        );
        let conditions_result =
            topic_with_prefix(&config.queue_topic_prefix, CONDITIONS_EVALUATE_RESULT_TOPIC);
        consumer.subscribe(&[
            config.progress_topic.as_str(),
            config.result_topic.as_str(),
            config.rdf_progress_topic.as_str(),
            config.rdf_result_topic.as_str(),
            ingest_progress.as_str(),
            ingest_result.as_str(),
            conditions_progress.as_str(),
            conditions_result.as_str(),
        ])?;
        let topics = Arc::new(TrackerTopics::from_config(config));
        let tracker = Self {
            job_repo: job_repo.clone(),
        };
        tracker.start_background(
            consumer,
            job_repo,
            file_repo,
            rdf_publisher,
            metrics,
            topics,
        );
        Ok(tracker)
    }

    fn start_background(
        &self,
        consumer: StreamConsumer,
        job_repo: JobRepository,
        file_repo: FileRepository,
        rdf_publisher: KafkaPublisher,
        metrics: Arc<GatewayMetrics>,
        topics: Arc<TrackerTopics>,
    ) {
        tokio::spawn(async move {
            let mut stream = consumer.stream();
            while let Some(result) = stream.next().await {
                match result {
                    Ok(message) => {
                        let topic = message.topic().to_string();
                        let topic_kind = topics.classify(&topic);
                        if let Some(kind) = topic_kind {
                            match message::handle_message(
                                kind,
                                &job_repo,
                                &file_repo,
                                &rdf_publisher,
                                &consumer,
                                &metrics,
                                &message,
                            )
                            .await
                            {
                                Ok(()) => {}
                                Err(TrackerError::UnknownJob(job_id)) => {
                                    tracing::info!(
                                        job_id,
                                        topic = topic.as_str(),
                                        "tracker dropping event for unknown job"
                                    );
                                    if let Err(err) =
                                        consumer.commit_message(&message, CommitMode::Async)
                                    {
                                        tracing::warn!(
                                            "tracker commit error after unknown job: {err}"
                                        );
                                    }
                                }
                                Err(err) => {
                                    tracing::warn!("tracker message error: {err}");
                                }
                            }
                        } else if let Err(err) =
                            consumer.commit_message(&message, CommitMode::Async)
                        {
                            tracing::warn!("tracker commit error: {err}");
                        }
                    }
                    Err(err) => {
                        tracing::warn!("tracker stream error: {err}");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        });
    }

    pub async fn register_pending(
        &self,
        job_id: Uuid,
        context_id: i32,
        pipeline_id: Uuid,
        file_id: Option<i64>,
        mode: JobLaunchMode,
    ) -> Result<(), TrackerError> {
        let upsert = JobSnapshotUpsert {
            job_id,
            context_id,
            file_id,
            pipeline_id: Some(pipeline_id),
            source_job_id: None,
            engine: JobEngine::Mir,
            kind: mode.as_kind(),
            status: JobStatus::Pending,
            stage: None,
            progress_pct: None,
            stage_progress_current: None,
            stage_progress_total: None,
            stage_progress_pct: None,
            message: None,
            result_json: None,
            updated_at: chrono::Utc::now(),
        };
        self.job_repo.upsert_snapshot(upsert).await?;
        Ok(())
    }
}

#[derive(Clone)]
struct TrackerTopics {
    mir_progress: String,
    mir_result: String,
    rdf_progress: String,
    rdf_result: String,
    ingest_progress: String,
    ingest_result: String,
    conditions_progress: String,
    conditions_result: String,
}

impl TrackerTopics {
    fn from_config(config: &AppConfig) -> Self {
        Self {
            mir_progress: config.progress_topic.clone(),
            mir_result: config.result_topic.clone(),
            rdf_progress: config.rdf_progress_topic.clone(),
            rdf_result: config.rdf_result_topic.clone(),
            ingest_progress: topic_with_prefix(&config.queue_topic_prefix, INGEST_PROGRESS_TOPIC),
            ingest_result: topic_with_prefix(&config.queue_topic_prefix, INGEST_RESULT_TOPIC),
            conditions_progress: topic_with_prefix(
                &config.queue_topic_prefix,
                CONDITIONS_EVALUATE_PROGRESS_TOPIC,
            ),
            conditions_result: topic_with_prefix(
                &config.queue_topic_prefix,
                CONDITIONS_EVALUATE_RESULT_TOPIC,
            ),
        }
    }

    fn classify(&self, topic: &str) -> Option<TopicKind> {
        if topic == self.mir_progress || topic == self.mir_result {
            Some(TopicKind::Mir)
        } else if topic == self.rdf_progress || topic == self.rdf_result {
            Some(TopicKind::Rdf)
        } else if topic == self.ingest_progress || topic == self.ingest_result {
            Some(TopicKind::Ingest)
        } else if topic == self.conditions_progress || topic == self.conditions_result {
            Some(TopicKind::Conditions)
        } else {
            None
        }
    }
}
