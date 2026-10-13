use std::io::Write;

use rig::{agent::AgentHook, streaming::Part};
use tracing::Level;

use crate::logging::is_level_enabled;

/// Logs significant events
pub struct LoggingHook {
    active_stream_part: std::sync::Mutex<Option<Part>>,
}

impl LoggingHook {
    pub fn new() -> Self {
        LoggingHook {
            active_stream_part: std::sync::Mutex::new(None),
        }
    }

    fn write_stderr(&self, level: Level, output: &str) {
        if !is_level_enabled(&level) {
            return;
        }

        let mut stderr = std::io::stderr().lock();
        if let Err(error) = stderr
            .write_all(output.as_bytes())
            .and_then(|()| stderr.flush())
        {
            tracing::error!("Failed to write streamed output to stderr: {error}");
        }
    }

    fn start_stream_part(&self, level: Level, part: Part, label: &str) {
        if !is_level_enabled(&level) {
            return;
        }

        let mut active_part = self
            .active_stream_part
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if *active_part == Some(part) {
            return;
        }

        *active_part = Some(part);
        self.write_stderr(Level::DEBUG, &format!("\n[{label}] "));
    }

    fn write_text_delta(&self, delta: &str) {
        let mut active_part = self
            .active_stream_part
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if active_part.take().is_some() {
            self.write_stderr(Level::INFO, &format!("\n{delta}"));
        } else {
            self.write_stderr(Level::INFO, delta);
        }
    }

    fn finish_stream_part(&self) {
        let mut active_part = self
            .active_stream_part
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *active_part = None;
        self.write_stderr(Level::DEBUG, "\n");
    }
}

impl AgentHook for LoggingHook {
    async fn on_model_turn_finished(
        &self,
        _ctx: &rig::agent::HookContext,
        event: rig::agent::ModelTurnFinished<'_>,
    ) -> rig::agent::ModelTurnAction {
        self.finish_stream_part();
        tracing::trace!("Model turn finished: {:?}", event.content);
        rig::agent::ModelTurnAction::Continue
    }

    async fn on_invalid_tool_call(
        &self,
        _ctx: &rig::agent::HookContext,
        event: &rig::run::InvalidToolCallContext,
    ) -> Option<rig::run::InvalidToolCallAction> {
        tracing::error!("Invalid tool call {:?}", event);
        None
    }

    async fn on_text_delta(
        &self,
        _ctx: &rig::agent::HookContext,
        event: rig::agent::TextDelta<'_>,
    ) -> rig::agent::ObservationAction {
        self.write_text_delta(event.delta);
        rig::agent::ObservationAction::Continue
    }

    async fn on_reasoning_delta(
        &self,
        _ctx: &rig::agent::HookContext,
        event: rig::agent::ReasoningDelta<'_>,
    ) -> rig::agent::ObservationAction {
        self.start_stream_part(Level::INFO, event.part, "Thinking");
        self.write_stderr(Level::INFO, event.delta);
        rig::agent::ObservationAction::Continue
    }

    async fn on_tool_call_delta(
        &self,
        _ctx: &rig::agent::HookContext,
        event: rig::agent::ToolCallDelta<'_>,
    ) -> rig::agent::ObservationAction {
        self.start_stream_part(
            Level::INFO,
            event.part,
            &format!("Tool call {}", event.tool_name),
        );
        self.write_stderr(Level::DEBUG, event.delta);
        rig::agent::ObservationAction::Continue
    }
}
