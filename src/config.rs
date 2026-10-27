use std::path::{Path, PathBuf};

use config::{Environment, File};
use serde::Deserialize;

use crate::{
    error::RigReviewError, review::rig::config::RigReviewEngineConfig,
    sink::discord::config::DiscordReviewSinkConfig,
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
    pub discord: Option<DiscordReviewSinkConfig>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn config_new_reads_network_config_from_environment() {
        const CHILD_PROCESS: &str = "RIG_REVIEW_CONFIG_ENV_TEST_CHILD";

        if std::env::var_os(CHILD_PROCESS).is_some() {
            let config = Config::new(&None).unwrap();
            assert!(config.network.ignore_certs);
            return;
        }

        // Make a temporary directory to execute tests in
        let unique_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let working_dir = std::env::temp_dir().join(format!(
            "rig-review-config-env-{}-{unique_id}",
            std::process::id()
        ));
        std::fs::create_dir(&working_dir).unwrap();

        // Write configuration file
        std::fs::write(
            working_dir.join("rig-review.json"),
            r#"{"rig_review_engine":{"base_url":"https://example.com","model":"test-model"}}"#,
        )
        .unwrap();

        // Run this test and capture the output. By setting CHILD_PROCESS to 1 we cause it
        // to actually run the test
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "config::tests::config_new_reads_network_config_from_environment",
                "--nocapture",
            ])
            .current_dir(&working_dir)
            .env(CHILD_PROCESS, "1")
            .env("RIG_REVIEW__network.ignore_certs", "true")
            .output()
            .unwrap();

        // Cleanup
        std::fs::remove_dir_all(&working_dir).unwrap();
        assert!(
            output.status.success(),
            "child test failed:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
