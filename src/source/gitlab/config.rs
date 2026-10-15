use serde::Deserialize;

/// Configuration for the GitlabMergeRequestReviewSource
#[derive(Debug, Deserialize, Clone)]
pub struct GitlabReviewSourceConfig {
    /// The host URL for the Gitlab instance. This is only required if you operate
    /// gitlab behind a reverse proxy, and use a custom path. For example, if you
    /// run gitlab on https://gitlab.my-org.lan then no configuration is required,
    /// but if you run it on https://my-org.lan/gitlab then you need to set that
    /// here.
    pub host: Option<String>,

    /// The API token for the Gitlab host
    pub api_token: String,
}
