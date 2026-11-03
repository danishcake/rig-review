use std::string::FromUtf8Error;

use config::ConfigError;
use gitlab::{
    GitlabError, RestError,
    api::{ApiError, projects::repository::files::FileRawBuilderError},
};
use gix::Error as GixError;
use rig::{error::ProviderError, run::PromptError};
use serenity::Error as DiscordError;
use url::ParseError;

/// Top level error type. Large variants are boxed
#[derive(thiserror::Error, Debug)]
pub enum RigReviewError {
    #[error(transparent)]
    RigProviderError(#[from] Box<ProviderError>),

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
    DiscordApiError(#[from] Box<DiscordError>),

    #[error(transparent)]
    ConfigError(#[from] ConfigError),

    #[error("missing configuration key: {0}")]
    MissingConfiguration(String),

    #[error("merge request URL '{0}' did not match expected structure ")]
    MergeRequestUrlStructureIncorrect(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    PromptError(#[from] Box<PromptError>),

    #[error(transparent)]
    UrlParseError(#[from] ParseError),

    #[error("url invalid: {0}")]
    InvalidUrl(String),

    #[error("file is not part of the diff: {0}")]
    FileNotChanged(String),

    #[error("file could not be decoded as UTF-8: {0}")]
    FileNotUtf8Error(#[from] FromUtf8Error),
}

impl From<ProviderError> for RigReviewError {
    fn from(value: ProviderError) -> Self {
        Self::RigProviderError(Box::new(value))
    }
}

impl From<DiscordError> for RigReviewError {
    fn from(value: DiscordError) -> Self {
        Self::DiscordApiError(Box::new(value))
    }
}

impl From<PromptError> for RigReviewError {
    fn from(value: PromptError) -> Self {
        Self::PromptError(Box::new(value))
    }
}
