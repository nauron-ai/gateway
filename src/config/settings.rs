use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use crate::config::GatewayArgs;
use crate::error::GatewayInitError;
use crate::inferencer::CircuitBreakerConfig;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub listen: SocketAddr,
    pub database: DatabaseSettings,
    pub queue_topic_prefix: String,
    pub input_bucket: String,
    pub input_prefix: String,
    pub output_bucket: String,
    pub output_prefix: String,
    pub request_topic: String,
    pub progress_topic: String,
    pub result_topic: String,
    pub rdf_start_topic: String,
    pub rdf_progress_topic: String,
    pub rdf_result_topic: String,
    pub status_group: String,
    pub kafka: KafkaSettings,
    pub storage: StorageSettings,
    pub files_dedup_enabled: bool,
    pub inferencer_url: String,
    pub inferencer_circuit_breaker: CircuitBreakerConfig,
    pub vector_api_url: String,
    pub auth: AuthSettings,
    pub admin: AdminSeed,
}

#[derive(Debug, Clone)]
pub struct KafkaSettings {
    pub brokers: String,
    pub tls: Option<KafkaTls>,
}

#[derive(Debug, Clone)]
pub struct KafkaTls {
    pub ca: PathBuf,
    pub cert: PathBuf,
    pub key: PathBuf,
}

#[derive(Debug, Clone)]
pub struct StorageSettings {
    pub endpoint: String,
    pub access_key: String,
    pub secret_key: String,
    pub region: String,
    pub force_path_style: bool,
}

#[derive(Debug, Clone)]
pub struct DatabaseSettings {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone)]
pub struct AuthSettings {
    pub jwt_secret: String,
    pub jwt_ttl_seconds: i64,
}

#[derive(Debug, Clone)]
pub struct AdminSeed {
    pub email: String,
    pub password: String,
}

impl AppConfig {
    pub fn try_from(args: GatewayArgs) -> Result<Self, GatewayInitError> {
        let listen = args
            .listen
            .parse()
            .map_err(|_| GatewayInitError::InvalidListenAddr(args.listen.clone()))?;
        let tls = match (args.kafka_tls_ca, args.kafka_tls_cert, args.kafka_tls_key) {
            (Some(ca), Some(cert), Some(key)) => Some(KafkaTls { ca, cert, key }),
            (None, None, None) => None,
            _ => return Err(GatewayInitError::KafkaTlsMismatch),
        };
        Ok(Self {
            listen,
            database: DatabaseSettings {
                url: args.database_url,
                max_connections: args.database_max_connections,
            },
            queue_topic_prefix: args.queue_topic_prefix,
            input_bucket: args.input_bucket,
            input_prefix: args.input_prefix.trim_matches('/').to_string(),
            output_bucket: args.output_bucket,
            output_prefix: args.output_prefix.trim_matches('/').to_string(),
            request_topic: args.request_topic,
            progress_topic: args.progress_topic,
            result_topic: args.result_topic,
            rdf_start_topic: args.rdf_start_topic,
            rdf_progress_topic: args.rdf_progress_topic,
            rdf_result_topic: args.rdf_result_topic,
            status_group: args.status_group,
            kafka: KafkaSettings {
                brokers: args.kafka_brokers,
                tls,
            },
            storage: StorageSettings {
                endpoint: args.s3_endpoint,
                access_key: args.s3_access_key,
                secret_key: args.s3_secret_key,
                region: args.s3_region,
                force_path_style: args.s3_force_path_style,
            },
            files_dedup_enabled: args.files_dedup_enabled,
            inferencer_url: args.inferencer_url,
            inferencer_circuit_breaker: CircuitBreakerConfig {
                failure_threshold: args.inferencer_cb_failure_threshold,
                success_threshold: args.inferencer_cb_success_threshold,
                open_duration: Duration::from_secs(args.inferencer_cb_open_duration_secs),
            },
            vector_api_url: args.vector_api_url,
            auth: AuthSettings {
                jwt_secret: args.jwt_secret,
                jwt_ttl_seconds: args.jwt_ttl_seconds,
            },
            admin: AdminSeed {
                email: args.admin_email,
                password: args.admin_password,
            },
        })
    }
}
