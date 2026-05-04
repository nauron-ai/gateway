mod transport;
mod types;

use std::sync::Arc;
use std::time::Duration;

use crate::db::connections::{ConnectionEventRepository, NewConnectionEvent};
use crate::inferencer::{CircuitBreaker, CircuitBreakerConfig, InferencerClientError};
use nauron_contracts::health::HealthResponse;
use reqwest::{Client, Method};
use serde_json::Value;
use url::Url;

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
}
