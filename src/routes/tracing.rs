use axum::http::Request;
use tower_http::{
    classify::ServerErrorsFailureClass,
    trace::{MakeSpan, OnFailure, OnRequest, OnResponse, TraceLayer},
};
use tracing::{Level, Span};

pub fn build_trace_layer() -> TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    HealthAwareSpan,
    HealthAwareOnRequest,
    HealthAwareOnResponse,
    tower_http::trace::DefaultOnBodyChunk,
    tower_http::trace::DefaultOnEos,
    HealthAwareOnFailure,
> {
    TraceLayer::new_for_http()
        .make_span_with(HealthAwareSpan)
        .on_request(HealthAwareOnRequest)
        .on_response(HealthAwareOnResponse)
        .on_failure(HealthAwareOnFailure)
}

#[derive(Clone, Copy)]
pub struct HealthAwareSpan;

impl<B> MakeSpan<B> for HealthAwareSpan {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        let path = request.uri().path();
        if path == "/healthz" {
            return Span::none();
        }

        tracing::info_span!(
            "http.request",
            method = %request.method(),
            uri = %path,
            status = tracing::field::Empty,
            latency_ms = tracing::field::Empty,
        )
    }
}

#[derive(Clone, Copy)]
pub struct HealthAwareOnRequest;

impl<B> OnRequest<B> for HealthAwareOnRequest {
    fn on_request(&mut self, _request: &Request<B>, span: &Span) {
        if span.is_disabled() {
            return;
        }
        tracing::event!(parent: span, Level::INFO, "request started");
    }
}

#[derive(Clone, Copy)]
pub struct HealthAwareOnResponse;

impl<B> OnResponse<B> for HealthAwareOnResponse {
    fn on_response(
        self,
        response: &axum::http::Response<B>,
        latency: std::time::Duration,
        span: &Span,
    ) {
        if span.is_disabled() {
            return;
        }
        let status = response.status().as_u16();
        let latency_ms = latency.as_millis() as i64;
        span.record("status", status);
        span.record("latency_ms", latency_ms);
        tracing::event!(parent: span, Level::INFO, %status, latency_ms, "request finished");
    }
}

#[derive(Clone, Copy)]
pub struct HealthAwareOnFailure;

impl OnFailure<ServerErrorsFailureClass> for HealthAwareOnFailure {
    fn on_failure(
        &mut self,
        failure_classification: ServerErrorsFailureClass,
        latency: std::time::Duration,
        span: &Span,
    ) {
        if span.is_disabled() {
            return;
        }
        let latency_ms = latency.as_millis() as i64;
        span.record("latency_ms", latency_ms);
        tracing::event!(parent: span, Level::ERROR, %failure_classification, latency_ms, "request failed");
    }
}
