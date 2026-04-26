use rdkafka::ClientConfig;
use rdkafka::consumer::StreamConsumer;

use crate::config::KafkaSettings;

use super::TrackerError;

pub fn create_consumer(
    settings: &KafkaSettings,
    group_id: &str,
) -> Result<StreamConsumer, TrackerError> {
    let mut config = ClientConfig::new();
    config.set("bootstrap.servers", &settings.brokers);
    config.set("group.id", group_id);
    config.set("enable.auto.commit", "false");
    config.set("auto.offset.reset", "earliest");
    if let Some(tls) = settings.tls.as_ref() {
        crate::kafka::apply_tls(&mut config, tls);
    }
    Ok(config.create()?)
}
