//! The base review engine trait

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::{error::RigReviewError, sink::base::ReviewSink, source::base::ReviewSource};

#[async_trait]
pub trait ReviewEngine {
    async fn review(
        &mut self,
        source: Arc<Mutex<dyn ReviewSource>>,
        sink: Arc<Mutex<dyn ReviewSink>>,
    ) -> Result<(), RigReviewError>;
}
