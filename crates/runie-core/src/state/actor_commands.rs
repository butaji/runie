use super::*;
pub(super) fn apply(state: &mut AgentStateSnapshot, cmd: StateCommand) {
    let mut pending = Some(cmd);
    if apply_scalar_command(state, &mut pending) {
        return;
    }
    apply_remaining_command(state, pending.expect("unhandled state command"));
}

pub(super) fn apply_scalar_command(
    state: &mut AgentStateSnapshot,
    pending: &mut Option<StateCommand>,
) -> bool {
    if apply_configuration_command(state, pending) {
        return true;
    }
    apply_streaming_command(state, pending)
}

pub(super) fn apply_configuration_command(
    state: &mut AgentStateSnapshot,
    pending: &mut Option<StateCommand>,
) -> bool {
    let command = pending.take().expect("state command is present");
    match command {
        StateCommand::SetSystemPrompt(s, ack) => {
            state.system_prompt = s;
            let _ = ack.send(());
            true
        }
        StateCommand::SetModel(m, ack) => {
            state.model = m;
            let _ = ack.send(());
            true
        }
        StateCommand::SetThinkingLevel(t, ack) => {
            state.thinking_level = t;
            let _ = ack.send(());
            true
        }
        StateCommand::ReplaceMessages(msgs, ack) => {
            state.messages = msgs;
            let _ = ack.send(());
            true
        }
        StateCommand::SetTools(tools, ack) => {
            state.tools = tools;
            let _ = ack.send(());
            true
        }
        other => {
            *pending = Some(other);
            false
        }
    }
}

pub(super) fn apply_streaming_command(
    state: &mut AgentStateSnapshot,
    pending: &mut Option<StateCommand>,
) -> bool {
    let command = pending.take().expect("state command is present");
    match command {
        StateCommand::MarkStreaming(on, ack) => {
            state.is_streaming = on;
            let _ = ack.send(());
            true
        }
        StateCommand::SetStreamingMessage(m, ack) => {
            state.streaming_message = m;
            let _ = ack.send(());
            true
        }
        StateCommand::SetStreamingState {
            streaming,
            message,
            ack,
        } => {
            state.is_streaming = streaming;
            state.streaming_message = message;
            let _ = ack.send(());
            true
        }
        other => {
            *pending = Some(other);
            false
        }
    }
}

pub(super) fn apply_remaining_command(state: &mut AgentStateSnapshot, cmd: StateCommand) {
    match cmd {
        StateCommand::PushMessage(m, ack) => apply_push_message(state, m, ack),
        StateCommand::AddPendingToolCall(id, ack) => {
            if !state.pending_tool_calls.contains(&id) {
                state.pending_tool_calls.push(id);
            }
            if let Some(ack) = ack {
                let _ = ack.send(());
            }
        }
        StateCommand::RemovePendingToolCall(id, ack) => {
            state.pending_tool_calls.retain(|x| x != &id);
            if let Some(ack) = ack {
                let _ = ack.send(());
            }
        }
        StateCommand::SetError(e, ack) => {
            state.error_message = e;
            if let Some(ack) = ack {
                let _ = ack.send(());
            }
        }
        StateCommand::ApplyEvent(event, ack) => {
            AgentStateActor::apply_event_to_state(state, *event);
            let _ = ack.send(());
        }
        StateCommand::Reset(ack) => {
            *state = AgentStateSnapshot::default();
            let _ = ack.send(());
        }
        _ => {}
    }
}

pub(super) fn apply_push_message(
    state: &mut AgentStateSnapshot,
    message: AgentMessage,
    ack: Option<oneshot::Sender<()>>,
) {
    state.messages.push(message);
    if let Some(ack) = ack {
        let _ = ack.send(());
    }
}
