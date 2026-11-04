use mattermost_api::{
    client::{AuthenticationData, Mattermost},
    models::PostBody,
};

use crate::{
    error::RigReviewError,
    sink::{base::ReviewSink, mattermost::config::MattermostSinkConfig},
};

pub struct MattermostSink {
    config: MattermostSinkConfig,
    api: Mattermost,
}

impl MattermostSink {
    pub fn new(config: &Option<MattermostSinkConfig>) -> Result<Self, RigReviewError> {
        let Some(config) = config else {
            return Err(RigReviewError::MissingConfiguration("mattermost".into()));
        };

        tracing::info!("Sink: mattermost");

        let auth = AuthenticationData::from_access_token(&config.access_token);
        let api = Mattermost::new(&config.host, auth)?;

        // The Github code has a 'with_client' method that would allow us to ignore certificates, but
        // it's not in the version published to crates.io

        Ok(Self {
            config: config.clone(),
            api,
        })
    }
}

#[async_trait::async_trait]
impl ReviewSink for MattermostSink {
    async fn output(&self, report: String) -> Result<(), RigReviewError> {
        let body = PostBody {
            channel_id: self.config.channel_id.clone(),
            message: report,
            root_id: None,
        };
        let _post = self.api.create_post(&body).await?;
        Ok(())
    }
}
