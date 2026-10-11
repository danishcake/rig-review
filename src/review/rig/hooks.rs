use rig::{
    agent::AgentHook,
    message::{AssistantMessage, Message, UserContent},
};

/// Logs significant events
pub struct LoggingHook {}

impl LoggingHook {
    pub fn new() -> Self {
        LoggingHook {}
    }
}

impl AgentHook for LoggingHook {
    async fn on_completion_call(
        &self,
        _ctx: &rig::agent::HookContext,
        event: rig::agent::CompletionCallEvent<'_>,
    ) -> rig::agent::CompletionCallAction {
        match &event.prompt {
            Message::User { content } => {
                let content: Vec<_> = content
                    .iter()
                    .map(|f| match f {
                        UserContent::Text(text) => text.text.clone(),
                        other => format!("{:?}", other),
                    })
                    .collect();

                tracing::debug!("Completion call user: {:?}", content.join(","));
            }
            Message::Assistant(AssistantMessage { content, .. }) => {
                tracing::debug!("Completion call assistant: {:?}", content);
            }
            Message::System { content } => {
                tracing::debug!("Completion call system: {:?}", content);
            }
        };

        rig::agent::CompletionCallAction::Continue
    }

    async fn on_model_turn_finished(
        &self,
        _ctx: &rig::agent::HookContext,
        event: rig::agent::ModelTurnFinished<'_>,
    ) -> rig::agent::ModelTurnAction {
        tracing::debug!("Model turn finished: {:?}", event.content);
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

    // async fn on_text_delta(
    //     &self,
    //     _ctx: &rig::agent::HookContext,
    //     _event: rig::agent::TextDelta<'_>,
    // ) -> rig::agent::ObservationAction {
    //     rig::agent::ObservationAction::Continue
    // }

    // async fn on_reasoning_delta(
    //     &self,
    //     _ctx: &rig::agent::HookContext,
    //     _event: rig::agent::ReasoningDelta<'_>,
    // ) -> rig::agent::ObservationAction {
    //     rig::agent::ObservationAction::Continue
    // }

    // async fn on_tool_call_delta(
    //     &self,
    //     _ctx: &rig::agent::HookContext,
    //     _event: rig::agent::ToolCallDelta<'_>,
    // ) -> rig::agent::ObservationAction {
    //     rig::agent::ObservationAction::Continue
    // }

    // async fn on_dispatch(
    //     &self,
    //     _ctx: &rig::agent::HookContext,
    //     _event: rig::agent::DispatchEvent<'_>,
    // ) -> rig::agent::DispatchAction {
    //     rig::agent::DispatchAction::Proceed
    // }

    // async fn on_outcome(
    //     &self,
    //     _ctx: &rig::agent::HookContext,
    //     _event: rig::agent::OutcomeEvent<'_>,
    // ) -> rig::agent::OutcomeAction {
    //     rig::agent::OutcomeAction::Proceed
    // }

    // fn observes(&self, kind: rig::agent::StepEventKind) -> bool {
    //     !matches!(
    //         kind,
    //         StepEventKind::EmbedDispatch
    //             | StepEventKind::RerankDispatch
    //             | StepEventKind::MemoryDispatch
    //             | StepEventKind::RetrieveDispatch
    //             | StepEventKind::CustomDispatch
    //     )
    // }
}
