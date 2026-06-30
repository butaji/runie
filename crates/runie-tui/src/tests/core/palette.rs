use runie_core::commands::{DialogKind, DialogState};
use runie_core::model::AppState;
use runie_core::Event;

use crate::tests::view;

fn palette_state(state: &AppState) -> Option<(String, usize)> {
    match &state.open_dialog {
        Some(DialogState::Active {
            kind: DialogKind::CommandPalette,
            panels: stack,
        }) => stack.current().map(|p| (p.filter.clone(), p.selected)),
        _ => None,
    }
}

fn model_selector_state(state: &AppState) -> Option<(String, usize)> {
    match &state.open_dialog {
        Some(DialogState::Active {
            kind: DialogKind::ModelSelector,
            panels: stack,
        }) => stack.current().map(|p| (p.filter.clone(), p.selected)),
        _ => None,
    }
}

#[test]
fn slash_opens_command_palette_when_input_empty() {
    let mut state = AppState::default();
    assert!(state.open_dialog.is_none());
    assert!(state.input.input.is_empty());
    state.update(Event::Input('/'));
    assert!(
        palette_state(&state).is_some(),
        "Typing / with empty input should open command palette"
    );
}

#[test]
fn slash_does_not_open_palette_when_input_not_empty() {
    let mut state = AppState::default();
    state.update(Event::Input('h'));
    state.update(Event::Input('i'));
    assert_eq!(state.input.input, "hi");
    state.update(Event::Input('/'));
    assert!(
        state.open_dialog.is_none(),
        "Typing / with non-empty input should NOT open palette"
    );
    assert_eq!(
        state.input.input, "hi/",
        "Slash should be inserted normally"
    );
}

#[test]
fn ctrl_p_opens_command_palette() {
    let mut state = AppState::default();
    assert!(state.open_dialog.is_none());
    state.update(Event::toggle_command_palette());
    assert!(
        palette_state(&state).is_some(),
        "Ctrl+P should open command palette"
    );
}

#[test]
fn toggle_opens_palette() {
    let mut state = AppState::default();
    assert!(state.open_dialog.is_none());
    state.update(Event::toggle_command_palette());
    assert!(palette_state(&state).is_some());
}

#[test]
fn select_pushes_palette_to_back_stack() {
    let mut state = AppState::default();
    state.update(Event::toggle_command_palette());
    // Filter to a command that opens a sub-dialog (thinking).
    for c in "thinking".chars() {
        state.update(Event::palette_filter(c));
    }
    // Select the "thinking" command.
    state.update(Event::palette_select());
    assert!(
        state.open_dialog.is_some(),
        "Activating a palette item should open the command's sub-dialog"
    );
    assert_eq!(
        state.dialog_back_stack.len(),
        1,
        "Palette should be pushed onto the back stack"
    );
}

#[test]
fn esc_from_subdialog_returns_to_palette() {
    let mut state = AppState::default();
    state.update(Event::toggle_command_palette());
    // Filter to and open a sub-dialog command.
    for c in "thinking".chars() {
        state.update(Event::palette_filter(c));
    }
    state.update(Event::palette_select());

    assert!(
        state.open_dialog.is_some(),
        "Sub-dialog should be open after select"
    );
    assert_eq!(state.dialog_back_stack.len(), 1);

    // Esc on the sub-dialog must pop back to the palette, not close.
    state.update(Event::dialog_back());
    assert!(
        matches!(
            state.open_dialog,
            Some(DialogState::Active {
                kind: DialogKind::CommandPalette,
                panels: _
            })
        ),
        "Esc on sub-dialog must return to the palette, got {:?}",
        state.open_dialog
    );
    assert!(state.dialog_back_stack.is_empty());

    // Esc on the palette (root) must close the bar.
    state.update(Event::dialog_back());
    assert!(state.open_dialog.is_none(), "Esc on palette must close");
}

