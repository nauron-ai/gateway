use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::db::jobs::JobKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobLaunchMode {
    Started,
    Reused,
    Linked,
}

impl JobLaunchMode {
    pub fn from_kind(kind: Option<JobKind>) -> Self {
        match kind {
            None => JobLaunchMode::Started,
            Some(JobKind::Reused) => JobLaunchMode::Reused,
            Some(JobKind::MirLinked) => JobLaunchMode::Linked,
            Some(JobKind::Fanout | JobKind::Retry) => JobLaunchMode::Started,
        }
    }

    pub fn as_kind(self) -> Option<JobKind> {
        match self {
            JobLaunchMode::Started => None,
            JobLaunchMode::Reused => Some(JobKind::Reused),
            JobLaunchMode::Linked => Some(JobKind::MirLinked),
        }
    }
}
