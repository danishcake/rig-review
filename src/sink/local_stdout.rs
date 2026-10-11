use crate::sink::base::ReviewSink;

pub struct LocalStdoutSink {}

impl LocalStdoutSink {
    pub fn new() -> Self {
        LocalStdoutSink {}
    }
}

#[async_trait::async_trait]
impl ReviewSink for LocalStdoutSink {
    async fn output(&self, report: String) -> Result<(), crate::error::RigReviewError> {
        print!("{report}");
        Ok(())
    }
}
