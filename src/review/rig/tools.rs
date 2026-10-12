//! Tools available to the rig agent

use std::sync::Arc;

use rig::{
    serde_json::json,
    tool::{Tool, ToolContext, ToolExecutionError},
};
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::source::base::ReviewSource;

pub struct GetChangedFilesTool {
    review_source: Arc<Mutex<dyn ReviewSource>>,
}

impl GetChangedFilesTool {
    pub fn new(review_source: Arc<Mutex<dyn ReviewSource>>) -> Self {
        GetChangedFilesTool { review_source }
    }
}

#[derive(Deserialize)]
pub struct GetChangedFilesToolArgs {}

impl Tool for GetChangedFilesTool {
    const NAME: &'static str = "GetChangedFiles";
    type Args = GetChangedFilesToolArgs;
    type Output = Vec<String>;
    type Error = ToolExecutionError;

    fn description(&self) -> String {
        "Gets a list of files with changes. These can be inspected with the GetFileDiff tool".into()
    }

    fn parameters(&self) -> rig::serde_json::Value {
        json!({
            "type": "object",
            "properties": {
            },
            "required": []
        })
    }

    async fn call(
        &self,
        _context: &mut ToolContext,
        _args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let review_source = self.review_source.lock().await;
        review_source
            .changed_files()
            .await
            .map_err(|e| ToolExecutionError::from_error(e))
    }
}

pub struct GetFileDiffTool {
    review_source: Arc<Mutex<dyn ReviewSource>>,
}

#[derive(Deserialize)]
pub struct GetFillDiffToolArgs {
    file: String,
}

impl GetFileDiffTool {
    pub fn new(review_source: Arc<Mutex<dyn ReviewSource>>) -> Self {
        GetFileDiffTool { review_source }
    }
}

impl Tool for GetFileDiffTool {
    const NAME: &'static str = "GetFileDiff";
    type Args = GetFillDiffToolArgs;
    type Output = String;
    type Error = ToolExecutionError;

    fn description(&self) -> String {
        "Gets a diff for the given file".into()
    }

    fn parameters(&self) -> rig::serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "file": { "type": "string", "description": "a file as returned by a prior call to GetChangedFiles" },
            },
            "required": ["file"]
        })
    }

    async fn call(
        &self,
        _context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let review_source = self.review_source.lock().await;
        let changes = review_source
            .changes(&args.file)
            .await
            .map_err(|e| ToolExecutionError::from_error(e))?;
        Ok(changes)
    }
}
