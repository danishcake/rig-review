use std::io::Write;

use rig::{agent::AgentHook, streaming::Part};
use tracing::Level;

use crate::logging::is_level_enabled;

/// Logs significant events
pub struct LoggingHook {
    stream_state: std::sync::Mutex<StreamState>,
}

#[derive(Default)]
struct StreamState {
    active_part: Option<(Part, Level)>,
    pending_text_whitespace: String,
}

impl StreamState {
    fn collect_text_delta(&mut self, delta: &str) -> Option<String> {
        if delta.chars().all(char::is_whitespace) {
            self.pending_text_whitespace.push_str(delta);
            return None;
        }

        let mut output = std::mem::take(&mut self.pending_text_whitespace);
        output.push_str(delta);
        Some(output)
    }

    fn discard_pending_text_whitespace(&mut self) {
        self.pending_text_whitespace.clear();
    }
}

#[allow(clippy::new_without_default)]
impl LoggingHook {
    /// Constructor
    pub fn new() -> Self {
        LoggingHook {
            stream_state: std::sync::Mutex::new(StreamState::default()),
        }
    }

    /// Writes to stderr at the specified verbosity
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

    /// Enters a 'part' of the output - this can be things like reasoning, tool calls, text output
    /// If the type of the part changes then the label is output
    fn start_stream_part(&self, level: Level, part: Part, label: &str) {
        let mut state = self
            .stream_state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        // Do nothing if we're already in the same part
        if state.active_part == Some((part, level)) {
            return;
        }

        if !is_level_enabled(&level) {
            state.discard_pending_text_whitespace();
            return;
        }

        // If we're changing part, inject a newline between them
        let had_active_part = state.active_part.replace((part, level)).is_some();
        state.discard_pending_text_whitespace();
        if had_active_part {
            self.write_stderr(level, "\n");
        }
        self.write_stderr(level, &format!("[{label}] "));
    }

    /// Appends text, retaining whitespace-only deltas until meaningful text arrives.
    fn write_text_delta(&self, delta: &str) {
        let mut state = self
            .stream_state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let Some(output) = state.collect_text_delta(delta) else {
            return;
        };

        if state.active_part.take().is_some() {
            self.write_stderr(Level::INFO, &format!("\n{output}"));
        } else {
            self.write_stderr(Level::INFO, &output);
        }
    }

    fn finish_stream_part(&self) {
        let mut state = self
            .stream_state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.active_part = None;
        state.discard_pending_text_whitespace();
        self.write_stderr(Level::DEBUG, "\n");
    }
}

#[cfg(test)]
mod tests {
    use super::StreamState;

    #[test]
    fn whitespace_deltas_are_flushed_with_the_next_non_whitespace_delta() {
        let mut state = StreamState::default();

        assert_eq!(state.collect_text_delta("\n"), None);
        assert_eq!(state.collect_text_delta(" \t"), None);
        assert_eq!(
            state.collect_text_delta("Review"),
            Some("\n \tReview".into())
        );
        assert_eq!(state.collect_text_delta(" done"), Some(" done".into()));
    }

    #[test]
    fn pending_whitespace_can_be_discarded_at_a_part_boundary() {
        let mut state = StreamState::default();

        assert_eq!(state.collect_text_delta("\n\t"), None);
        state.discard_pending_text_whitespace();
        assert_eq!(state.collect_text_delta("Review"), Some("Review".into()));
    }
}

impl AgentHook for LoggingHook {
    async fn on_model_turn_finished(
        &self,
        _ctx: &rig::agent::HookContext,
        event: rig::agent::ModelTurnFinished<'_>,
    ) -> rig::agent::ModelTurnAction {
        self.finish_stream_part();
        tracing::debug!(
            turn = event.turn,
            total_tokens = ?event.usage.total_tokens,
            "Token usage for model turn"
        );
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
            Level::DEBUG,
            event.part,
            &format!("Tool call {}", event.tool_name),
        );
        self.write_stderr(Level::DEBUG, event.delta);
        rig::agent::ObservationAction::Continue
    }
}
