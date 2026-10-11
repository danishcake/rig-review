//! The base sink trait

use async_trait::async_trait;

use crate::error::RigReviewError;

/// A destination of review
#[async_trait]
pub trait ReviewSink: Send + Sync {
    /// Add to the output
    async fn output(&self, report: String) -> Result<(), RigReviewError>;
}
