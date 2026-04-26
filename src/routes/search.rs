use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    auth::AuthUser, error::GatewayError, routes::contexts::ensure_context_owner, state::AppState,
    vector_api::SemanticSearchResult,
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/v1/search", get(global_search))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct SearchQuery {
    q: String,
    context_id: Option<i32>,
    limit: Option<u16>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SearchResponse {
    results: Vec<SearchResult>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SearchResult {
    context_id: i32,
    doc_id: Uuid,
    /// Deprecated compatibility alias equal to `doc_id`.
    file_id: Uuid,
    file_name: Option<String>,
    paragraph_id: Option<String>,
    snippet: Option<String>,
    relevance_score: f32,
    evidence: Option<SearchEvidence>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SearchEvidence {
    doc_id: Uuid,
    paragraph_id: Option<String>,
    snippet: Option<String>,
}

#[utoipa::path(
    get,
    path = "/v1/search",
    summary = "Semantic search across documents",
    description = "Performs semantic vector search within a context. Returns document fragments ranked by relevance \
to the query. Use for finding specific information in documents without conversational context.",
    params(SearchQuery),
    responses(
        (status = 200, description = "Search results", body = SearchResponse),
        (status = 400, description = "Invalid query", body = crate::error::ErrorResponse)
    ),
    tag = "Search"
)]
async fn global_search(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthUser>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<SearchResponse>, GatewayError> {
    let context_id = query.context_id.ok_or_else(|| GatewayError::InvalidField {
        field: "context_id".into(),
        message: "context_id is required for search".into(),
    })?;
    if query.q.trim().is_empty() {
        return Err(GatewayError::InvalidField {
            field: "q".into(),
            message: "query cannot be empty".into(),
        });
    }
    ensure_context_owner(&state, context_id, &user).await?;
    let limit = query.limit.unwrap_or(20);
    if limit == 0 {
        return Err(GatewayError::InvalidField {
            field: "limit".into(),
            message: "limit must be greater than zero".into(),
        });
    }
    let search_response = state
        .vector_api_client
        .search(&query.q, Some(context_id), Some(limit as i32))
        .await?;
    let results = map_results(search_response.results, context_id)?;
    Ok(Json(SearchResponse { results }))
}

fn map_results(
    results: Vec<SemanticSearchResult>,
    request_context_id: i32,
) -> Result<Vec<SearchResult>, GatewayError> {
    let mut mapped = Vec::new();

    for item in results {
        let context_id = match item.context_id.as_deref() {
            Some(raw) => raw.parse::<i32>().map_err(|_| GatewayError::InvalidField {
                field: "vector_api.context_id".into(),
                message: "vector search returned invalid context_id".into(),
            })?,
            None => request_context_id,
        };

        let Some(doc_id) = item.doc_id else {
            continue;
        };
        let paragraph_id = item.paragraph_id.clone();
        let snippet = item.snippet.clone();
        mapped.push(SearchResult {
            context_id,
            doc_id,
            file_id: doc_id,
            file_name: item.title.clone(),
            paragraph_id: paragraph_id.clone(),
            snippet: snippet.clone(),
            relevance_score: item.score,
            evidence: Some(SearchEvidence {
                doc_id,
                paragraph_id,
                snippet,
            }),
        });
    }

    Ok(mapped)
}

#[cfg(test)]
mod tests {
    use super::map_results;
    use crate::vector_api::SemanticSearchResult;
    use uuid::Uuid;

    #[test]
    fn map_results_exposes_doc_id_and_legacy_file_id_alias() {
        let doc_id = Uuid::parse_str("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee").expect("uuid");
        let results = map_results(
            vec![SemanticSearchResult {
                score: 0.9,
                title: Some("Contract".into()),
                snippet: Some("Clause text".into()),
                doc_id: Some(doc_id),
                paragraph_id: Some("p-1".into()),
                context_id: Some("42".into()),
            }],
            7,
        )
        .expect("valid vector search result");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].context_id, 42);
        assert_eq!(results[0].doc_id, doc_id);
        assert_eq!(results[0].file_id, doc_id);
        assert_eq!(results[0].evidence.as_ref().map(|e| e.doc_id), Some(doc_id));
    }

    #[test]
    fn map_results_skips_items_without_doc_id() {
        let results = map_results(
            vec![SemanticSearchResult {
                score: 0.9,
                title: None,
                snippet: None,
                doc_id: None,
                paragraph_id: Some("p-1".into()),
                context_id: None,
            }],
            7,
        )
        .expect("valid vector search result");

        assert!(results.is_empty());
    }

    #[test]
    fn map_results_rejects_invalid_context_id() {
        let doc_id = Uuid::parse_str("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee").expect("uuid");
        let result = map_results(
            vec![SemanticSearchResult {
                score: 0.9,
                title: None,
                snippet: None,
                doc_id: Some(doc_id),
                paragraph_id: None,
                context_id: Some("invalid".into()),
            }],
            7,
        );

        assert!(result.is_err());
    }
}
