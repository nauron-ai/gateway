use clap::Parser;
use nauron_contracts::{
    MIR_PROGRESS_TOPIC, MIR_REQUEST_TOPIC, MIR_RESULT_TOPIC, RDF_PROGRESS_TOPIC, RDF_RESULT_TOPIC,
    RDF_START_TOPIC,
};
use std::path::PathBuf;

/// CLI / env configuration for the Gateway service.
#[derive(Debug, Parser, Clone)]
#[command(name = "gateway", about = "HTTP ingress for MIR worker")]
pub struct GatewayArgs {
    #[arg(long, env = "GATEWAY_LISTEN_ADDR", default_value = "0.0.0.0:8080")]
    pub listen: String,

    #[arg(
        long,
        env = "DATABASE_URL",
        default_value = "postgres://ordung:ordung@localhost:5432/ordung"
    )]
    pub database_url: String,

    #[arg(long, env = "DATABASE_MAX_CONNECTIONS", default_value_t = 10)]
    pub database_max_connections: u32,

    #[arg(long, env = "MIR_INPUT_BUCKET", default_value = "mir-input")]
    pub input_bucket: String,

    #[arg(long, env = "MIR_INPUT_PREFIX", default_value = "uploads")]
    pub input_prefix: String,

    #[arg(long, env = "MIR_OUTPUT_BUCKET", default_value = "mir-output")]
    pub output_bucket: String,

    #[arg(long, env = "MIR_OUTPUT_PREFIX", default_value = "jobs")]
    pub output_prefix: String,

    #[arg(long, env = "MIR_REQUEST_TOPIC", default_value = MIR_REQUEST_TOPIC)]
    pub request_topic: String,

    #[arg(long, env = "MIR_PROGRESS_TOPIC", default_value = MIR_PROGRESS_TOPIC)]
    pub progress_topic: String,

    #[arg(long, env = "MIR_RESULT_TOPIC", default_value = MIR_RESULT_TOPIC)]
    pub result_topic: String,
    #[arg(long, env = "RDF_START_TOPIC", default_value = RDF_START_TOPIC)]
    pub rdf_start_topic: String,
    #[arg(long, env = "RDF_PROGRESS_TOPIC", default_value = RDF_PROGRESS_TOPIC)]
    pub rdf_progress_topic: String,
    #[arg(long, env = "RDF_RESULT_TOPIC", default_value = RDF_RESULT_TOPIC)]
    pub rdf_result_topic: String,

    #[arg(long, env = "KAFKA_BROKERS", default_value = "127.0.0.1:9093")]
    pub kafka_brokers: String,

    #[arg(long, env = "QUEUE_TOPIC_PREFIX", default_value = "")]
    pub queue_topic_prefix: String,

    #[arg(
        long,
        env = "GATEWAY_STATUS_GROUP",
        default_value = "mir-gateway-status"
    )]
    pub status_group: String,

    #[arg(long, env = "KAFKA_TLS_CA")]
    pub kafka_tls_ca: Option<PathBuf>,

    #[arg(long, env = "KAFKA_TLS_CERT")]
    pub kafka_tls_cert: Option<PathBuf>,

    #[arg(long, env = "KAFKA_TLS_KEY")]
    pub kafka_tls_key: Option<PathBuf>,

    #[arg(long, env = "S3_ENDPOINT")]
    pub s3_endpoint: String,

    #[arg(long, env = "S3_ACCESS_KEY")]
    pub s3_access_key: String,

    #[arg(long, env = "S3_SECRET_KEY")]
    pub s3_secret_key: String,

    #[arg(long, env = "S3_REGION", default_value = "us-east-1")]
    pub s3_region: String,

    #[arg(long, env = "S3_FORCE_PATH_STYLE", default_value_t = true)]
    pub s3_force_path_style: bool,

    #[arg(long, env = "FILES_DEDUP_ENABLED", default_value_t = true)]
    pub files_dedup_enabled: bool,

    #[arg(long, env = "INFERENCER_URL", default_value = "http://inferencer:8081")]
    pub inferencer_url: String,

    #[arg(long, env = "INFERENCER_CB_FAILURE_THRESHOLD", default_value_t = 5)]
    pub inferencer_cb_failure_threshold: u32,

    #[arg(long, env = "INFERENCER_CB_SUCCESS_THRESHOLD", default_value_t = 2)]
    pub inferencer_cb_success_threshold: u32,

    #[arg(long, env = "INFERENCER_CB_OPEN_DURATION_SECS", default_value_t = 30)]
    pub inferencer_cb_open_duration_secs: u64,

    #[arg(long, env = "VECTOR_API_URL", default_value = "http://vector-api:8082")]
    pub vector_api_url: String,

    #[arg(long, env = "JWT_SECRET")]
    pub jwt_secret: String,

    #[arg(long, env = "JWT_TTL_SECONDS", default_value_t = 86400)]
    pub jwt_ttl_seconds: i64,

    #[arg(long, env = "ADMIN_EMAIL")]
    pub admin_email: String,

    #[arg(long, env = "ADMIN_PASSWORD")]
    pub admin_password: String,
}
