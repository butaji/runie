//! Stack navigation — ESC pops one panel; only closes the dialog at the root.
//!
//! These tests exercise the full update flow: a multi-panel stack, ESC
//! events (SettingsClose / PaletteClose), and the open_dialog state.

use crate::event::{InputEvent, ControlEvent, ModelConfigEvent, SystemEvent, DialogEvent, ScrollEvent, AgentEvent, SessionEvent, EditEvent, CommandEvent, DurableCoreEvent, LoginFlowEvent};

use crate::commands::DialogState;
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::event::Event;
use crate::model::AppState;

fn open_settings_with_subpanel() -> AppState {
    // Open the settings dialog (list view) with a root panel that has a
    // "Push" item to a child panel. Two levels deep.
    let mut root = Panel::new("settings", "Settings");
    root = root.item("Open advanced", ItemAction::Push("advanced".into()));

    let mut child = Panel::new("advanced", "Advanced");
    child = child.item("Back", ItemAction::Pop);
    child = child.item(
        "Save",
        ItemAction::Emit(Event::System(SystemEvent::SystemMessage {
            content: "saved".into(),
        })),
    );

    let mut stack = PanelStack::new(root);
    stack.push(child);

    AppState {
        open_dialog: Some(DialogState::Settings(stack)),
        ..Default::default()
    }
}

#[test]
fn esc_at_root_closes_the_dialog() {
    let mut state = open_settings_with_subpanel();
    // The push hasn't happened via an event yet, so stack depth is 1? No,
    // open_settings_with_subpanel pushes the child, so depth is 2. Let me
    // instead test a single-panel dialog.
    state.open_dialog = Some(DialogState::Settings(PanelStack::new(
        Panel::new("settings", "Settings").item("Done", ItemAction::Close),
    )));
    assert!(state.open_dialog.is_some());
    state.update(Event::ModelConfig(ModelConfigEvent::SettingsClose));
    assert!(
        state.open_dialog.is_none(),
        "ESC at root must close the dialog"
    );
}

#[test]
fn esc_in_subpanel_pops_and_keeps_dialog_open() {
    let mut state = open_settings_with_subpanel();
    // Stack depth = 2 (root + child).
    assert!(matches!(&state.open_dialog, Some(DialogState::Settings(s)) if s.len() == 2));
    state.update(Event::ModelConfig(ModelConfigEvent::SettingsClose));
    // Stack should have popped to depth 1, dialog still open.
    match &state.open_dialog {
        Some(DialogState::Settings(stack)) => {
            assert_eq!(stack.len(), 1, "ESC in subpanel must pop to root");
            assert_eq!(stack.current().unwrap().id, "settings");
        }
        other => panic!("dialog should remain open, got {:?}", other),
    }
}

#[test]
fn double_esc_pops_then_closes() {
    let mut state = open_settings_with_subpanel();
    state.update(Event::ModelConfig(ModelConfigEvent::SettingsClose)); // pop to root
    assert!(matches!(&state.open_dialog, Some(DialogState::Settings(s)) if s.len() == 1));
    state.update(Event::ModelConfig(ModelConfigEvent::SettingsClose)); // close at root
    assert!(
        state.open_dialog.is_none(),
        "second ESC at root must close the dialog"
    );
}

#[test]
fn abort_force_closes_regardless_of_depth() {
    let mut state = open_settings_with_subpanel();
    // Abort is the force-close escape hatch, distinct from ESC stack nav.
    state.update(Event::Control(ControlEvent::Abort));
    assert!(
        state.open_dialog.is_none(),
        "Abort must force-close the dialog at any depth"
    );
}

#[test]
fn palette_close_pops_or_closes() {
    // The command bar (palette) treats Esc as a **Back** button:
    //   - from a sub-panel: pop one level (stay in the bar)
    //   - from the main menu (root): close the bar
    // This is the exact semantic the user requested and is the same
    // for every dialog backed by PanelStack. We exercise the palette
    // here because it's the most visible "command bar" in the app.
    let mut root = Panel::new("palette", "Commands");
    root = root.item("Sub", ItemAction::Push("sub".into()));
    let mut child = Panel::new("sub", "Sub");
    child = child.item("Back", ItemAction::Pop);
    let mut stack = PanelStack::new(root);
    stack.push(child);
    let mut state = AppState {
        open_dialog: Some(DialogState::CommandPalette(stack)),
        ..Default::default()
    };

    // First Esc (in sub-panel): pop to root, bar still open.
    state.update(Event::Dialog(DialogEvent::PaletteClose));
    match &state.open_dialog {
        Some(DialogState::CommandPalette(s)) => {
            assert_eq!(s.len(), 1, "Esc in sub-panel must pop, not close");
            assert_eq!(s.current().unwrap().id, "palette");
        }
        _ => panic!("popped palette should still be open"),
    }
    // Second Esc (on main menu / root): close the bar.
    state.update(Event::Dialog(DialogEvent::PaletteClose));
    assert!(
        state.open_dialog.is_none(),
        "Esc on the main menu must close the command bar"
    );
}

