use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::error::ErrorResponse;
use crate::routes::health::GatewayComponents;
use nauron_contracts::health::{ComponentStatus, HealthResponse, ServiceStatus};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::health::health,
        crate::routes::contexts::list_contexts,
        crate::routes::contexts::create_context,
        crate::routes::contexts::update_context,
        crate::routes::contexts::delete_context,
        crate::routes::contexts::list_context_files,
        crate::routes::contexts::download_context_file,
        crate::routes::contexts::jobs::list_context_jobs,
        crate::routes::contexts::stats::context_job_stats,
        crate::routes::contexts::graph::get_context_graph,
        crate::routes::contexts::graph::get_context_entities,
        crate::routes::contexts::create_evaluate_conditions_job,
        crate::routes::search::global_search,
        crate::routes::settings::get_settings,
        crate::routes::settings::update_settings,
        crate::routes::shares::add_share,
        crate::routes::shares::remove_share,
        crate::routes::load::load_context,
        crate::routes::jobs::job_status,
        crate::routes::jobs::download_result,
        crate::routes::jobs::retry_job,
        crate::routes::pipelines::pipeline_status,
        crate::routes::auth::login,
        crate::routes::admin::users::list_users,
        crate::routes::admin::users::create_user,
        crate::routes::admin::users::update_user,
        crate::routes::admin::documents::download_document,
        crate::routes::admin::files::list_files,
        crate::routes::admin::contexts::list_context_files,
        crate::routes::admin::files::file_details,
        crate::routes::admin::files::retry_file,
        crate::routes::admin::connections::list_connection_events,
        crate::routes::admin::chat::list_sessions,
        crate::routes::admin::chat::get_session,
        crate::routes::admin::chat::get_reasoning,
        crate::routes::chat::chat_emb,
        crate::routes::chat::chat_rdf_emb,
        crate::routes::chat::chat_bn,
        crate::routes::chat::list_sessions,
        crate::routes::chat::get_session,
        crate::routes::chat::get_reasoning,
        crate::routes::oneshots::create_oneshot,
        crate::routes::oneshots::create_ingest_job
    ),
    components(
        schemas(
            ErrorResponse,
            HealthResponse<GatewayComponents>,
            GatewayComponents,
            ComponentStatus,
            ServiceStatus,
            crate::routes::contexts::ContextResponse,
            crate::routes::contexts::ContextsResponse,
            crate::routes::contexts::ContextFilesResponse,
            crate::routes::contexts::ContextFileEntry,
            crate::routes::contexts::ContextFileCursor,
            crate::routes::contexts::ContextFilesQuery,
            crate::routes::contexts::conditions::EvaluateConditionsRequest,
            crate::routes::contexts::conditions::CreateEvaluateConditionsJobResponse,
            crate::db::contexts::ContextMode,
            crate::routes::contexts::jobs::ContextJobsResponse,
            crate::routes::contexts::jobs::ContextJob,
            crate::routes::contexts::jobs::ContextJobCursor,
            crate::routes::contexts::jobs::ContextJobsQuery,
            crate::routes::contexts::stats::ContextJobStatsResponse,
            crate::routes::contexts::stats::JobStatusCounts,
            crate::routes::contexts::stats::JobEngineCounts,
            crate::routes::contexts::UpdateContextRequest,
            crate::routes::contexts::CreateContextRequest,
            crate::routes::load::response::LoadContextResponse,
            crate::routes::load::response::LoadEntry,
            crate::routes::load::response::UploadedLocation,
            crate::routes::oneshots::CreateIngestBody,
            crate::routes::oneshots::CreateIngestJobResponse,
            crate::routes::load::response::TopicInfo,
            crate::routes::load::response::LoadContextForm,
            crate::routes::jobs::JobStatusResponse,
            crate::routes::jobs::JobStageResponse,
            crate::routes::jobs::DownloadQuery,
            crate::routes::pipelines::PipelineStatusResponse,
            crate::db::users::UserRole,
            crate::routes::auth::LoginRequest,
            crate::routes::auth::LoginResponse,
            crate::routes::admin::users::UserResponse,
            crate::routes::admin::users::UsersResponse,
            crate::routes::admin::users::UserCursor,
            crate::routes::admin::users::CreateUserRequest,
            crate::routes::admin::users::UpdateUserRequest,
            crate::routes::admin::files::AdminFilesResponse,
            crate::routes::admin::files::AdminFilesCursor,
            crate::routes::admin::files::AdminFileEntry,
            crate::routes::admin::files::RetryResponse,
            crate::routes::admin::contexts::ContextFilesResponse,
            crate::routes::admin::contexts::ContextFileEntry,
            crate::routes::admin::files::FileDetailsResponse,
            crate::routes::admin::files::FileContextAttachment,
            crate::routes::admin::connections::ConnectionEventsResponse,
            crate::routes::admin::connections::ServiceConnectionSummary,
            crate::routes::admin::connections::RecentConnectionEvent,
            crate::routes::search::SearchResponse,
            crate::routes::search::SearchResult,
            crate::routes::search::SearchEvidence,
            nauron_contracts::chat::ChatMessageRequest,
            nauron_contracts::chat::SseExample,
            nauron_contracts::conditions::ConditionEvaluationResponse,
            nauron_contracts::conditions::ConditionEvaluationResult,
            nauron_contracts::conditions::ConditionEvaluationOptions,
            nauron_contracts::conditions::ConditionMatch,
            nauron_contracts::conditions::ConditionRawEvidence,
            nauron_contracts::conditions::ConditionSpec,
            nauron_contracts::conditions::SeverityLevel,
            nauron_contracts::conditions::RiskLevel,
            crate::routes::settings::UserSettingsResponse,
            crate::routes::settings::UpdateUserSettingsRequest,
            crate::routes::shares::CreateShareRequest
        )
    ),
    security(
        ("bearerAuth" = [])
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Health", description = "Health checks for dependencies"),
        (name = "Contexts", description = "Context and file management"),
        (name = "Shares", description = "Context sharing"),
        (name = "Jobs", description = "Job status and artifacts"),
        (name = "Load", description = "Document ingestion"),
        (name = "Pipelines", description = "Pipeline lifecycle overview"),
        (name = "Settings", description = "User preferences"),
        (name = "Auth", description = "Authentication"),
        (name = "Users", description = "User management"),
        (name = "Admin", description = "Administrative endpoints"),
        (name = "Chat", description = "Chat embeddings APIs"),
        (name = "Conditions", description = "Condition evaluation on contexts")
    )
)]
pub struct GatewayApiDoc;

impl GatewayApiDoc {
    pub fn openapi_spec() -> utoipa::openapi::OpenApi {
        Self::openapi()
    }
}

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let bearer = SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer));
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme("bearerAuth", bearer);
        }
    }
}
