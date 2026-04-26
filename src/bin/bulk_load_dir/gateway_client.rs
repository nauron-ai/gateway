use serde::{Deserialize, Serialize};
use thiserror::Error;

const DEFAULT_TIMEOUT_SECS: u64 = 60;

#[derive(Debug, Error)]
pub enum GatewayClientError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("invalid base url")]
    InvalidBaseUrl,
}

#[derive(Debug, Clone)]
pub struct GatewayClient {
    base_url: String,
    http: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    token: String,
}

#[derive(Debug, Deserialize)]
struct CreateContextResponse {
    id: i32,
}

#[derive(Debug, Serialize)]
struct LoginRequest<'a> {
    email: &'a str,
    password: &'a str,
}

#[derive(Debug, Serialize)]
struct CreateContextRequest<'a> {
    mode: &'a str,
}

#[derive(Debug, Serialize)]
struct UpdateContextRequest<'a> {
    title: &'a str,
    description: &'a str,
}

impl GatewayClient {
    pub fn new(base_url: String) -> Result<Self, GatewayClientError> {
        let base = base_url.trim().trim_end_matches('/').to_string();
        if base.is_empty() {
            return Err(GatewayClientError::InvalidBaseUrl);
        }
        Ok(Self {
            base_url: base,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()?,
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<String, GatewayClientError> {
        let url = format!("{}/auth/login", self.base_url);
        let response = self
            .http
            .post(url)
            .json(&LoginRequest { email, password })
            .send()
            .await?
            .error_for_status()?;
        let body: LoginResponse = response.json().await?;
        Ok(body.token)
    }

    pub async fn create_context(&self, token: &str, mode: &str) -> Result<i32, GatewayClientError> {
        let url = format!("{}/v1/contexts", self.base_url);
        let response = self
            .http
            .post(url)
            .bearer_auth(token)
            .json(&CreateContextRequest { mode })
            .send()
            .await?
            .error_for_status()?;
        let body: CreateContextResponse = response.json().await?;
        Ok(body.id)
    }

    pub async fn update_context(
        &self,
        token: &str,
        context_id: i32,
        title: &str,
        description: &str,
    ) -> Result<(), GatewayClientError> {
        let url = format!("{}/v1/contexts/{}", self.base_url, context_id);
        self.http
            .patch(url)
            .bearer_auth(token)
            .json(&UpdateContextRequest { title, description })
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }
}
