use super::{
    RawEntitiesResponse, RawGraphResponse, SemanticSearchFilters, SemanticSearchRequest,
    SemanticSearchResponse, VectorApiClientError,
};
use crate::db::connections::{ConnectionEventRepository, NewConnectionEvent};
use crate::routes::contexts::graph::EntitiesResponse;
use reqwest::{Client, Method, RequestBuilder, Response};
use serde::de::DeserializeOwned;
use std::time::{Duration, Instant};
use tracing::{instrument, warn};
use url::Url;

#[derive(Clone)]
pub struct VectorApiClient {
    client: Client,
    base_url: Url,
    connection_events: ConnectionEventRepository,
}

impl VectorApiClient {
    pub fn new(
        base_url: &str,
        connection_events: ConnectionEventRepository,
    ) -> Result<Self, VectorApiClientError> {
        let base_url = Url::parse(base_url)?;
        let client = Client::builder().timeout(Duration::from_secs(30)).build()?;
        Ok(Self {
            client,
            base_url,
            connection_events,
        })
    }

    #[instrument(name = "vector_api.health_check", skip(self))]
    pub async fn health_check(&self) -> Result<(), VectorApiClientError> {
        let url = self.base_url.join("/healthz")?;
        let builder = self.client.get(url);
        self.send_no_body(Method::GET, "/healthz", builder).await
    }

    pub async fn search(
        &self,
        query: &str,
        context_id: Option<i32>,
        limit: Option<i32>,
    ) -> Result<SemanticSearchResponse, VectorApiClientError> {
        let endpoint = "/tool/semantic-search";
        let url = self.base_url.join(endpoint)?;
        let payload = SemanticSearchRequest {
            query: query.to_string(),
            limit: limit.map(|value| value as u16),
            filters: Some(SemanticSearchFilters {
                context_id: context_id.map(|value| value.to_string()),
            }),
            preferred_entity_ids: None,
        };
        let builder = self.client.post(url).json(&payload);
        self.send_json(Method::POST, endpoint, builder).await
    }

    #[instrument(
        name = "vector_api.get_context_graph",
        skip(self),
        fields(context_id, limit, entity_types = tracing::field::Empty)
    )]
    pub async fn get_context_graph(
        &self,
        context_id: i32,
        limit: Option<u32>,
        entity_types: Option<Vec<String>>,
    ) -> Result<crate::routes::contexts::graph::GraphResponse, VectorApiClientError> {
        if let Some(ref types) = entity_types {
            tracing::Span::current().record("entity_types", types.join(",").as_str());
        }
        let endpoint = format!("/contexts/{context_id}/graph");
        let mut url = self.base_url.join(&endpoint)?;
        {
            let mut pairs = url.query_pairs_mut();
            if let Some(l) = limit {
                pairs.append_pair("limit", &l.to_string());
            }
            if let Some(ref types) = entity_types {
                pairs.append_pair("entity_types", &types.join(","));
            }
        }
        let builder = self.client.get(url);
        let raw: RawGraphResponse = self.send_json(Method::GET, &endpoint, builder).await?;
        Ok(crate::routes::contexts::graph::GraphResponse {
            context_id,
            nodes: raw.nodes,
            edges: raw.edges,
            stats: raw.stats,
        })
    }

    #[instrument(
        name = "vector_api.get_context_entities",
        skip(self),
        fields(context_id, entity_type = tracing::field::Empty, search = tracing::field::Empty, limit)
    )]
    pub async fn get_context_entities(
        &self,
        context_id: i32,
        entity_type: Option<String>,
        search: Option<String>,
        limit: Option<u32>,
    ) -> Result<EntitiesResponse, VectorApiClientError> {
        if let Some(ref t) = entity_type {
            tracing::Span::current().record("entity_type", t.as_str());
        }
        if let Some(ref s) = search {
            tracing::Span::current().record("search", s.as_str());
        }
        let endpoint = format!("/contexts/{context_id}/entities");
        let mut url = self.base_url.join(&endpoint)?;
        {
            let mut query_pairs = url.query_pairs_mut();
            if let Some(ref t) = entity_type {
                query_pairs.append_pair("type", t);
            }
            if let Some(ref s) = search {
                query_pairs.append_pair("search", s);
            }
            if let Some(l) = limit {
                query_pairs.append_pair("limit", &l.to_string());
            }
        }
        let builder = self.client.get(url);
        let raw: RawEntitiesResponse = self.send_json(Method::GET, &endpoint, builder).await?;
        Ok(EntitiesResponse {
            entities: raw.entities,
        })
    }

    async fn send_no_body(
        &self,
        method: Method,
        endpoint: &str,
        builder: RequestBuilder,
    ) -> Result<(), VectorApiClientError> {
        let (response, start) = self.send(endpoint, method.clone(), builder).await?;
        let status = response.status();
        let response_bytes = response.content_length().map(|value| value as i64);
        if !status.is_success() {
            warn!(%status, endpoint, "vector-api returned non-success status");
            self.record_event(
                endpoint,
                &method,
                start,
                status.as_u16() as i32,
                response_bytes,
                Some(format!("unexpected status: {status}")),
            );
            return Err(VectorApiClientError::UnexpectedStatus(status));
        }
        self.record_event(
            endpoint,
            &method,
            start,
            status.as_u16() as i32,
            response_bytes,
            None,
        );
        Ok(())
    }

    async fn send_json<T: DeserializeOwned>(
        &self,
        method: Method,
        endpoint: &str,
        builder: RequestBuilder,
    ) -> Result<T, VectorApiClientError> {
        let (response, start) = self.send(endpoint, method.clone(), builder).await?;
        let status = response.status();
        let response_bytes = response.content_length().map(|value| value as i64);
        if !status.is_success() {
            warn!(%status, endpoint, "vector-api returned non-success status");
            self.record_event(
                endpoint,
                &method,
                start,
                status.as_u16() as i32,
                response_bytes,
                Some(format!("unexpected status: {status}")),
            );
            return Err(VectorApiClientError::UnexpectedStatus(status));
        }

        let parsed = response.json().await.inspect_err(|err| {
            self.record_event(
                endpoint,
                &method,
                start,
                status.as_u16() as i32,
                response_bytes,
                Some(format!("decode error: {err}")),
            );
        })?;

        self.record_event(
            endpoint,
            &method,
            start,
            status.as_u16() as i32,
            response_bytes,
            None,
        );
        Ok(parsed)
    }

    async fn send(
        &self,
        endpoint: &str,
        method: Method,
        builder: RequestBuilder,
    ) -> Result<(Response, Instant), VectorApiClientError> {
        let start = Instant::now();
        let response = builder.send().await.inspect_err(|err| {
            warn!(error = %err, endpoint, "vector-api request failed");
            self.record_event(endpoint, &method, start, 0, None, Some(err.to_string()));
        })?;
        Ok((response, start))
    }

    fn record_event(
        &self,
        endpoint: &str,
        method: &Method,
        start: Instant,
        status: i32,
        response_bytes: Option<i64>,
        error: Option<String>,
    ) {
        let latency_ms = Self::elapsed_ms(start);
        self.connection_events.spawn_insert(NewConnectionEvent {
            service: "vector_api".to_string(),
            endpoint: endpoint.to_string(),
            method: method.as_str().to_string(),
            status,
            latency_ms,
            response_bytes,
            error,
        });
    }

    fn elapsed_ms(start: Instant) -> i32 {
        let millis = start.elapsed().as_millis();
        std::cmp::min(millis, i32::MAX as u128) as i32
    }
}
