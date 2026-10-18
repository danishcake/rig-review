use tokio::io::{self, AsyncWriteExt};

use crate::sink::base::ReviewSink;

pub struct LocalStdoutSink {}

impl LocalStdoutSink {
    pub fn new() -> Self {
        tracing::info!("Sink: stdout");
        LocalStdoutSink {}
    }
}

#[async_trait::async_trait]
impl ReviewSink for LocalStdoutSink {
    async fn output(&self, report: String) -> Result<(), crate::error::RigReviewError> {
        let mut stdout = io::stdout();
        stdout.write_all(report.as_bytes()).await?;
        stdout.flush().await?;
        Ok(())
    }
}
