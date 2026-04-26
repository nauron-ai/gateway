use std::time::Duration;

use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord, Producer};
use serde::Serialize;
use tracing::error;

use crate::config::{KafkaSettings, KafkaTls};
use nauron_contracts::MirRequest;

#[derive(Clone)]
pub struct KafkaPublisher {
    producer: FutureProducer,
    topic: String,
}

impl KafkaPublisher {
    pub fn new(settings: &KafkaSettings, topic: String) -> Result<Self, KafkaError> {
        let mut config = ClientConfig::new();
        config.set("bootstrap.servers", &settings.brokers);
        config.set("message.timeout.ms", "5000");
        if let Some(tls) = settings.tls.as_ref() {
            apply_tls(&mut config, tls);
        }
        let producer = config.create().map_err(KafkaError::Create)?;
        Ok(Self { producer, topic })
    }

    pub async fn publish_request(&self, request: &MirRequest) -> Result<(), KafkaError> {
        self.publish_json(&request.job_id, request).await
    }

    pub async fn publish_json<K: ToString, T: Serialize>(
        &self,
        key: K,
        payload: &T,
    ) -> Result<(), KafkaError> {
        let key = key.to_string();
        let bytes = serde_json::to_vec(payload).map_err(|err| {
            error!(topic = %self.topic, key = %key, error = %err, "kafka serialize failed");
            KafkaError::Serialize(err)
        })?;
        self.producer
            .send(
                FutureRecord::to(&self.topic).key(&key).payload(&bytes),
                Duration::from_secs(0),
            )
            .await
            .map_err(|(err, _)| {
                error!(topic = %self.topic, key = %key, error = %err, "kafka send failed");
                KafkaError::Send(err)
            })?;
        Ok(())
    }

    pub fn check_health(&self) -> Result<(), KafkaError> {
        self.producer
            .client()
            .fetch_metadata(None, Duration::from_secs(1))
            .map(|_| ())
            .map_err(KafkaError::Metadata)
    }
}

pub fn topic_with_prefix(prefix: &str, topic: &str) -> String {
    let trimmed = prefix.trim();
    if trimmed.is_empty() {
        return topic.to_string();
    }
    if trimmed.ends_with('.') {
        return format!("{trimmed}{topic}");
    }
    format!("{trimmed}.{topic}")
}

pub(crate) fn apply_tls(config: &mut ClientConfig, tls: &KafkaTls) {
    config.set("security.protocol", "ssl");
    if let Some(ca) = tls.ca.to_str() {
        config.set("ssl.ca.location", ca);
    }
    if let Some(cert) = tls.cert.to_str() {
        config.set("ssl.certificate.location", cert);
    }
    if let Some(key) = tls.key.to_str() {
        config.set("ssl.key.location", key);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum KafkaError {
    #[error("failed to create producer: {0}")]
    Create(#[from] rdkafka::error::KafkaError),
    #[error("failed to serialize request: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("failed to send kafka message: {0}")]
    Send(rdkafka::error::KafkaError),
    #[error("failed to fetch kafka metadata: {0}")]
    Metadata(rdkafka::error::KafkaError),
}
