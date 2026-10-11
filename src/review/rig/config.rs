use serde::Deserialize;

/// Configuration for the RigReviewEngine
#[derive(Debug, Deserialize, Clone)]
pub struct RigReviewEngineConfig {
    /// The base URL for the OpenAI compatible endpoint
    pub base_url: String,

    /// The API token for the OpenAI compatible endpoint
    pub api_token: Option<String>,

    /// The model to review with
    pub model: String,
}
