use std::sync::Arc;

use gitlab::{
    AsyncGitlab,
    api::{AsyncQuery, projects::merge_requests::notes::CreateMergeRequestNote},
};
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::sink::base::ReviewSink;

pub struct GitlabReviewSink {
    project_path: String,
    mr_iid: u64,
    client: Arc<Mutex<AsyncGitlab>>,
}

impl GitlabReviewSink {
    pub fn new(
        mr_url: String,
        project_path: String,
        mr_iid: u64,
        client: Arc<Mutex<AsyncGitlab>>,
    ) -> Self {
        tracing::info!("Sink: {mr_url}");
        GitlabReviewSink {
            project_path,
            mr_iid,
            client,
        }
    }
}

#[async_trait::async_trait]
impl ReviewSink for GitlabReviewSink {
    async fn output(&self, report: String) -> Result<(), crate::error::RigReviewError> {
        // Post comment
        // https://docs.gitlab.com/api/notes/#create-a-merge-request-note
        let endpoint = CreateMergeRequestNote::builder()
            .project(&self.project_path)
            .merge_request(self.mr_iid)
            .body(report)
            .build()
            .unwrap();

        let client = self.client.lock().await;

        let _note: Note = endpoint.query_async(&*client).await?;
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