#[test]
fn message_command_from_palette_closes_palette() {
    let mut state = AppState::default();
    state.update(Event::toggle_command_palette());
    // Filter to a message-only command (history).
    for c in "history".chars() {
        state.update(Event::palette_filter(c));
    }
    state.update(Event::palette_select());

    assert!(
        state.open_dialog.is_none(),
        "Message command should close the palette"
    );
    assert!(state.dialog_back_stack.is_empty());
}

#[test]
fn submit_on_message_command_closes_palette() {
    let mut state = AppState::default();
    state.update(Event::toggle_command_palette());
    // Filter to a message-only command (new).
    for c in "new".chars() {
        state.update(Event::palette_filter(c));
    }
    // A real Enter key produces Submit, not PaletteSelect.
    state.update(Event::submit());

    assert!(
        state.open_dialog.is_none(),
        "Submit on a message command should close the palette, got {:?}",
        state.open_dialog
    );
    assert!(state.dialog_back_stack.is_empty());
}

#[test]
fn close_pops_dialog() {
    let mut state = AppState::default();
    state.update(Event::toggle_command_palette());
    assert!(state.open_dialog.is_some());
    state.update(Event::PaletteClose);
    assert!(state.open_dialog.is_none());
}

#[test]
fn esc_closes_palette() {
    let mut state = AppState::default();
    state.update(Event::toggle_command_palette());
    assert!(state.open_dialog.is_some());
    state.update(Event::Abort);
    assert!(state.open_dialog.is_none());
}

#[test]
fn filter_reduces_selection() {
    let mut state = AppState::default();
    state.update(Event::toggle_command_palette());
    // Type "q" to filter to quit
    state.update(Event::PaletteFilter('q'));
    let (filter, selected) = palette_state(&state).expect("Palette should be open");
    assert_eq!(filter, "q");
    assert_eq!(selected, 0, "Filter resets selection to 0");
}

#[test]
fn typing_in_model_selector_filters_models() {
    // Regression: typing a character in the model selector must update
    // the model selector's filter, not pop back to the command palette.
    super::super::configure_test_providers(&[("openai".into(), vec!["gpt-4o".into()])]);
    let mut state = AppState::default();
    super::super::apply_test_config_to_state(&mut state);
    state.update(Event::toggle_command_palette());
    // Filter to and open the model selector.
    for c in "model".chars() {
        state.update(Event::palette_filter(c));
    }
    state.update(Event::palette_select());

    assert!(
        model_selector_state(&state).is_some(),
        "Model selector should be open after selecting /model"
    );

    // Type in the model selector.
    state.update(Event::ModelSelectorFilter('m'));

    assert!(
        model_selector_state(&state).is_some(),
        "Typing in model selector must keep it open, got {:?}",
        state.open_dialog
    );
    let (filter, _) = model_selector_state(&state).expect("model selector still open");
    assert_eq!(filter, "m", "Model selector filter should contain 'm'");
}

#[test]
fn esc_restores_palette_in_same_state() {
    // When Esc pops back from a sub-dialog to the palette, the palette
    // must be restored with the same filter and selection as before.
    let mut state = AppState::default();
    state.update(Event::toggle_command_palette());
    for c in "thinking".chars() {
        state.update(Event::palette_filter(c));
    }
    // Move selection down once so it is non-zero.
    state.update(Event::PaletteDown);
    let (filter_before, selected_before) = palette_state(&state).expect("palette should be open");
    assert_eq!(filter_before, "thinking");

    state.update(Event::palette_select());
    assert!(
        !matches!(
            state.open_dialog,
            Some(DialogState::Active {
                kind: DialogKind::CommandPalette,
                panels: _
            })
        ),
        "Sub-dialog should be open"
    );

    state.update(Event::dialog_back());
    let (filter_after, selected_after) = palette_state(&state).expect("palette should be restored");
    assert_eq!(
        filter_after, filter_before,
        "Esc must preserve palette filter"
    );
    assert_eq!(
        selected_after, selected_before,
        "Esc must preserve palette selection"
    );
}

