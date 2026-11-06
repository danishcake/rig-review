use rig::serde_json::Value;
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

    /// The maximum number of turns
    #[serde(default = "default_max_turns")]
    pub max_turns: usize,

    /// Additional parameters to pass the the OpenAI endpoint
    /// For example, reasoning_effort = "low" can be used to control overthing by Qwen models
    pub additional_params: Option<Value>,

    /// If present, replaces the default prompt
    #[serde(default = "default_prompt")]
    pub prompt: String,

    /// If present, appended to the prompt
    pub extra_prompt: Option<String>,
}

fn default_prompt() -> String {
    include_str!("prompt.txt").into()
}

fn default_max_turns() -> usize {
    30
}
