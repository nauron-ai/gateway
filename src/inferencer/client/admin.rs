use nauron_contracts::chat::{
    AdminSessionsQuery, ReasoningResponse, SessionDetailResponse, SessionsResponse,
};
use reqwest::Method;
use url::Url;
use uuid::Uuid;

use super::InferencerClient;
use crate::inferencer::InferencerClientError;

impl InferencerClient {
    pub async fn list_sessions_admin(
        &self,
        query: &AdminSessionsQuery,
    ) -> Result<SessionsResponse, InferencerClientError> {
        let mut url = self.base_url.join("/chat/admin/sessions")?;
        append_admin_session_query_params(&mut url, query);
        let builder = self.client.get(url).header("X-Admin-Override", "true");
        self.send_json(Method::GET, "/chat/admin/sessions", builder)
            .await
    }

    pub async fn get_session_admin(
        &self,
        session_id: Uuid,
    ) -> Result<SessionDetailResponse, InferencerClientError> {
        let endpoint = format!("/chat/admin/sessions/{session_id}");
        let url = self.base_url.join(&endpoint)?;
        let builder = self.client.get(url).header("X-Admin-Override", "true");
        self.send_json(Method::GET, &endpoint, builder).await
    }

    pub async fn get_reasoning_admin(
        &self,
        session_id: Uuid,
        message_id: Uuid,
    ) -> Result<ReasoningResponse, InferencerClientError> {
        let endpoint = format!("/chat/admin/{session_id}/reasoning");
        let mut url = self.base_url.join(&endpoint)?;
        url.query_pairs_mut()
            .append_pair("message_id", &message_id.to_string());
        let builder = self.client.get(url).header("X-Admin-Override", "true");
        self.send_json(Method::GET, &endpoint, builder).await
    }
}

fn append_admin_session_query_params(url: &mut Url, query: &AdminSessionsQuery) {
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
    if let Some(user_id) = query.user_id {
        pairs.append_pair("user_id", &user_id.to_string());
    }
}
