use serde::Deserialize;

/// Configuration for the GitlabMergeRequestReviewSource
#[derive(Debug, Deserialize, Clone)]
pub struct GitlabMergeRequestReviewSourceConfig {
    /// The base URL for the Gitlab instance
    pub base_url: String,

    /// The API token for the Gitlab host
    pub api_token: String,
}
