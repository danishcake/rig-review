use serenity::{all::Http, json::json};

use crate::{
    error::RigReviewError,
    sink::{base::ReviewSink, discord::config::DiscordSinkConfig},
};

pub struct DiscordSink {
    config: DiscordSinkConfig,
}

impl DiscordSink {
    pub fn new(config: &Option<DiscordSinkConfig>) -> Result<Self, RigReviewError> {
        let Some(config) = config else {
            return Err(RigReviewError::MissingConfiguration("discord".into()));
        };

        tracing::info!("Sink: discord");
        Ok(Self {
            config: config.clone(),
        })
    }
}

#[async_trait::async_trait]
impl ReviewSink for DiscordSink {
    async fn output(&self, report: String) -> Result<(), RigReviewError> {
        let http = Http::new("");
        let map = json!({"content": report});
        http.execute_webhook(
            self.config.webhook_id.into(),
            None,
            &self.config.webhook_token,
            false,
            vec![],
            &map,
        )
        .await?;
        Ok(())
    }
}
