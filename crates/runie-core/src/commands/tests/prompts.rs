use crate::event::CommandEvent;
use crate::model::AppState;
use crate::prompts::{PromptSource, PromptTemplate};

#[test]
fn prompt_switch_updates() {
    let mut state = AppState {
        prompts: vec![PromptTemplate {
            name: "custom".into(),
            content: "Be concise.".into(),
            source: PromptSource::BuiltIn,
        }],
        ..Default::default()
    };
    state.update(CommandEvent::RunPromptCommand {
        name: "custom".into(),
    });
    assert_eq!(state.input.current_prompt, "custom");
    let last = state.session.messages.last().expect("should have message");
    assert!(last.content().contains("custom"));
}

#[test]
fn prompt_shows_current_when_no_args() {
    let mut state = AppState {
        prompts: vec![PromptTemplate {
            name: "default".into(),
            content: "Be helpful.".into(),
            source: PromptSource::BuiltIn,
        }],
        ..Default::default()
    };
    state.update(CommandEvent::RunPromptCommand { name: "".into() });
    let last = state.session.messages.last().expect("should have message");
    assert!(last.content().contains("default"), "got: {}", last.content());
}

#[test]
fn prompt_unknown_returns_error() {
    let mut state = AppState::default();
    state.update(CommandEvent::RunPromptCommand {
        name: "unknown".into(),
    });
    let last = state.session.messages.last().expect("should have message");
    assert!(last.content().contains("not found"), "got: {}", last.content());
}
