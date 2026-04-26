use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{auth::AuthUser, error::GatewayError, state::AppState};

use super::utils::ensure_context_owner;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GraphQuery {
    /// Maximum number of nodes to return
    pub limit: Option<u32>,
    /// Comma-separated list of entity types to filter (e.g. "person,company,concept")
    pub entity_types: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct EntitiesQuery {
    /// Filter by entity type (e.g. "person", "company")
    #[serde(rename = "type")]
    pub entity_type: Option<String>,
    /// Text search within entity labels
    pub search: Option<String>,
    /// Maximum number of entities to return
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub relation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<f32>,
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct GraphStats {
    #[serde(default)]
    pub total_nodes: u32,
    #[serde(default)]
    pub total_edges: u32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GraphResponse {
    pub context_id: i32,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub stats: GraphStats,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EntityEntry {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub entity_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mention_count: Option<u32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EntitiesResponse {
    pub entities: Vec<EntityEntry>,
}

#[utoipa::path(
    get,
    path = "/v1/contexts/{context_id}/graph",
    summary = "Get knowledge graph",
    description = "Retrieves the RDF knowledge graph for a context. Returns nodes (entities) and edges (relationships) \
extracted from documents. Can filter by entity types and limit result size.",
    params(
        ("context_id" = i32, Path, description = "Context identifier"),
        GraphQuery
    ),
    responses(
        (status = 200, description = "RDF Knowledge Graph", body = GraphResponse),
        (status = 404, description = "Context not found", body = crate::error::ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Contexts"
)]
pub async fn get_context_graph(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthUser>,
    Path(context_id): Path<i32>,
    Query(query): Query<GraphQuery>,
) -> Result<Json<GraphResponse>, GatewayError> {
    ensure_context_owner(&state, context_id, &user).await?;

    let entity_types = query
        .entity_types
        .map(|s| s.split(',').map(|t| t.trim().to_string()).collect());

    let graph = state
        .vector_api_client
        .get_context_graph(context_id, query.limit, entity_types)
        .await?;

    Ok(Json(graph))
}

#[utoipa::path(
    get,
    path = "/v1/contexts/{context_id}/entities",
    summary = "List entities in knowledge graph",
    description = "Returns list of named entities extracted from context documents. \
Entities include people, organizations, concepts, locations, etc. Can filter by type and search within labels.",
    params(
        ("context_id" = i32, Path, description = "Context identifier"),
        EntitiesQuery
    ),
    responses(
        (status = 200, description = "List of entities", body = EntitiesResponse),
        (status = 404, description = "Context not found", body = crate::error::ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Contexts"
)]
pub async fn get_context_entities(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthUser>,
    Path(context_id): Path<i32>,
    Query(query): Query<EntitiesQuery>,
) -> Result<Json<EntitiesResponse>, GatewayError> {
    ensure_context_owner(&state, context_id, &user).await?;

    let entities = state
        .vector_api_client
        .get_context_entities(context_id, query.entity_type, query.search, query.limit)
        .await?;

    Ok(Json(entities))
}