#[test]
fn palette_model_with_zero_providers_shows_message() {
    // Ensure no providers are configured.
    super::super::configure_test_providers(&[]);
    let mut state = AppState::default();
    state.update(Event::toggle_command_palette());
    for c in "model".chars() {
        state.update(Event::palette_filter(c));
    }
    state.update(Event::palette_select());

    assert!(
        state.open_dialog.is_none(),
        "model command with no providers should close palette, got {:?}",
        state.open_dialog
    );
    let msgs: Vec<String> = state.session.messages.iter().map(|m| m.content()).collect();
    assert!(
        msgs.iter().any(|m| m.contains("No connected providers")),
        "expected message about no connected providers, got messages: {:?}",
        msgs
    );
}

#[test]
fn palette_model_with_zero_providers_renders_message() {
    use ratatui::{backend::TestBackend, Terminal};

    super::super::configure_test_providers(&[]);
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    state.update(Event::toggle_command_palette());
    for c in "model".chars() {
        state.update(Event::palette_filter(c));
    }
    state.update(Event::palette_select());

    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let content: String = buf.content().iter().map(|c| c.symbol()).collect();
    assert!(
        content.contains("No connected providers"),
        "render should show no-providers message: {}",
        content
    );
}

#[test]
fn palette_model_with_args_switches_model() {
    super::super::configure_test_providers(&[(
        "openai".into(),
        vec!["gpt-4o".into(), "gpt-4o-mini".into()],
    )]);
    let mut state = AppState::default();
    super::super::apply_test_config_to_state(&mut state);
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o".into();

    state.update(Event::toggle_command_palette());
    for c in "model gpt-4o-mini".chars() {
        state.update(Event::palette_filter(c));
    }
    state.update(Event::submit());

    assert!(
        state.open_dialog.is_none(),
        "palette should close after /model with args"
    );
    assert_eq!(state.config.current_model, "gpt-4o-mini");
    let msgs: Vec<String> = state.session.messages.iter().map(|m| m.content()).collect();
    assert!(
        msgs.iter()
            .any(|m| m.contains("Switched to openai/gpt-4o-mini")),
        "expected switch message, got messages: {:?}",
        msgs
    );
}

// ── UiActor-level Layer 2: palette closes after slash command ─────────────────

/// Layer 2: UiActor::dispatch_submit_content closes the command palette
/// before executing a slash command, so the result is visible without overlay.
#[test]
fn ui_actor_dispatch_submit_closes_palette() {
    use crate::ui_actor::UiActor;
    use crate::ui_actor_agent_handles::AgentHandleBox;
    use std::sync::Arc;

    struct NoopAgent;
    impl runie_core::actors::leader::LeaderAgentHandle for NoopAgent {
        fn run(
            &self,
            _cmd: runie_core::actors::leader::LeaderAgentCmd,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
            Box::pin(async {})
        }
    }

    let agent_handle =
        crate::ui_actor_agent_handles::LeaderAgentActorHandle::new(Arc::new(NoopAgent));

    let state = runie_core::AppState::default();
    let (kb_tx, _kb_rx) = tokio::sync::watch::channel(Default::default());
    let bus = runie_core::bus::EventBus::<runie_core::Event>::new(16);
    let (shutdown_tx, _) = tokio::sync::oneshot::channel();
    let caps = crate::terminal::caps::TermCaps::default();

    let mut ui = UiActor::with_agent_handle(
        state,
        AgentHandleBox::Leader(agent_handle),
        kb_tx,
        bus,
        shutdown_tx,
        caps,
    );

    // Open the command palette (e.g., user typed '/')
    ui.state
        .update(runie_core::Event::ToggleCommandPalette);
    assert!(
        ui.state.open_dialog.is_some(),
        "palette should be open before dispatch"
    );

    // Dispatch a slash command — this should close the palette.
    ui.dispatch_submit_content("/session".to_owned());

    assert!(
        ui.state.open_dialog.is_none(),
        "command palette must close after slash command, got {:?}",
        ui.state.open_dialog
    );
}
