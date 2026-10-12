//! The base source trait

use async_trait::async_trait;

use crate::error::RigReviewError;

/// A source of a diff to review
#[async_trait]
pub trait ReviewSource: Send + Sync {
    /// List changed files
    async fn changed_files(&self) -> Result<Vec<String>, RigReviewError>;

    /// List changes for a file, represented as a diff
    async fn changes(&self, file: &str) -> Result<String, RigReviewError>;

    /// Gets the original file contents
    async fn original_file(&self, file: &str) -> Result<String, RigReviewError>;

    /// Gets the modified file contents
    async fn modified_file(&self, file: &str) -> Result<String, RigReviewError>;
}
