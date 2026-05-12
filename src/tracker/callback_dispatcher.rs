use std::time::Duration;

use nauron_contracts::conditions::{ConditionsEvaluateEvent, ConditionsEvaluateResult};
use nauron_contracts::{IngestEvent, IngestResult};
use reqwest::header::HeaderValue;
use serde::Serialize;
use uuid::Uuid;

use crate::db::job_callbacks::JobCallbackRepository;

use super::TrackerError;

const CALLBACK_SECRET_HEADER: &str = "x-nauron-callback-secret";

#[derive(Clone)]
pub(super) struct TrackerCallbackDispatcher {
    repo: JobCallbackRepository,
    http: reqwest::Client,
}

#[derive(Serialize)]
struct CallbackEnvelope<'a, T> {
    engine: &'static str,
    event_type: &'static str,
    job_id: Uuid,
    context_id: i32,
    event: &'a T,
}

impl TrackerCallbackDispatcher {
    pub(super) fn new(repo: JobCallbackRepository) -> Result<Self, TrackerError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;
        Ok(Self { repo, http })
    }

    pub(super) async fn dispatch_ingest(&self, event: &IngestEvent) -> Result<(), TrackerError> {
        let (job_id, context_id, event_type) = match event {
            IngestEvent::Progress(progress) => {
                (progress.job_id, progress.context_id, "ingest.progress")
            }
            IngestEvent::Result(result) => (
                ingest_result_job_id(result),
                ingest_result_context_id(result),
                "ingest.result",
            ),
        };

        let Some(target) = self.repo.get(job_id).await? else {
            return Ok(());
        };

        let body = CallbackEnvelope {
            engine: "ingest",
            event_type,
            job_id,
            context_id,
            event,
        };

        let mut request = self.http.post(target.url).json(&body);
        if let Some(secret) = target.secret {
            if let Ok(header) = HeaderValue::from_str(secret.as_str()) {
                request = request.header(CALLBACK_SECRET_HEADER, header);
            }
        }
        request.send().await?.error_for_status()?;
        Ok(())
    }

    pub(super) async fn dispatch_conditions(
        &self,
        event: &ConditionsEvaluateEvent,
    ) -> Result<(), TrackerError> {
        let (job_id, context_id, event_type) = match event {
            ConditionsEvaluateEvent::Progress(progress) => {
                (progress.job_id, progress.context_id, "conditions.progress")
            }
            ConditionsEvaluateEvent::Result(result) => (
                conditions_result_job_id(result),
                conditions_result_context_id(result),
                "conditions.result",
            ),
        };

        let Some(target) = self.repo.get(job_id).await? else {
            return Ok(());
        };

        let body = CallbackEnvelope {
            engine: "conditions",
            event_type,
            job_id,
            context_id,
            event,
        };

        let mut request = self.http.post(target.url).json(&body);
        if let Some(secret) = target.secret {
            if let Ok(header) = HeaderValue::from_str(secret.as_str()) {
                request = request.header(CALLBACK_SECRET_HEADER, header);
            }
        }
        request.send().await?.error_for_status()?;
        Ok(())
    }
}

fn ingest_result_job_id(result: &IngestResult) -> Uuid {
    match result {
        IngestResult::Success { job_id, .. } | IngestResult::Failure { job_id, .. } => *job_id,
    }
}

fn ingest_result_context_id(result: &IngestResult) -> i32 {
    match result {
        IngestResult::Success { context_id, .. } | IngestResult::Failure { context_id, .. } => {
            *context_id
        }
    }
}

fn conditions_result_job_id(result: &ConditionsEvaluateResult) -> Uuid {
    match result {
        ConditionsEvaluateResult::Success { job_id, .. }
        | ConditionsEvaluateResult::Failure { job_id, .. } => *job_id,
    }
}

fn conditions_result_context_id(result: &ConditionsEvaluateResult) -> i32 {
    match result {
        ConditionsEvaluateResult::Success { context_id, .. }
        | ConditionsEvaluateResult::Failure { context_id, .. } => *context_id,
    }
}
