use std::path::{Path, PathBuf};

use config::{Environment, File};
use serde::Deserialize;

use crate::{
    error::RigReviewError, review::rig::config::RigReviewEngineConfig,
    source::gitlab::config::GitlabReviewSourceConfig,
};

#[derive(Debug, Deserialize, Default, Clone)]
pub struct NetworkConfig {
    /// Ignore invalid certificates
    #[serde(default)]
    pub ignore_certs: bool,

    /// Custom CA certificates to trust
    #[serde(default)]
    pub extra_ca: Vec<PathBuf>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    // TODO: Add logging
    pub rig_review_engine: RigReviewEngineConfig,
    pub gitlab: Option<GitlabReviewSourceConfig>,
    #[serde(default)]
    pub network: NetworkConfig,
}

impl Config {
    pub fn new(explicit_file: &Option<PathBuf>) -> Result<Self, RigReviewError> {
        let mut config_builder = config::Config::builder()
            // Load config from environment
            .add_source(Environment::with_prefix("RIG_REVIEW").prefix_separator("__"));

        // Load the local config file, if present
        if let Some(local_file) = Config::find_local_config() {
            config_builder = config_builder.add_source(File::from(local_file));
        }

        // Or from a manually specified file, if present
        if let Some(explicit_file) = explicit_file {
            config_builder = config_builder.add_source(File::from(explicit_file.clone()))
        }

        let config = config_builder.build()?;

        Ok(config.try_deserialize()?)
    }

    fn find_local_config() -> Option<PathBuf> {
        [Path::new("rig-review.toml"), Path::new("rig-review.json")]
            .into_iter()
            .find(|path| path.is_file())
            .map(Path::to_path_buf)
    }
}
