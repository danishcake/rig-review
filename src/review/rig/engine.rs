use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::{
    error::RigReviewError,
    review::{
        base::ReviewEngine,
        rig::{
            config::RigReviewEngineConfig,
            hooks::LoggingHook,
            tools::{GetChangedFilesTool, GetFileDiffTool},
        },
    },
    sink::base::ReviewSink,
    source::base::ReviewSource,
};

use rig::{AgentBuilder, providers::openai::OpenAIConfig};

pub struct RigReviewEngine {
    config: RigReviewEngineConfig,
}

impl RigReviewEngine {
    pub fn new(config: &RigReviewEngineConfig) -> Self {
        RigReviewEngine {
            config: config.clone(),
        }
    }
}

#[async_trait]
impl ReviewEngine for RigReviewEngine {
    async fn review(
        &mut self,
        source: Arc<Mutex<dyn ReviewSource>>,
        sink: Arc<Mutex<dyn ReviewSink>>,
    ) -> Result<(), RigReviewError> {
        // Determine API token, falling back to 'unused'
        let api_token = self.config.api_token.clone().unwrap_or("unused".into());

        // Create client
        let client = OpenAIConfig::new(api_token)
            .with_base_url(&self.config.base_url)
            .client();

        // Get the model
        let model = client.chat(&self.config.model);

        // Create tools
        let get_files_tool = GetChangedFilesTool::new(source.clone());
        let get_file_diff_tool = GetFileDiffTool::new(source.clone());

        // Create hooks
        let logging_hook = LoggingHook::new();

        // Create agent
        let agent = AgentBuilder::new(model)
            .tool(get_files_tool)
            .tool(get_file_diff_tool)
            .add_hook(logging_hook)
            .preamble(include_str!("prompt.txt"))
            .default_max_turns(self.config.max_turns.unwrap_or(30))
            .build();

        // Perform the review
        let response = agent.prompt("Review changes").await?;
        let report = response.output();

        let sink = sink.lock().await;
        sink.output(report).await?;

        Ok(())
    }
}
