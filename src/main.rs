mod artifacts;
mod auth;
mod cleanup;
mod config;
mod db;
mod docs;
mod error;
mod idempotency;
mod inferencer;
mod job_mode;
mod kafka;
mod metrics;
mod routes;
mod state;
mod storage;
#[cfg(test)]
mod test_utils;
mod tracker;
mod vector_api;

use axum::Router;
use clap::Parser;
use config::AppConfig;
use db::{
    connections::ConnectionEventRepository, contexts::ContextRepository, files::FileRepository,
    jobs::JobRepository, settings::UserSettingsRepository, shares::ContextShareRepository,
    users::UserRepository,
};
use error::GatewayInitError;
use inferencer::InferencerClient;
use metrics::GatewayMetrics;
use routes::build_router;
use sqlx::postgres::PgPoolOptions;
use state::{AppClients, AppPublishers, AppRepositories, AppState};
use std::io;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tracing_subscriber::EnvFilter;
use tracker::JobTracker;
use {
    crate::kafka::topic_with_prefix,
    nauron_contracts::{CONDITIONS_EVALUATE_START_TOPIC, INGEST_START_TOPIC},
};

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("gateway failed: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), GatewayInitError> {
    init_tracing();

    let args = config::GatewayArgs::parse();
    let app_config = AppConfig::try_from(args)?;

    let db_pool = PgPoolOptions::new()
        .max_connections(app_config.database.max_connections)
        .connect(&app_config.database.url)
        .await?;
    sqlx::migrate!("./migrations").run(&db_pool).await?;
    let context_repo = ContextRepository::new(db_pool.clone());
    let connection_repo = ConnectionEventRepository::new(db_pool.clone());
    let file_repo = FileRepository::new(db_pool.clone());
    let job_repo = JobRepository::new(db_pool.clone());
    cleanup::spawn_ingest_job_cleanup(job_repo.clone());
    let share_repo = ContextShareRepository::new(db_pool.clone());
    let settings_repo = UserSettingsRepository::new(db_pool.clone());
    let user_repo = UserRepository::new(db_pool.clone());
    let admin_password_hash = auth::hash_password(&app_config.admin.password)
        .map_err(|e| GatewayInitError::Io(io::Error::other(e.to_string())))?;
    user_repo
        .ensure_admin(&app_config.admin.email, &admin_password_hash)
        .await?;
    let storage = storage::StorageClient::new(app_config.storage.clone()).await?;
    let inferencer_client = InferencerClient::new(
        &app_config.inferencer_url,
        connection_repo.clone(),
        app_config.inferencer_circuit_breaker.clone(),
    )
    .map_err(|e| GatewayInitError::Io(io::Error::other(e)))?;
    let vector_api_client =
        vector_api::VectorApiClient::new(&app_config.vector_api_url, connection_repo.clone())
            .map_err(|e| GatewayInitError::Io(io::Error::other(e)))?;
    let mir_publisher =
        kafka::KafkaPublisher::new(&app_config.kafka, app_config.request_topic.clone())?;
    let rdf_publisher =
        kafka::KafkaPublisher::new(&app_config.kafka, app_config.rdf_start_topic.clone())?;
    let ingest_publisher = kafka::KafkaPublisher::new(
        &app_config.kafka,
        topic_with_prefix(&app_config.queue_topic_prefix, INGEST_START_TOPIC),
    )?;
    let conditions_evaluate_publisher = kafka::KafkaPublisher::new(
        &app_config.kafka,
        topic_with_prefix(
            &app_config.queue_topic_prefix,
            CONDITIONS_EVALUATE_START_TOPIC,
        ),
    )?;
    let metrics = std::sync::Arc::new(GatewayMetrics::new());
    let tracker = JobTracker::spawn(
        &app_config,
        job_repo.clone(),
        file_repo.clone(),
        rdf_publisher.clone(),
        metrics.clone(),
    )
    .await?;
    let repositories = AppRepositories {
        context: context_repo,
        connections: connection_repo,
        file: file_repo,
        job: job_repo,
        shares: share_repo,
        settings: settings_repo,
        user: user_repo,
    };
    let publishers = AppPublishers {
        mir: mir_publisher,
        rdf: rdf_publisher,
        ingest: ingest_publisher,
        conditions_evaluate: conditions_evaluate_publisher,
    };

    let clients = AppClients {
        inferencer: inferencer_client,
        vector_api: vector_api_client,
    };
    let state = Arc::new(AppState::new(
        app_config.clone(),
        repositories,
        storage,
        publishers,
        tracker,
        metrics,
        clients,
    ));

    let router: Router = build_router(state);
    let listener = TcpListener::bind(app_config.listen).await?;
    let addr = match listener.local_addr() {
        Ok(value) => value,
        Err(_) => app_config.listen,
    };
    tracing::info!("gateway listening on {addr}");

    axum::serve(listener, router.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|err| GatewayInitError::Io(io::Error::other(err)))?;
    Ok(())
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .try_init();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if signal::ctrl_c().await.is_err() {
            std::future::pending::<()>().await;
        }
    };
    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        if let Ok(mut sigterm) = signal(SignalKind::terminate()) {
            sigterm.recv().await;
        } else {
            std::future::pending::<()>().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
}
