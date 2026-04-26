mod admin;
mod transport;
mod types;

use std::sync::Arc;
use std::time::Duration;

use crate::db::connections::{ConnectionEventRepository, NewConnectionEvent};
use crate::inferencer::{CircuitBreaker, CircuitBreakerConfig, InferencerClientError};
use nauron_contracts::chat::{
    ReasoningResponse, SessionDetailResponse, SessionsQuery, SessionsResponse,
};
use nauron_contracts::health::HealthResponse;
use reqwest::{Client, Method, Response};
use serde_json::Value;
use url::Url;
use uuid::Uuid;

pub use types::*;

#[derive(Clone)]
pub struct InferencerClient {
    client: Client,
    base_url: Url,
    connection_events: ConnectionEventRepository,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl InferencerClient {
    pub fn new(
        base_url: &str,
        connection_events: ConnectionEventRepository,
        cb_config: CircuitBreakerConfig,
    ) -> Result<Self, InferencerClientError> {
        let base_url = Url::parse(base_url)?;
        let client = Client::builder()
            .timeout(Duration::from_secs(15 * 60))
            .build()?;
        Ok(Self {
            client,
            base_url,
            connection_events,
            circuit_breaker: Arc::new(CircuitBreaker::new(cb_config)),
        })
    }

    fn check_circuit(&self) -> Result<(), InferencerClientError> {
        if !self.circuit_breaker.allow_request() {
            return Err(InferencerClientError::CircuitOpen);
        }
        Ok(())
    }

    pub async fn check_health(&self) -> Result<HealthResponse<Value>, InferencerClientError> {
        let url = self.base_url.join("/healthz")?;
        let builder = self.client.get(url);
        self.send_json(Method::GET, "/healthz", builder).await
    }

    pub async fn chat_emb(
        &self,
        request: &nauron_contracts::chat::ChatMessageRequest,
        user_id: &str,
    ) -> Result<Response, InferencerClientError> {
        self.post_streaming("/chat/emb/message", request, user_id)
            .await
    }

    pub async fn chat_rdf_emb(
        &self,
        request: &nauron_contracts::chat::ChatMessageRequest,
        user_id: &str,
    ) -> Result<Response, InferencerClientError> {
        self.post_streaming("/chat/rdf-emb/message", request, user_id)
            .await
    }

    pub async fn chat_bn(
        &self,
        request: &nauron_contracts::chat::ChatMessageRequest,
        user_id: &str,
    ) -> Result<Response, InferencerClientError> {
        self.post_streaming("/chat/bn/message", request, user_id)
            .await
    }

    pub async fn list_sessions(
        &self,
        user_id: &str,
        query: &SessionsQuery,
    ) -> Result<SessionsResponse, InferencerClientError> {
        let mut url = self.base_url.join("/chat/sessions")?;
        append_session_query_params(&mut url, query);
        let builder = self.client.get(url).header("X-User-Id", user_id);
        self.send_json(Method::GET, "/chat/sessions", builder).await
    }

    pub async fn get_session(
        &self,
        user_id: &str,
        session_id: Uuid,
    ) -> Result<SessionDetailResponse, InferencerClientError> {
        let endpoint = format!("/chat/sessions/{session_id}");
        let url = self.base_url.join(&endpoint)?;
        let builder = self.client.get(url).header("X-User-Id", user_id);
        self.send_json(Method::GET, &endpoint, builder).await
    }

    pub async fn get_reasoning(
        &self,
        user_id: &str,
        session_id: Uuid,
        message_id: Uuid,
    ) -> Result<ReasoningResponse, InferencerClientError> {
        let endpoint = format!("/chat/{}/reasoning", session_id);
        let mut url = self.base_url.join(&endpoint)?;
        url.query_pairs_mut()
            .append_pair("message_id", &message_id.to_string());
        let builder = self.client.get(url).header("X-User-Id", user_id);
        self.send_json(Method::GET, &endpoint, builder).await
    }

    pub async fn oneshot(
        &self,
        request: &OneshotRequest,
        user_id: &str,
    ) -> Result<OneshotResult, InferencerClientError> {
        let endpoint = "/oneshots";
        let url = self.base_url.join(endpoint)?;
        let builder = self
            .client
            .post(url)
            .header("X-User-Id", user_id)
            .json(request);

        let (status, body) = self
            .send_json_allowing(
                Method::POST,
                endpoint,
                builder,
                &[reqwest::StatusCode::BAD_REQUEST],
            )
            .await?;

        if status.is_success() {
            let parsed: OneshotSuccessResponse = serde_json::from_str(&body)?;
            return Ok(OneshotResult::Success(parsed));
        }

        let parsed: OneshotFailureResponse = serde_json::from_str(&body)?;
        Ok(OneshotResult::Failure(parsed))
    }

    async fn post_streaming(
        &self,
        endpoint: &str,
        request: &nauron_contracts::chat::ChatMessageRequest,
        user_id: &str,
    ) -> Result<Response, InferencerClientError> {
        let url = self.base_url.join(endpoint)?;
        let builder = self
            .client
            .post(url)
            .header("X-User-Id", user_id)
            .json(request);
        self.send_stream(Method::POST, endpoint, builder).await
    }
}

fn append_session_query_params(url: &mut Url, query: &SessionsQuery) {
    let mut pairs = url.query_pairs_mut();
    if let Some(ctx) = query.context_id {
        pairs.append_pair("context_id", &ctx.to_string());
    }
    if let Some(lim) = query.limit {
        pairs.append_pair("limit", &lim.to_string());
    }
    if let Some(ts) = query.cursor_updated_at {
        pairs.append_pair("cursor_updated_at", &ts.to_rfc3339());
    }
    if let Some(id) = query.cursor_id {
        pairs.append_pair("cursor_id", &id.to_string());
    }
}
