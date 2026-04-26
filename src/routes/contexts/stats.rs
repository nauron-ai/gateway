use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{
    db::jobs::{JobEngine, JobEngineAggregate, JobStatus, JobStatusAggregate},
    error::{ErrorResponse, GatewayError},
    state::AppState,
};

#[utoipa::path(
    get,
    path = "/v1/contexts/{context_id}/jobs/stats",
    summary = "Get job statistics for context",
    description = "Returns aggregated statistics about jobs in this context: counts by status (pending, in_progress, success, failure) \
and by engine (MIR, RDF, LPG, Bayesian). Useful for monitoring processing progress.",
    params(
        ("context_id" = i32, Path, description = "Context identifier")
    ),
    responses(
        (status = 200, description = "Job statistics for the context", body = ContextJobStatsResponse),
        (status = 404, description = "Context not found", body = ErrorResponse)
    ),
    tag = "Jobs"
)]
pub async fn context_job_stats(
    Path(context_id): Path<i32>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, GatewayError> {
    let status_rows = state.job_repo.count_by_status(context_id).await?;
    let engine_rows = state.job_repo.count_by_engine(context_id).await?;

    let total: i64 = status_rows.iter().map(|row| row.count).sum();
    if total == 0 {
        let exists = state.context_repo.get(context_id).await?.is_some();
        if !exists {
            return Err(GatewayError::ContextNotFound(context_id));
        }
    }

    let status_counts = JobStatusCounts::from_rows(&status_rows);
    let engine_counts = JobEngineCounts::from_rows(&engine_rows);

    Ok(Json(ContextJobStatsResponse {
        context_id,
        total,
        status_counts,
        engine_counts,
    }))
}

#[derive(Serialize, ToSchema)]
pub struct ContextJobStatsResponse {
    context_id: i32,
    total: i64,
    status_counts: JobStatusCounts,
    engine_counts: JobEngineCounts,
}

#[derive(Default, Serialize, ToSchema)]
pub struct JobStatusCounts {
    pending: i64,
    in_progress: i64,
    success: i64,
    failure: i64,
    retryable: i64,
    retired: i64,
}

impl JobStatusCounts {
    fn from_rows(rows: &[JobStatusAggregate]) -> Self {
        let mut counts = JobStatusCounts::default();
        for row in rows {
            match row.status {
                JobStatus::Pending => counts.pending = row.count,
                JobStatus::InProgress => counts.in_progress = row.count,
                JobStatus::Success => counts.success = row.count,
                JobStatus::Failure => counts.failure = row.count,
                JobStatus::Retryable => counts.retryable = row.count,
                JobStatus::Retired => counts.retired = row.count,
            }
        }
        counts
    }
}

#[derive(Default, Serialize, ToSchema)]
pub struct JobEngineCounts {
    #[serde(skip_serializing_if = "is_zero")]
    mir: i64,
    #[serde(skip_serializing_if = "is_zero")]
    rdf: i64,
    #[serde(skip_serializing_if = "is_zero")]
    lpg: i64,
    #[serde(skip_serializing_if = "is_zero")]
    bayessian: i64,
    #[serde(skip_serializing_if = "is_zero")]
    ingest: i64,
    #[serde(skip_serializing_if = "is_zero")]
    conditions: i64,
}

impl JobEngineCounts {
    fn from_rows(rows: &[JobEngineAggregate]) -> Self {
        let mut counts = JobEngineCounts::default();
        for row in rows {
            match row.engine {
                JobEngine::Mir => counts.mir = row.count,
                JobEngine::Rdf => counts.rdf = row.count,
                JobEngine::Lpg => counts.lpg = row.count,
                JobEngine::Bayessian => counts.bayessian = row.count,
                JobEngine::Ingest => counts.ingest = row.count,
                JobEngine::Conditions => counts.conditions = row.count,
            }
        }
        counts
    }
}

fn is_zero(value: &i64) -> bool {
    *value == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_missing_statuses_with_zero() {
        let rows = vec![JobStatusAggregate {
            status: JobStatus::Success,
            count: 3,
        }];
        let counts = JobStatusCounts::from_rows(&rows);
        assert_eq!(counts.success, 3);
        assert_eq!(counts.pending, 0);
    }

    #[test]
    fn fills_engine_counts() {
        let rows = vec![
            JobEngineAggregate {
                engine: JobEngine::Mir,
                count: 10,
            },
            JobEngineAggregate {
                engine: JobEngine::Rdf,
                count: 2,
            },
        ];
        let counts = JobEngineCounts::from_rows(&rows);
        assert_eq!(counts.mir, 10);
        assert_eq!(counts.rdf, 2);
        assert_eq!(counts.lpg, 0);
    }
}
