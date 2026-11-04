use tokio::io::{self, AsyncWriteExt};

use crate::{error::RigReviewError, sink::base::ReviewSink};

pub struct LocalStdoutSink {}

#[allow(clippy::new_without_default)]
impl LocalStdoutSink {
    pub fn new() -> Self {
        tracing::info!("Sink: stdout");
        LocalStdoutSink {}
    }
}

#[async_trait::async_trait]
impl ReviewSink for LocalStdoutSink {
    async fn output(&self, report: String) -> Result<(), RigReviewError> {
        let mut stdout = io::stdout();
        stdout.write_all(report.as_bytes()).await?;
        stdout.flush().await?;
        Ok(())
    }
}
