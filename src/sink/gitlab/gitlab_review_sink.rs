use std::sync::Arc;

use gitlab::{
    AsyncGitlab,
    api::{AsyncQuery, projects::merge_requests::notes::CreateMergeRequestNote},
};
use serde::Deserialize;
use tokio::sync::Mutex;
use tokio_retry2::{Retry, RetryError};

use crate::{
    error::RigReviewError, sink::base::ReviewSink, util::tokio_retry2_utils::RetryStrategyFactory,
};

pub struct GitlabReviewSink {
    project_path: String,
    mr_iid: u64,
    client: Arc<Mutex<AsyncGitlab>>,

    // Failed request retry strategy
    retry_strategy_factory: RetryStrategyFactory,
}

impl GitlabReviewSink {
    pub fn new(
        mr_url: String,
        project_path: String,
        mr_iid: u64,
        client: Arc<Mutex<AsyncGitlab>>,
        retry_strategy_factory: RetryStrategyFactory,
    ) -> Self {
        tracing::info!("Sink: {mr_url}");
        GitlabReviewSink {
            project_path,
            mr_iid,
            client,
            retry_strategy_factory,
        }
    }
}

#[async_trait::async_trait]
impl ReviewSink for GitlabReviewSink {
    async fn output(&self, report: String) -> Result<(), RigReviewError> {
        // Post comment
        // https://docs.gitlab.com/api/notes/#create-a-merge-request-note
        let endpoint = CreateMergeRequestNote::builder()
            .project(&self.project_path)
            .merge_request(self.mr_iid)
            .body(report)
            .build()
            .unwrap();

        let retry_strategy = (self.retry_strategy_factory)();
        let _note: Note = Retry::spawn(retry_strategy, || async {
            let client = self.client.lock().await;
            endpoint
                .query_async(&*client)
                .await
                .map_err(RetryError::transient)
        })
        .await?;

        Ok(())
    }
}

/// Response to note creation
/// https://docs.gitlab.com/api/notes/#create-a-merge-request-note
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Note {
    id: u64,
    body: String,
}
