use std::num::NonZeroUsize;

use crate::util::serde_utils::deserialize_non_blank_string;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    /// The address the HTTP server binds to
    #[serde(default = "default_server_host")]
    pub host: String,

    /// The port the HTTP server listens on
    #[serde(default = "default_server_port")]
    pub port: u16,

    /// Shared secret expected in GitLab's X-Gitlab-Token header
    #[serde(deserialize_with = "deserialize_non_blank_string")]
    pub webhook_secret: String,

    /// Maximum number of waiting jobs
    #[serde(default = "default_server_queue_capacity")]
    pub queue_capacity: NonZeroUsize,

    /// Number of concurrent review workers
    #[serde(default = "default_server_worker_count")]
    pub worker_count: NonZeroUsize,

    /// Maximum number of recent jobs retained in memory
    #[serde(default = "default_server_job_history_limit")]
    pub job_history_limit: NonZeroUsize,
}

fn default_server_host() -> String {
    "127.0.0.1".to_owned()
}

fn default_server_port() -> u16 {
    8080
}

fn default_server_queue_capacity() -> NonZeroUsize {
    NonZeroUsize::new(10).unwrap()
}

fn default_server_worker_count() -> NonZeroUsize {
    NonZeroUsize::new(1).unwrap()
}

fn default_server_job_history_limit() -> NonZeroUsize {
    NonZeroUsize::new(1000).unwrap()
}
