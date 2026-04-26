use std::time::Instant;

use reqwest::{Method, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use tracing::warn;

use crate::inferencer::InferencerClientError;

use super::{InferencerClient, NewConnectionEvent};

impl InferencerClient {
    pub(crate) async fn send_json<T: DeserializeOwned>(
        &self,
        method: Method,
        endpoint: &str,
        builder: RequestBuilder,
    ) -> Result<T, InferencerClientError> {
        let (status, raw_body) = self
            .send_text_allowing(method, endpoint, builder, &[])
            .await?;

        debug_assert!(
            status.is_success(),
            "send_json should only succeed on 2xx statuses"
        );

        let parsed = serde_json::from_str(&raw_body).inspect_err(|err| {
            let snippet = truncate(&raw_body, 512);
            warn!(error = %err, endpoint, body_snippet = %snippet, "inferencer decode error");
        })?;
        Ok(parsed)
    }

    pub(crate) async fn send_json_allowing(
        &self,
        method: Method,
        endpoint: &str,
        builder: RequestBuilder,
        allowed_statuses: &[StatusCode],
    ) -> Result<(StatusCode, String), InferencerClientError> {
        self.send_text_allowing(method, endpoint, builder, allowed_statuses)
            .await
    }

    async fn send_text_allowing(
        &self,
        method: Method,
        endpoint: &str,
        builder: RequestBuilder,
        allowed_statuses: &[StatusCode],
    ) -> Result<(StatusCode, String), InferencerClientError> {
        self.check_circuit()?;

        let result = self
            .send_text_allowing_inner(method, endpoint, builder, allowed_statuses)
            .await;
        self.record_circuit_result(&result);
        result
    }

    async fn send_text_allowing_inner(
        &self,
        method: Method,
        endpoint: &str,
        builder: RequestBuilder,
        allowed_statuses: &[StatusCode],
    ) -> Result<(StatusCode, String), InferencerClientError> {
        let (response, start) = self.send(endpoint, method.clone(), builder).await?;
        let status = response.status();
        let response_bytes = response.content_length().map(|value| value as i64);
        let raw_body = response.text().await.inspect_err(|err| {
            self.record_event(
                endpoint,
                &method,
                start,
                status.as_u16() as i32,
                response_bytes,
                Some(format!("body read error: {err}")),
            );
            warn!(error = %err, endpoint, "inferencer body read error");
        })?;

        if !status.is_success() && !allowed_statuses.contains(&status) {
            let snippet = truncate(&raw_body, 512);
            warn!(%status, endpoint, body_snippet = %snippet, "inferencer non-success");
            self.record_event(
                endpoint,
                &method,
                start,
                status.as_u16() as i32,
                response_bytes,
                Some(format!("unexpected status: {status}; body: {snippet}")),
            );
            return Err(InferencerClientError::UnexpectedStatusBody(
                status, raw_body,
            ));
        }

        self.record_event(
            endpoint,
            &method,
            start,
            status.as_u16() as i32,
            response_bytes,
            None,
        );
        Ok((status, raw_body))
    }

    pub(crate) async fn send_stream(
        &self,
        method: Method,
        endpoint: &str,
        builder: RequestBuilder,
    ) -> Result<Response, InferencerClientError> {
        self.check_circuit()?;

        let result = self.send_stream_inner(method, endpoint, builder).await;
        self.record_circuit_result(&result);
        result
    }

    async fn send_stream_inner(
        &self,
        method: Method,
        endpoint: &str,
        builder: RequestBuilder,
    ) -> Result<Response, InferencerClientError> {
        let (response, start) = self.send(endpoint, method.clone(), builder).await?;
        let status = response.status();
        let response_bytes = response.content_length().map(|value| value as i64);
        if !status.is_success() {
            warn!(%status, endpoint, "inferencer returned non-success status");
            self.record_event(
                endpoint,
                &method,
                start,
                status.as_u16() as i32,
                response_bytes,
                Some(format!("unexpected status: {status}")),
            );
            return Err(InferencerClientError::UnexpectedStatus(status));
        }
        self.record_event(
            endpoint,
            &method,
            start,
            status.as_u16() as i32,
            response_bytes,
            None,
        );
        Ok(response)
    }

    async fn send(
        &self,
        endpoint: &str,
        method: Method,
        builder: RequestBuilder,
    ) -> Result<(Response, Instant), InferencerClientError> {
        let start = Instant::now();
        let response = builder.send().await.inspect_err(|err| {
            self.record_event(endpoint, &method, start, 0, None, Some(err.to_string()));
        })?;
        Ok((response, start))
    }

    fn record_circuit_result<T>(&self, result: &Result<T, InferencerClientError>) {
        match result {
            Ok(_) => self.circuit_breaker.record_success(),
            Err(e) if Self::is_transient_error(e) => self.circuit_breaker.record_failure(),
            Err(_) => {}
        }
    }

    fn is_transient_error(err: &InferencerClientError) -> bool {
        match err {
            InferencerClientError::Http(e) => e.is_connect() || e.is_timeout(),
            InferencerClientError::UnexpectedStatus(s) => s.is_server_error(),
            InferencerClientError::UnexpectedStatusBody(s, _) => s.is_server_error(),
            InferencerClientError::CircuitOpen
            | InferencerClientError::UrlParse(_)
            | InferencerClientError::Decode(_) => false,
        }
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
        let latency_ms = elapsed_ms(start);
        self.connection_events.spawn_insert(NewConnectionEvent {
            service: "inferencer".to_string(),
            endpoint: endpoint.to_string(),
            method: method.as_str().to_string(),
            status,
            latency_ms,
            response_bytes,
            error,
        });
    }
}

fn truncate(body: &str, max: usize) -> String {
    if body.len() <= max {
        body.to_string()
    } else {
        format!("{}...", &body[..max])
    }
}

fn elapsed_ms(start: Instant) -> i32 {
    let millis = start.elapsed().as_millis();
    std::cmp::min(millis, i32::MAX as u128) as i32
}
