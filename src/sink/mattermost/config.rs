use serde::Deserialize;

/// Configuration for the MattermostReviewSink
#[derive(Debug, Deserialize, Clone)]
pub struct MattermostSinkConfig {
    /// The URL of the Mattermost instance
    pub host: String,

    /// The access token used to authenticate with Mattermost
    pub access_token: String,

    /// The channel to post to
    pub channel_id: String,
}
