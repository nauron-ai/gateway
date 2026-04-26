use crate::config::AppConfig;
use crate::db::{
    connections::ConnectionEventRepository, contexts::ContextRepository, files::FileRepository,
    jobs::JobRepository, settings::UserSettingsRepository, shares::ContextShareRepository,
    users::UserRepository,
};
use crate::inferencer::InferencerClient;
use crate::kafka::KafkaPublisher;
use crate::metrics::GatewayMetrics;
use crate::storage::StorageClient;
use crate::tracker::JobTracker;
use crate::vector_api::VectorApiClient;
use std::sync::Arc;

pub struct AppRepositories {
    pub context: ContextRepository,
    pub connections: ConnectionEventRepository,
    pub file: FileRepository,
    pub job: JobRepository,
    pub shares: ContextShareRepository,
    pub settings: UserSettingsRepository,
    pub user: UserRepository,
}

pub struct AppPublishers {
    pub mir: KafkaPublisher,
    pub rdf: KafkaPublisher,
    pub ingest: KafkaPublisher,
    pub conditions_evaluate: KafkaPublisher,
}

pub struct AppState {
    pub config: AppConfig,
    pub context_repo: ContextRepository,
    pub connection_repo: ConnectionEventRepository,
    pub file_repo: FileRepository,
    pub job_repo: JobRepository,
    pub share_repo: ContextShareRepository,
    pub settings_repo: UserSettingsRepository,
    pub user_repo: UserRepository,
    pub storage: StorageClient,
    pub mir_publisher: KafkaPublisher,
    pub rdf_publisher: KafkaPublisher,
    pub ingest_publisher: KafkaPublisher,
    pub conditions_evaluate_publisher: KafkaPublisher,
    pub tracker: JobTracker,
    pub metrics: Arc<GatewayMetrics>,
    pub inferencer_client: InferencerClient,
    pub vector_api_client: VectorApiClient,
}

impl AppState {
    pub fn new(
        config: AppConfig,
        repositories: AppRepositories,
        storage: StorageClient,
        publishers: AppPublishers,
        tracker: JobTracker,
        metrics: Arc<GatewayMetrics>,
        clients: AppClients,
    ) -> Self {
        let AppRepositories {
            context,
            connections,
            file,
            job,
            shares,
            settings,
            user,
        } = repositories;
        let AppPublishers {
            mir,
            rdf,
            ingest,
            conditions_evaluate,
        } = publishers;
        let AppClients {
            inferencer,
            vector_api,
        } = clients;
        Self {
            config,
            context_repo: context,
            connection_repo: connections,
            file_repo: file,
            job_repo: job,
            share_repo: shares,
            settings_repo: settings,
            user_repo: user,
            storage,
            mir_publisher: mir,
            rdf_publisher: rdf,
            ingest_publisher: ingest,
            conditions_evaluate_publisher: conditions_evaluate,
            tracker,
            metrics,
            inferencer_client: inferencer,
            vector_api_client: vector_api,
        }
    }
}

pub struct AppClients {
    pub inferencer: InferencerClient,
    pub vector_api: VectorApiClient,
}
