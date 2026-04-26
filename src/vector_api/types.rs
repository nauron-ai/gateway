use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::routes::contexts::graph::{EntityEntry, GraphEdge, GraphNode, GraphStats};

#[derive(Debug, Serialize)]
pub struct SemanticSearchFilters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SemanticSearchRequest {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<SemanticSearchFilters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_entity_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct SemanticSearchResponse {
    pub results: Vec<SemanticSearchResult>,
}

#[derive(Debug, Deserialize)]
pub struct SemanticSearchResult {
    pub score: f32,
    pub title: Option<String>,
    pub snippet: Option<String>,
    pub doc_id: Option<Uuid>,
    pub paragraph_id: Option<String>,
    pub context_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawGraphResponse {
    #[serde(default)]
    pub nodes: Vec<GraphNode>,
    #[serde(default)]
    pub edges: Vec<GraphEdge>,
    #[serde(default)]
    pub stats: GraphStats,
}

#[derive(Debug, Deserialize)]
pub struct RawEntitiesResponse {
    #[serde(default)]
    pub entities: Vec<EntityEntry>,
}
