use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, patch, post},
};
use utoipa_swagger_ui::SwaggerUi;

use crate::{docs::GatewayApiDoc, state::AppState};

mod cors;
mod download_headers;
mod tracing;

pub(crate) mod admin;
pub(crate) mod auth;
pub(crate) mod chat;
pub(crate) mod contexts;
pub(crate) mod health;
pub(crate) mod jobs;
pub(crate) mod load;
pub(crate) mod oneshots;
mod pagination;
pub(crate) mod pipelines;
pub(crate) mod search;
pub(crate) mod settings;
pub(crate) mod shares;

const MAX_BODY_SIZE_BYTES: usize = 200 * 1024 * 1024;

pub fn build_router(state: Arc<AppState>) -> Router {
    let public_router = Router::new()
        .route("/healthz", get(health::health))
        .route("/auth/login", post(auth::login));

    let admin_router = admin::router().layer(middleware::from_fn(crate::auth::require_admin));

    let protected_router = Router::new()
        .route(
            "/v1/contexts",
            get(contexts::list_contexts).post(contexts::create_context),
        )
        .route(
            "/v1/contexts/{context_id}",
            patch(contexts::update_context).delete(contexts::delete_context),
        )
        .route(
            "/v1/contexts/{context_id}/jobs",
            get(contexts::jobs::list_context_jobs),
        )
        .route(
            "/v1/contexts/{context_id}/jobs/stats",
            get(contexts::stats::context_job_stats),
        )
        .route(
            "/v1/contexts/{context_id}/files",
            get(contexts::list_context_files),
        )
        .route(
            "/v1/contexts/{context_id}/files/{context_file_id}/download",
            get(contexts::download_context_file),
        )
        .route(
            "/v1/contexts/{context_id}/graph",
            get(contexts::get_context_graph),
        )
        .route(
            "/v1/contexts/{context_id}/entities",
            get(contexts::get_context_entities),
        )
        .route(
            "/v1/contexts/{context_id}/conditions/evaluate/jobs",
            post(contexts::create_evaluate_conditions_job),
        )
        .route(
            "/v1/contexts/{context_id}/shares",
            post(shares::add_share).delete(shares::remove_share),
        )
        .route("/v1/load/context", post(load::load_context))
        .route("/v1/load/context/", post(load::load_context))
        .route("/v1/jobs/{job_id}", get(jobs::job_status))
        .route("/v1/jobs/{job_id}/retry", post(jobs::retry_job))
        .route("/v1/jobs/{job_id}/result", get(jobs::download_result))
        .route(
            "/v1/pipelines/{pipeline_id}",
            get(pipelines::pipeline_status),
        )
        .merge(search::router())
        .merge(oneshots::router())
        .route(
            "/v1/settings",
            get(settings::get_settings).patch(settings::update_settings),
        )
        .nest("/v1/chat", chat::router())
        .nest("/admin", admin_router)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::auth::require_auth,
        ))
        .layer(DefaultBodyLimit::max(MAX_BODY_SIZE_BYTES));

    let api_router = public_router.merge(protected_router).merge(
        SwaggerUi::new("/swagger").url("/swagger/openapi.json", GatewayApiDoc::openapi_spec()),
    );

    api_router
        .layer(tracing::build_trace_layer())
        .layer(cors::build_cors_layer())
        .with_state(state)
}
