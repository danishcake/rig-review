//! The base source trait

use crate::error::RigReviewError;

pub trait ReviewSource {
    /// List changed files
    fn changed_files(&self) -> Vec<String>;

    /// List changes for a file, represented as a diff
    fn changes(&self, file: &str) -> Result<String, RigReviewError>;

    /// Gets the original file contents
    fn original_file(&self, file: &str) -> Result<String, RigReviewError>;

    /// Gets the modified file contents
    fn modified_file(&self, file: &str) -> Result<String, RigReviewError>;
}
