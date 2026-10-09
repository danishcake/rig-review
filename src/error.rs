use rig::error::ProviderError;
use gix::Error as GixError;

#[derive(thiserror::Error, Debug)]
pub enum RigReviewError {
    #[error(transparent)]
    RigProviderError(#[from] ProviderError),

    #[error(transparent)]
    GitError(#[from] GixError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("file is not part of the diff: {0}")]
    FileNotChanged(String),
}