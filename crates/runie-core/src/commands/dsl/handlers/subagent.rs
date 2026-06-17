//! Subagent commands.

use crate::commands::{CommandCategory, CommandRegistry, CommandResult};
use crate::event::ControlEvent;
use crate::model::AppState;

use super::spec::{CommandKind, CommandSpec};

static SUBAGENT_COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        name: "spawn",
        desc: "Run a subagent turn (delegated task)",
        aliases: &[],
        category: CommandCategory::System,
        sub: false,
        kind: CommandKind::Handler(handle_spawn),
    },
    CommandSpec {
        name: "steer",
        desc: "Send a message to a running subagent",
        aliases: &[],
        category: CommandCategory::System,
        sub: false,
        kind: CommandKind::Handler(handle_steer),
    },
    CommandSpec {
        name: "cancel",
        desc: "Cancel a running subagent",
        aliases: &[],
        category: CommandCategory::System,
        sub: false,
        kind: CommandKind::Handler(handle_cancel),
    },
];

pub fn register(registry: &mut CommandRegistry) {
    super::spec::register_commands(registry, SUBAGENT_COMMANDS);
}

/// `/spawn <prompt>` — if a prompt is provided as an argument, emit
/// a `SpawnAgent` event directly. Otherwise, open a form to collect
/// the prompt from the user.
pub fn handle_spawn(_state: &mut AppState, args: &str) -> CommandResult {
    let prompt = args.trim();
    if prompt.is_empty() {
        return CommandResult::OpenPanelStack(Box::new(crate::commands::build_spawn_form_panel()));
    }
    CommandResult::Event(ControlEvent::SpawnAgent {
        prompt: prompt.to_string(),
    })
}

/// `/steer <agent> <message>` — emit a `SteerAgent` event.
pub fn handle_steer(_state: &mut AppState, args: &str) -> CommandResult {
    let trimmed = args.trim();
    let (agent_id, message) = split_first_word(trimmed);
    if agent_id.is_empty() {
        return CommandResult::OpenPanelStack(Box::new(crate::commands::build_steer_form_panel()));
    }
    CommandResult::Event(ControlEvent::SteerAgent {
        agent_id: agent_id.to_string(),
        message: message.to_string(),
    })
}

/// `/cancel <agent>` — emit a `CancelAgent` event.
pub fn handle_cancel(_state: &mut AppState, args: &str) -> CommandResult {
    let agent_id = args.trim();
    if agent_id.is_empty() {
        return CommandResult::OpenPanelStack(Box::new(crate::commands::build_cancel_form_panel()));
    }
    CommandResult::Event(ControlEvent::CancelAgent {
        agent_id: agent_id.to_string(),
    })
}

fn split_first_word(input: &str) -> (&str, &str) {
    let input = input.trim_start();
    match input.split_once(char::is_whitespace) {
        Some((first, rest)) => (first, rest.trim_start()),
        None => (input, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::CommandResult;
    use crate::event::ControlEvent;
    use crate::model::AppState;

    #[test]
    fn steer_with_args_emits_event() {
        let mut state = AppState::default();
        let result = handle_steer(&mut state, "researcher-A1B focus on src/lib.rs");
        match result {
            CommandResult::Event(ControlEvent::SteerAgent { agent_id, message }) => {
                assert_eq!(agent_id, "researcher-A1B");
                assert_eq!(message, "focus on src/lib.rs");
            }
            other => panic!("expected SteerAgent event, got {:?}", other),
        }
    }

    #[test]
    fn steer_without_agent_opens_form() {
        let mut state = AppState::default();
        let result = handle_steer(&mut state, "");
        assert!(
            matches!(result, CommandResult::OpenPanelStack(_)),
            "expected form panel, got {:?}",
            result
        );
    }

    #[test]
    fn cancel_with_args_emits_event() {
        let mut state = AppState::default();
        let result = handle_cancel(&mut state, "researcher-A1B");
        match result {
            CommandResult::Event(ControlEvent::CancelAgent { agent_id }) => {
                assert_eq!(agent_id, "researcher-A1B");
            }
            other => panic!("expected CancelAgent event, got {:?}", other),
        }
    }

    #[test]
    fn cancel_without_agent_opens_form() {
        let mut state = AppState::default();
        let result = handle_cancel(&mut state, "");
        assert!(
            matches!(result, CommandResult::OpenPanelStack(_)),
            "expected form panel, got {:?}",
            result
        );
    }
}
