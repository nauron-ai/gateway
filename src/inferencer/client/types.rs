use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct OneshotRequest {
    pub context_id: i64,
    pub prompt: String,
    pub language: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct TokensUsed {
    pub prompt: Option<u32>,
    pub completion: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct OneshotSuccessResponse {
    pub oneshot_id: Uuid,
    pub answer: String,
    pub language: String,
    pub tokens_used: Option<TokensUsed>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct OneshotFailureResponse {
    pub oneshot_id: Option<Uuid>,
    pub status: String,
    pub error: String,
}

#[derive(Debug, Clone)]
pub enum OneshotResult {
    Success(OneshotSuccessResponse),
    Failure(OneshotFailureResponse),
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct IngestSchemaField {
    pub key: String,
    pub name: Option<String>,
    pub description: String,
    pub r#type: Option<Value>,
    pub required: Option<bool>,
}
