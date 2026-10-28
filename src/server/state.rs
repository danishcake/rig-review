use std::{
    collections::BTreeMap,
    sync::{Arc, atomic::AtomicU64},
    time::SystemTime,
};

use serde::Serialize;
use tokio::sync::{RwLock, mpsc};

use crate::source::gitlab::config::GitlabReviewSourceConfig;
use crate::util::serde_utils::{
    serialize_optional_system_time_as_posix_seconds, serialize_system_time_as_posix_seconds,
};

#[derive(Clone)]
pub(super) struct AppState {
    pub(super) gitlab_config: GitlabReviewSourceConfig,
    pub(super) jobs: Arc<RwLock<JobStore>>,
    pub(super) sender: mpsc::Sender<u64>,
    pub(super) next_job_id: Arc<AtomicU64>,
    pub(super) webhook_secret: Arc<str>,
}

#[derive(Default)]
pub(super) struct JobStore {
    pub(super) jobs: BTreeMap<u64, Job>,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct Job {
    pub(super) id: u64,
    pub(super) merge_request_url: String,
    pub(super) status: JobStatus,
    #[serde(serialize_with = "serialize_system_time_as_posix_seconds")]
    pub(super) created_at: SystemTime,
    #[serde(serialize_with = "serialize_optional_system_time_as_posix_seconds")]
    pub(super) started_at: Option<SystemTime>,
    #[serde(serialize_with = "serialize_optional_system_time_as_posix_seconds")]
    pub(super) completed_at: Option<SystemTime>,
    pub(super) error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum JobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
}