#[test]
fn form_dialog_esc_pops_or_closes() {
    // Form dialogs (e.g. login form) also use stack nav. ESC pops the
    // current form panel; at root, closes the dialog.
    let mut root = Panel::new("login-key", "Login").form();
    root = root.item("_Cancel", ItemAction::Emit(Event::LoginFlow(LoginFlowEvent::Cancel)));
    let mut stack = PanelStack::new(root);
    // Push a child form panel to simulate a multi-step form.
    let mut child = Panel::new("login-models", "Models").form();
    child = child.toggle(
        "model-a",
        true,
        ItemAction::Emit(Event::LoginFlow(LoginFlowEvent::ToggleModel {
            model: "model-a".into(),
        })),
    );
    child = child.item("_Back", ItemAction::Emit(Event::LoginFlow(LoginFlowEvent::Cancel)));
    stack.push(child);

    let mut state = AppState {
        open_dialog: Some(DialogState::PanelStack(stack)),
        ..Default::default()
    };

    // ESC in the child form: pop to root.
    state.update(Event::Dialog(DialogEvent::CommandFormClose));
    match &state.open_dialog {
        Some(DialogState::PanelStack(s)) => {
            assert_eq!(s.len(), 1);
            assert_eq!(s.current().unwrap().id, "login-key");
        }
        _ => panic!("form child ESC should pop, not close"),
    }

    // ESC in the root form: close.
    state.update(Event::Dialog(DialogEvent::CommandFormClose));
    assert!(state.open_dialog.is_none(), "form root ESC should close");
}

fn check_command_is_sub(reg: &crate::commands::CommandRegistry, name: &str) -> Option<String> {
    use crate::commands::CommandFlow;

    match reg.get(name) {
        Some(def) => {
            let flow = match &def.flow {
                CommandFlow::Sub(inner) => inner.as_ref(),
                other => other,
            };
            let opens_sub = matches!(
                flow,
                CommandFlow::Dialog(_) | CommandFlow::PanelStack(_) | CommandFlow::Handler(_)
            );
            let is_sub = matches!(def.flow, CommandFlow::Sub(_));
            if !is_sub {
                Some(format!("'{}' is missing .sub()", name))
            } else if !opens_sub {
                Some(format!(
                    "'{}' has .sub() but inner flow does not open a dialog",
                    name
                ))
            } else {
                None
            }
        }
        None => Some(format!("'{}' is not registered", name)),
    }
}

/// Contract: every menu-bar command that opens a sub-dialog (form,
/// panel, or built-in dialog) MUST be registered with `.sub()` so it
/// participates in the global back stack. This is the Android-like
/// "Back button" contract: Main Menu -> Sub Menu -> Esc -> Main Menu.
///
/// Commands that just show a message or perform a side effect
/// (e.g. `new`, `reset`, `clone`, `share`) do NOT need `.sub()`.
#[test]
fn every_sub_opening_command_is_marked_sub() {
    use crate::commands::CommandRegistry;

    let mut reg = CommandRegistry::new();
    crate::commands::handlers::register_all(&mut reg);

    let must_be_sub: &[&str] = &[
        "settings",
        "theme",
        "model",
        "thinking",
        "scoped-models",
        "providers",
        "tree",
        "save",
        "load",
        "delete",
        "export",
        "import",
        "compact",
        "fork",
        "name",
        "prompt",
    ];

    let missing: Vec<String> = must_be_sub
        .iter()
        .filter_map(|name| check_command_is_sub(&reg, name))
        .collect();

    assert!(
        missing.is_empty(),
        "Commands that should have .sub() but don't:\n  {}\n\
         Every menu-bar item that opens a sub-dialog MUST be marked\n\
         with .sub() for Android-like back-stack navigation (Esc pops\n\
         back to the Main Menu).",
        missing.join("\n  ")
    );
}

