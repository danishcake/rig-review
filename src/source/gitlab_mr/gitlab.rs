use std::sync::Arc;

use crate::{
    error::RigReviewError,
    source::{base::ReviewSource, gitlab_mr::config::GitlabMergeRequestReviewSourceConfig},
};

use gitlab::{
    AsyncGitlab, GitlabBuilder, RestError,
    api::{
        self, ApiError, AsyncQuery,
        projects::{
            merge_requests::{MergeRequest, MergeRequestDiffs},
            repository::files::FileRaw,
        },
    },
};
use regex::Regex;
use serde::Deserialize;
use tokio::sync::{Mutex, OnceCell};
use url::Url;

pub struct GitlabMergeRequestReviewSource {
    project_path: String,
    mr_id: u64,
    client: Arc<Mutex<AsyncGitlab>>,

    // Cached requests
    changes: OnceCell<Vec<FileDiff>>,
    merge_request_metadata: OnceCell<MergeRequestMetadata>,
}

impl GitlabMergeRequestReviewSource {
    pub async fn new(
        mr_url: impl Into<String>,
        config: GitlabMergeRequestReviewSourceConfig,
    ) -> Result<Self, RigReviewError> {
        let base_url = Url::parse(&config.base_url)?;
        let api_url = base_url.join("/api/v4")?;

        let (project_path, mr_id) = Self::extract_project_id_and_mr_id(&base_url, mr_url)?;

        let client = GitlabBuilder::new(api_url, config.api_token.clone())
            .build_async()
            .await?;
        Ok(Self {
            project_path,
            mr_id,
            client: Arc::new(Mutex::new(client)),
            changes: OnceCell::new(),
            merge_request_metadata: OnceCell::new(),
        })
    }

    async fn get_mr_diff(&self) -> Result<&Vec<FileDiff>, ApiError<RestError>> {
        let client = self.client.lock().await;

        self.changes
            .get_or_try_init(async || {
                let endpoint: MergeRequestDiffs<'_> = MergeRequestDiffs::builder()
                    .project(&self.project_path)
                    .merge_request(self.mr_id)
                    .build()
                    .unwrap();

                let diffs: Result<Vec<FileDiff>, ApiError<RestError>> =
                    api::paged(endpoint, api::Pagination::All)
                        .query_async(&*client)
                        .await;
                diffs
            })
            .await
    }

    async fn get_mr_metadata(&self) -> Result<&MergeRequestMetadata, ApiError<RestError>> {
        let client = self.client.lock().await;

        self.merge_request_metadata
            .get_or_try_init(async || {
                let endpoint = MergeRequest::builder()
                    .project(&self.project_path)
                    .merge_request(self.mr_id)
                    .build()
                    .unwrap();

                let metadata: Result<MergeRequestMetadata, ApiError<RestError>> =
                    endpoint.query_async(&*client).await;
                metadata
            })
            .await
    }

    fn extract_project_id_and_mr_id(
        base_url: &Url,
        mr_url: impl Into<String>,
    ) -> Result<(String, u64), RigReviewError> {
        // The project is extracted from the mr_url which must be relative to the base URL
        let mr_url = Url::parse(&mr_url.into())?;
        let base_path = base_url.path();
        let mr_path = mr_url.path();
        let relative_path = mr_path.strip_prefix(base_path).ok_or(
            RigReviewError::MergeRequestOnDifferentServer(mr_url.to_string(), base_url.to_string()),
        )?;

        // This should have produced group/subgroup/project/-/merge_requests/{id}. Let's extract that
        let re = Regex::new(r"^(?P<project>.+)/-/merge_requests/(?P<iid>\d+)/?$").unwrap();
        let caps =
            re.captures(relative_path)
                .ok_or(RigReviewError::MergeRequestUrlStructureIncorrect(
                    relative_path.to_owned(),
                ))?;

        let project = &caps["project"];
        let iid: u64 = caps["iid"].parse().unwrap();
        Ok((project.to_owned(), iid))
    }
}

// Refer to https://docs.gitlab.com/api/merge_requests/ for response details

#[derive(Debug, Deserialize)]
struct FileDiff {
    old_path: String,
    new_path: String,
    diff: String,
}

#[derive(Debug, Deserialize)]
struct MergeRequestDiffRefs {
    base_sha: String,
    head_sha: String,
}

#[derive(Debug, Deserialize)]
struct MergeRequestMetadata {
    diff_refs: MergeRequestDiffRefs,
}

#[async_trait::async_trait]
impl ReviewSource for GitlabMergeRequestReviewSource {
    async fn changed_files(&self) -> Result<Vec<String>, RigReviewError> {
        let diffs = self.get_mr_diff().await?;
        Ok(diffs.iter().map(|diff| diff.new_path.clone()).collect())
    }

    async fn changes(&self, file: &str) -> Result<String, RigReviewError> {
        let mr_diff = self.get_mr_diff().await?;

        let file_diff =
            mr_diff
                .iter()
                .find(|f| f.new_path == file)
                .ok_or(RigReviewError::FileNotChanged(format!(
                    "{} not found in diff",
                    file
                )))?;

        Ok(file_diff.diff.clone())
    }

    async fn original_file(&self, file: &str) -> Result<String, RigReviewError> {
        // Determine the old path
        let mr_diff = self.get_mr_diff().await?;
        let old_path = mr_diff
            .iter()
            .find(|f| f.new_path == file)
            .ok_or(RigReviewError::FileNotChanged(format!(
                "{} not found in diff",
                file
            )))?
            .old_path
            .clone();

        // Determine the old commit hash
        let mr_metadata = self.get_mr_metadata().await?;
        let base_sha = mr_metadata.diff_refs.base_sha.clone();

        // Retrieve that file from that commit
        let endpoint = FileRaw::builder()
            .project(&self.project_path)
            .ref_(base_sha)
            .file_path(old_path)
            .build()?;

        let client = self.client.lock().await;
        let data: Vec<u8> = endpoint.query_async(&*client).await?;

        Ok(String::from_utf8(data)?)
    }

    async fn modified_file(&self, file: &str) -> Result<String, RigReviewError> {
        // Determine the modified commit hash
        let mr_metadata = self.get_mr_metadata().await?;
        let head_sha = mr_metadata.diff_refs.head_sha.clone();

        // Retrieve that file from that commit
        let endpoint = FileRaw::builder()
            .project(&self.project_path)
            .ref_(head_sha)
            .file_path(file)
            .build()?;

        let client = self.client.lock().await;
        let data: Vec<u8> = endpoint.query_async(&*client).await?;

        Ok(String::from_utf8(data)?)
    }
}
