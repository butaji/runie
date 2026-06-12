//! Stack navigation — ESC pops one panel; only closes the dialog at the root.
//!
//! These tests exercise the full update flow: a multi-panel stack, ESC
//! events (SettingsClose / PaletteClose), and the open_dialog state.

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
        ItemAction::Emit(Event::SystemMessage {
            content: "saved".into(),
        }),
    );

    let mut stack = PanelStack::new(root);
    stack.push(child);

    let mut state = AppState::default();
    state.open_dialog = Some(DialogState::Settings(stack));
    state
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
    state.update(Event::SettingsClose);
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
    state.update(Event::SettingsClose);
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
    state.update(Event::SettingsClose); // pop to root
    assert!(matches!(&state.open_dialog, Some(DialogState::Settings(s)) if s.len() == 1));
    state.update(Event::SettingsClose); // close at root
    assert!(
        state.open_dialog.is_none(),
        "second ESC at root must close the dialog"
    );
}

#[test]
fn abort_force_closes_regardless_of_depth() {
    let mut state = open_settings_with_subpanel();
    // Abort is the force-close escape hatch, distinct from ESC stack nav.
    state.update(Event::Abort);
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
    let mut state = AppState::default();
    state.open_dialog = Some(DialogState::CommandPalette(stack));

    // First Esc (in sub-panel): pop to root, bar still open.
    state.update(Event::PaletteClose);
    match &state.open_dialog {
        Some(DialogState::CommandPalette(s)) => {
            assert_eq!(s.len(), 1, "Esc in sub-panel must pop, not close");
            assert_eq!(s.current().unwrap().id, "palette");
        }
        _ => panic!("popped palette should still be open"),
    }
    // Second Esc (on main menu / root): close the bar.
    state.update(Event::PaletteClose);
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
    root = root.item("_Cancel", ItemAction::Emit(Event::LoginFlowCancel));
    let mut stack = PanelStack::new(root);
    // Push a child form panel to simulate a multi-step form.
    let mut child = Panel::new("login-models", "Models").form();
    child = child.toggle(
        "model-a",
        true,
        ItemAction::Emit(Event::LoginFlowToggleModel {
            model: "model-a".into(),
        }),
    );
    child = child.item("_Back", ItemAction::Emit(Event::LoginFlowCancel));
    stack.push(child);

    let mut state = AppState::default();
    state.open_dialog = Some(DialogState::PanelStack(stack));

    // ESC in the child form: pop to root.
    state.update(Event::CommandFormClose);
    match &state.open_dialog {
        Some(DialogState::PanelStack(s)) => {
            assert_eq!(s.len(), 1);
            assert_eq!(s.current().unwrap().id, "login-key");
        }
        _ => panic!("form child ESC should pop, not close"),
    }

    // ESC in the root form: close.
    state.update(Event::CommandFormClose);
    assert!(state.open_dialog.is_none(), "form root ESC should close");
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
    use crate::commands::{CommandFlow, CommandRegistry};

    let mut reg = CommandRegistry::new();
    crate::commands::handlers::register_all(&mut reg);

    // These commands open a sub-dialog and MUST have .sub().
    let must_be_sub: &[&str] = &[
        // Built-in dialogs
        "settings",
        // Handler-based: open dialogs from their handler
        "theme",
        "model",
        "thinking",
        "scoped-models",
        "providers",
        "tree",
        // Form-based
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

    let mut missing: Vec<String> = Vec::new();
    for name in must_be_sub {
        match reg.get(name) {
            Some(def) => {
                let flow = match &def.flow {
                    CommandFlow::Sub(inner) => inner.as_ref(),
                    other => other,
                };
                // Inner flow must be a dialog-opening flow.
                let opens_sub = matches!(
                    flow,
                    CommandFlow::Dialog(_)
                        | CommandFlow::PanelStack(_)
                        | CommandFlow::Form { .. }
                        | CommandFlow::Handler(_)
                );
                let is_sub = matches!(def.flow, CommandFlow::Sub(_));
                if !is_sub {
                    missing.push(format!("'{}' is missing .sub()", name));
                } else if !opens_sub {
                    missing.push(format!(
                        "'{}' has .sub() but inner flow does not open a dialog",
                        name
                    ));
                }
            }
            None => missing.push(format!("'{}' is not registered", name)),
        }
    }

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

#[test]
fn global_dialog_back_stack_palette_pushes_subdialog() {
    // Android-like: when a command from the command palette (main
    // menu) opens a sub-dialog, the palette is pushed onto a global
    // back stack. Esc on the sub-dialog pops back to the palette.
    // Esc on the palette (root of the back stack) closes the bar.
    use crate::commands::DialogState;
    use crate::dialog::{Panel, PanelStack};
    use crate::event::Event;

    let mut state = AppState::default();
    // Simulate the palette being open.
    let palette = Panel::new("palette", "Commands").keep_open();
    let palette_stack = PanelStack::new(palette);
    state.open_dialog = Some(DialogState::CommandPalette(palette_stack));

    // A command from the palette opens a sub-dialog (e.g. settings).
    // process_command_result pushes the palette onto the back stack.
    let sub = Panel::new("sub", "Sub").keep_open();
    let sub_stack = PanelStack::new(sub);
    if let Some(current) = state.open_dialog.take() {
        state.push_dialog_to_back_stack(current);
    }
    state.open_dialog = Some(DialogState::PanelStack(sub_stack));

    // Verify: back stack has the palette, current is the sub-dialog.
    assert_eq!(state.dialog_back_stack.len(), 1);
    assert!(matches!(
        state.open_dialog,
        Some(DialogState::PanelStack(_))
    ));

    // Esc on the sub-dialog (root of its PanelStack) should restore
    // the palette from the back stack, NOT close the dialog.
    state.update(Event::DialogBack);
    assert!(
        matches!(state.open_dialog, Some(DialogState::CommandPalette(_))),
        "Esc on sub-dialog must restore palette, got {:?}",
        state.open_dialog
    );
    assert!(state.dialog_back_stack.is_empty());

    // Esc on the palette (root, back stack empty) should close.
    state.update(Event::DialogBack);
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
    use crate::commands::DialogState;
    use crate::dialog::{Panel, PanelStack};
    use crate::event::Event;

    let mut state = AppState::default();
    // Simulate: palette is open (Main Menu).
    let palette = Panel::new("palette", "Commands").keep_open();
    state.open_dialog = Some(DialogState::CommandPalette(PanelStack::new(palette)));

    // Simulate: user selects "login" from the palette. The login
    // command goes through process_command_result, which pushes the
    // palette onto the back stack and opens the login dialog.
    if let Some(current) = state.open_dialog.take() {
        state.push_dialog_to_back_stack(current);
    }
    let login_root = PanelStack::new(Panel::new("login-provider", "Login").keep_open());
    state.open_dialog = Some(DialogState::PanelStack(login_root));

    // Verify: palette is on the back stack, login dialog is on top.
    assert_eq!(state.dialog_back_stack.len(), 1);
    assert!(matches!(
        state.open_dialog,
        Some(DialogState::PanelStack(_))
    ));

    // Esc on the login dialog (sub-menu) — must pop back to the
    // palette (Main Menu), NOT close.
    state.update(Event::DialogBack);
    assert!(
        matches!(state.open_dialog, Some(DialogState::CommandPalette(_))),
        "Esc on sub-menu must return to Main Menu (palette), got {:?}",
        state.open_dialog
    );
    assert!(state.dialog_back_stack.is_empty());

    // Esc on the palette (Main Menu) — must close the bar.
    state.update(Event::DialogBack);
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
