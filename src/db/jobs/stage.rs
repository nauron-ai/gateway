use nauron_contracts::conditions::ConditionsEvaluateStage;
use nauron_contracts::{IngestStage, MirStage, RdfStage};

use super::model::JobEngine;

#[derive(Debug, Clone, Copy)]
pub enum JobStage {
    Mir(MirStage),
    Rdf(RdfStage),
    Ingest(IngestStage),
    Conditions(ConditionsEvaluateStage),
}

#[derive(Debug, Clone, Default)]
pub struct JobStageColumns {
    pub mir_stage: Option<MirStage>,
    pub rdf_stage: Option<RdfStage>,
    pub ingest_stage: Option<IngestStage>,
    pub conditions_stage: Option<ConditionsEvaluateStage>,
}

impl JobStage {
    pub fn from_columns(
        engine: JobEngine,
        mir_stage: Option<MirStage>,
        rdf_stage: Option<RdfStage>,
        ingest_stage: Option<IngestStage>,
        conditions_stage: Option<ConditionsEvaluateStage>,
    ) -> Option<Self> {
        match engine {
            JobEngine::Mir => mir_stage.map(Self::Mir),
            JobEngine::Rdf => rdf_stage.map(Self::Rdf),
            JobEngine::Ingest => ingest_stage.map(Self::Ingest),
            JobEngine::Conditions => conditions_stage.map(Self::Conditions),
            JobEngine::Lpg | JobEngine::Bayessian => None,
        }
    }

    pub fn columns(self) -> JobStageColumns {
        match self {
            Self::Mir(stage) => JobStageColumns {
                mir_stage: Some(stage),
                ..JobStageColumns::default()
            },
            Self::Rdf(stage) => JobStageColumns {
                rdf_stage: Some(stage),
                ..JobStageColumns::default()
            },
            Self::Ingest(stage) => JobStageColumns {
                ingest_stage: Some(stage),
                ..JobStageColumns::default()
            },
            Self::Conditions(stage) => JobStageColumns {
                conditions_stage: Some(stage),
                ..JobStageColumns::default()
            },
        }
    }
}

impl From<JobStage> for JobEngine {
    fn from(stage: JobStage) -> Self {
        match stage {
            JobStage::Mir(_) => Self::Mir,
            JobStage::Rdf(_) => Self::Rdf,
            JobStage::Ingest(_) => Self::Ingest,
            JobStage::Conditions(_) => Self::Conditions,
        }
    }
}

impl From<MirStage> for JobStage {
    fn from(stage: MirStage) -> Self {
        Self::Mir(stage)
    }
}

impl From<RdfStage> for JobStage {
    fn from(stage: RdfStage) -> Self {
        Self::Rdf(stage)
    }
}

impl From<IngestStage> for JobStage {
    fn from(stage: IngestStage) -> Self {
        Self::Ingest(stage)
    }
}

impl From<ConditionsEvaluateStage> for JobStage {
    fn from(stage: ConditionsEvaluateStage) -> Self {
        Self::Conditions(stage)
    }
}
