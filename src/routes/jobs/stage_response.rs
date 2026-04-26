use serde::Serialize;
use utoipa::ToSchema;

use crate::db::jobs::{JobEngine, JobStage};

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct JobStageResponse {
    engine: JobEngine,
    value: String,
}

impl From<JobStage> for JobStageResponse {
    fn from(stage: JobStage) -> Self {
        let engine = JobEngine::from(stage);
        let value = stage_value(stage);
        Self { engine, value }
    }
}

fn stage_value(stage: JobStage) -> String {
    match stage {
        JobStage::Mir(stage) => stage.to_string(),
        JobStage::Rdf(stage) => stage.to_string(),
        JobStage::Ingest(stage) => stage.to_string(),
        JobStage::Conditions(stage) => stage.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use nauron_contracts::{IngestStage, MirStage, RdfStage};
    use serde_json::json;

    use super::*;

    #[test]
    fn serializes_mir_stage_as_tagged_object() {
        let stage = JobStageResponse::from(JobStage::from(MirStage::Completed));
        let encoded = serde_json::to_value(stage).expect("json");

        assert_eq!(encoded, json!({"engine": "mir", "value": "completed"}));
    }

    #[test]
    fn serializes_rdf_stage_as_tagged_object() {
        let stage = JobStageResponse::from(JobStage::from(RdfStage::InformationExtraction));
        let encoded = serde_json::to_value(stage).expect("json");

        assert_eq!(
            encoded,
            json!({"engine": "rdf", "value": "information_extraction"})
        );
    }

    #[test]
    fn serializes_ingest_stage_as_tagged_object() {
        let stage = JobStageResponse::from(JobStage::from(IngestStage::Queued));
        let encoded = serde_json::to_value(stage).expect("json");

        assert_eq!(encoded, json!({"engine": "ingest", "value": "queued"}));
    }
}
