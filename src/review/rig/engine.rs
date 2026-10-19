use std::sync::Arc;

use async_trait::async_trait;
use futures_util::StreamExt;
use tokio::sync::Mutex;

use crate::{
    config::NetworkConfig,
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
    util::reqwest_utils::load_certificates,
};

use rig::{AgentBuilder, providers::openai::OpenAIConfig};
use rig::{ProviderError, agent::MultiTurnStreamItem};

pub struct RigReviewEngine {
    config: RigReviewEngineConfig,
    network_config: NetworkConfig,
}

impl RigReviewEngine {
    pub fn new(config: &RigReviewEngineConfig, network_config: &NetworkConfig) -> Self {
        RigReviewEngine {
            config: config.clone(),
            network_config: network_config.clone(),
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

        // Load the additional certificates
        let extra_certificates = load_certificates(&self.network_config)?;

        // Create HTTP client
        let http_client = reqwest::Client::builder()
            .danger_accept_invalid_certs(self.network_config.ignore_certs)
            .tls_certs_merge(extra_certificates)
            .build()?;
        // Wrap that in the rig wrapper
        let http_client: rig::rig_reqwest::ReqwestClient = http_client.into();

        // Create client
        let client = OpenAIConfig::new(api_token)
            .with_base_url(&self.config.base_url)
            .client()
            .with_http(http_client);

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

        // Perform the review. We do this in a streaming fashion so we can observe the tokens as they
        // arrive. Only the final turn
        let mut stream = agent.prompt("Review changes").stream();
        let mut response = None;

        // Iterate until we get the final reponse
        while let Some(item) = stream.next().await {
            match item? {
                MultiTurnStreamItem::FinalResponse(final_response) => {
                    response = Some(final_response);
                    break;
                }
                _ => {}
            }
        }

        let report = response
            .ok_or_else(|| {
                ProviderError::Response("stream ended before a final response was received".into())
            })?
            .output();

        let sink = sink.lock().await;
        sink.output(report).await?;

        Ok(())
    }
}