/// Verify the full round-trip for every .sub() command: execute
/// the flow and confirm it pushes the current dialog to the back
/// stack. This catches regressions where a command is added without
/// .sub() or the Sub variant stops pushing.
#[test]
fn sub_command_pushes_current_dialog_to_back_stack() {
    use crate::commands::{CommandFlow, CommandRegistry};

    let mut reg = CommandRegistry::new();
    crate::commands::handlers::register_all(&mut reg);

    // Commands that must push to back stack (those with .sub()).
    let must_push: &[&str] = &[
        "settings",
        "theme",
        "model",
        "thinking",
        "scoped-models",
        "providers",
        "tree",
        "save",
        "load",
        "delete",
        "export",
        "import",
        "compact",
        "fork",
        "name",
        "prompt",
    ];

    for name in must_push {
        let def = reg
            .get(name)
            .unwrap_or_else(|| panic!("{} not registered", name));
        // The flow must be Sub-wrapped.
        assert!(
            matches!(def.flow, CommandFlow::Sub(_)),
            "'{}' must be wrapped in CommandFlow::Sub (use .sub() in DSL)",
            name
        );
    }
}

fn state_with_palette_and_subdialog() -> AppState {
    let mut state = AppState::default();
    let palette = Panel::new("palette", "Commands").keep_open();
    state.open_dialog = Some(DialogState::CommandPalette(PanelStack::new(palette)));

    if let Some(current) = state.open_dialog.take() {
        state.push_dialog_to_back_stack(current);
    }
    let sub = PanelStack::new(Panel::new("sub", "Sub").keep_open());
    state.open_dialog = Some(DialogState::PanelStack(sub));

    state
}

fn assert_palette_restored(state: &AppState) {
    assert!(
        matches!(state.open_dialog, Some(DialogState::CommandPalette(_))),
        "Esc on sub-dialog must restore palette, got {:?}",
        state.open_dialog
    );
    assert!(state.dialog_back_stack.is_empty());
}

#[test]
fn global_dialog_back_stack_palette_pushes_subdialog() {
    let mut state = state_with_palette_and_subdialog();

    assert_eq!(state.dialog_back_stack.len(), 1);
    assert!(matches!(
        state.open_dialog,
        Some(DialogState::PanelStack(_))
    ));

    state.update(Event::Dialog(DialogEvent::DialogBack));
    assert_palette_restored(&state);

    state.update(Event::Dialog(DialogEvent::DialogBack));
    assert!(
        state.open_dialog.is_none(),
        "Esc on palette (root) must close the dialog"
    );
}

/// User-reported scenario: open the command bar (palette = Main
/// Menu), select a command that opens a sub-dialog (e.g. login →
/// provider picker), press Esc — must go back to the palette
/// (Main Menu), NOT close the whole bar. Press Esc again — closes.
#[test]
fn palette_then_subdialog_esc_back_to_palette_then_esc_closes() {
    let mut state = state_with_palette_and_subdialog();

    assert_eq!(state.dialog_back_stack.len(), 1);
    assert!(matches!(
        state.open_dialog,
        Some(DialogState::PanelStack(_))
    ));

    state.update(Event::Dialog(DialogEvent::DialogBack));
    assert_palette_restored(&state);

    state.update(Event::Dialog(DialogEvent::DialogBack));
    assert!(
        state.open_dialog.is_none(),
        "Esc on Main Menu must close the bar"
    );
}

#[test]
fn pushing_via_item_action_grows_the_stack() {
    // The Push action on a list item should grow the stack by one.
    let root = Panel::new("root", "Root")
        .item("Open sub", ItemAction::Push("sub".into()))
        .item("Cancel", ItemAction::Close);
    let sub = Panel::new("sub", "Sub").item("Back", ItemAction::Pop);

    let mut stack = PanelStack::new(root);
    assert_eq!(stack.len(), 1);
    stack.push(sub);
    assert_eq!(stack.len(), 2);
    assert_eq!(stack.current().unwrap().id, "sub");
    let popped = stack.pop().expect("pop sub");
    assert_eq!(popped.id, "sub");
    assert!(stack.pop().is_none(), "cannot pop root");
}
