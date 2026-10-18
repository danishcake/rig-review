use std::string::FromUtf8Error;

use config::ConfigError;
use gitlab::{
    GitlabError, RestError,
    api::{ApiError, projects::repository::files::FileRawBuilderError},
};
use gix::Error as GixError;
use rig::{error::ProviderError, run::PromptError};
use url::ParseError;

#[derive(thiserror::Error, Debug)]
pub enum RigReviewError {
    #[error(transparent)]
    RigProviderError(#[from] ProviderError),

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    #[error(transparent)]
    GitError(#[from] GixError),

    #[error(transparent)]
    GitlabError(#[from] GitlabError),

    #[error(transparent)]
    GitlabApiRestError(#[from] ApiError<RestError>),

    #[error(transparent)]
    GitlabApiRawError(#[from] FileRawBuilderError),

    #[error(transparent)]
    ConfigError(#[from] ConfigError),

    #[error("missing configuration key: {0}")]
    MissingConfiguration(String),

    #[error("merge request URL '{0}' did not match expected structure ")]
    MergeRequestUrlStructureIncorrect(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    PromptError(#[from] PromptError),

    #[error(transparent)]
    UrlParseError(#[from] ParseError),

    #[error("url invalid: {0}")]
    InvalidUrl(String),

    #[error("file is not part of the diff: {0}")]
    FileNotChanged(String),

    #[error("file could not be decoded as UTF-8: {0}")]
    FileNotUtf8Error(#[from] FromUtf8Error),
}
