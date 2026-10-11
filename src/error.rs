use config::ConfigError;
use gix::Error as GixError;
use rig::{error::ProviderError, run::PromptError};

#[derive(thiserror::Error, Debug)]
pub enum RigReviewError {
    #[error(transparent)]
    RigProviderError(#[from] ProviderError),

    #[error(transparent)]
    GitError(#[from] GixError),

    #[error(transparent)]
    ConfigError(#[from] ConfigError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    PromptError(#[from] PromptError),

    #[error("file is not part of the diff: {0}")]
    FileNotChanged(String),

    #[error("mutex was poisoned")]
    MutexPoisoned,
}
