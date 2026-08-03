//! System commands.

use crate::commands::dsl::handlers::NamedHandler;
use crate::commands::{CommandResult, DialogType};
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::model::AppState;

pub fn register_handlers(registry: &mut crate::commands::dsl::handlers::registry::HandlerRegistry) {
    registry.register("settings", NamedHandler::Handler(handle_settings));
    registry.register("copy", NamedHandler::Handler(handle_copy));
    registry.register("reload", NamedHandler::Handler(handle_reload));
    registry.register("diagnostics", NamedHandler::Handler(handle_diagnostics));
    registry.register("skills", NamedHandler::Handler(handle_skills));
    registry.register(
        "skill",
        NamedHandler::FormWithHandler {
            title: "Show Skill",
            fields: &[("Name", "skill-name", "name")],
            handler: run_skill,
        },
    );
    registry.register("create-skill", NamedHandler::Handler(handle_create_skill));
    registry.register("delete-skill", NamedHandler::Handler(handle_delete_skill));
    registry.register("reload-skills", NamedHandler::Handler(handle_reload_skills));
    registry.register("prompt", NamedHandler::Handler(handle_prompt));
    registry.register("hotkeys", NamedHandler::Handler(handle_hotkeys));
    registry.register("theme", NamedHandler::Handler(handle_theme));
    registry.register("approve", NamedHandler::Handler(handle_approve));
    registry.register("reject", NamedHandler::Handler(handle_reject));
    registry.register("provider", NamedHandler::Handler(handle_providers));
    registry.register("mcp-servers", NamedHandler::Handler(handle_mcp_servers));
    registry.register("skills-dialog", NamedHandler::Handler(handle_skills_dialog));
}

pub fn handle_copy(state: &mut AppState, _: &str) -> CommandResult {
    let text = state
        .session
        .messages
        .iter()
        .rev()
        .find(|m| m.role == crate::model::Role::Assistant)
        .map(|m| m.content())
        .unwrap_or_default();
    if text.is_empty() {
        return CommandResult::Message(crate::ui_strings::system::NOTHING_TO_COPY.into());
    }
    CommandResult::Event(crate::Event::CopyToClipboard(text))
}

pub fn handle_reload(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::Event::ReloadAll)
}

pub fn handle_settings(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::Event::ToggleSettingsDialog)
}

pub fn handle_diagnostics(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::Event::ShowDiagnostics)
}

pub fn handle_skills(state: &mut AppState, _: &str) -> CommandResult {
    use crate::ui_strings::system as s;
    if state.skills().is_empty() {
        return CommandResult::Warning(s::NO_SKILLS.into());
    }
    let lines: Vec<_> = std::iter::once(s::LOADED_SKILLS.into())
        .chain(
            state
                .skills()
                .iter()
                .map(|sk| format!("  {}", sk.summary())),
        )
        .collect();
    CommandResult::Message(lines.join("\n"))
}

/// Handler for `/skill <name>` — shows skill info.
pub fn run_skill(state: &mut AppState, args: &str) -> CommandResult {
    use crate::ui_strings::system as s;
    let name = args.trim();
    match state.skills().iter().find(|sk| sk.name == name) {
        Some(skill) => CommandResult::Message(s::skill_info(
            &skill.name,
            Some(&skill.description),
            Some(&skill.context),
        )),
        None => CommandResult::Message(s::skill_not_found(name)),
    }
}

/// Handler for `/create-skill <name>` — creates a skill from the standard
/// template in the user skills directory, then reloads the in-memory list.
pub fn handle_create_skill(state: &mut AppState, args: &str) -> CommandResult {
    use crate::ui_strings::system as s;
    let name = args.trim();
    match crate::skills::crud::create_skill(name) {
        Ok(path) => {
            state.set_skills(crate::skills::load_all());
            CommandResult::Message(s::skill_created(name, path.display()))
        }
        Err(e) => CommandResult::Warning(e),
    }
}

/// Handler for `/delete-skill <name>` — deletes a skill file (flat or nested)
/// from the user skills directory, then reloads the in-memory list.
pub fn handle_delete_skill(state: &mut AppState, args: &str) -> CommandResult {
    use crate::ui_strings::system as s;
    let name = args.trim();
    match crate::skills::crud::delete_skill(name) {
        Ok(path) => {
            state.set_skills(crate::skills::load_all());
            CommandResult::Message(s::skill_deleted(name, path.display()))
        }
        Err(e) => CommandResult::Warning(e),
    }
}

/// Handler for `/reload-skills` — re-scans skill directories and refreshes the
/// in-memory list without restarting.
pub fn handle_reload_skills(state: &mut AppState, _: &str) -> CommandResult {
    use crate::ui_strings::system as s;
    let skills = crate::skills::load_all();
    let count = skills.len();
    state.set_skills(skills);
    CommandResult::Message(s::skills_reloaded(count))
}

pub fn handle_theme(_state: &mut AppState, args: &str) -> CommandResult {
    let name = args.trim();
    if name.is_empty() {
        return CommandResult::OpenDialog(DialogType::ThemeSelector);
    }
    CommandResult::Event(crate::Event::SwitchTheme { name: name.to_owned() })
}

pub fn handle_approve(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::Event::ApproveEdit)
}

pub fn handle_reject(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::Event::RejectEdit)
}

pub fn handle_providers(_: &mut AppState, _args: &str) -> CommandResult {
    CommandResult::Event(crate::Event::ProvidersDialog)
}

pub fn handle_hotkeys(state: &mut AppState, _: &str) -> CommandResult {
    let mut panel = Panel::new("hotkeys", " Keyboard Shortcuts ");

    let mut bindings: Vec<_> = state
        .config
        .keybindings
        .iter()
        .map(|(combo, name)| (combo.clone(), name.clone()))
        .collect();
    bindings.sort_by(|a, b| a.0.cmp(&b.0));

    if bindings.is_empty() {
        panel = panel.header("No keybindings configured.");
    } else {
        panel = panel.header(format!("{} bindings", bindings.len()));
        for (index, (combo, name)) in bindings.into_iter().enumerate() {
            panel = panel.item_with_help(
                format!("{}  →  {}", combo, name),
                ItemAction::Push(format!("hotkey-detail-{index}-{name}")),
                shortcut_help(&name),
            );
        }
    }
    CommandResult::OpenPanelStack(Box::new(PanelStack::new(panel)))
}

/// Build the detail panel requested by a shortcuts-list row. The row index is
/// intentionally opaque to the panel DSL; activation resolves it lazily so
/// the root list remains the active panel until Enter is pressed.
pub fn hotkey_detail_panel(id: &str) -> Option<Panel> {
    let rest = id.strip_prefix("hotkey-detail-")?;
    let (_, action) = rest.split_once('-')?;
    let help = shortcut_help(action);
    Some(
        Panel::new(id, " Shortcut details ")
            .header(help)
            .item("_Esc back", ItemAction::Pop),
    )
}

/// Shared-DSL panels for the inline edit resubmit flow.
pub fn inline_edit_panel(id: &str) -> Option<Panel> {
    match id {
        "inline-edit-mode" => Some(
            Panel::new(id, " Resubmit from here ")
                .header("Choose what to regenerate")
                .item(
                    "_Conversation only",
                    ItemAction::Push("inline-edit-confirm".into()),
                )
                .item("_Cancel", ItemAction::Pop),
        ),
        "inline-edit-confirm" => Some(
            Panel::new(id, " Confirm resubmit ")
                .header("The conversation tail will be replaced by the edited prompt.")
                .item(
                    "_Confirm",
                    ItemAction::Emit(crate::Event::RunPaletteCommand {
                        name: "inline-edit-confirm".into(),
                        args: String::new(),
                    }),
                )
                .item("_Cancel", ItemAction::Pop),
        ),
        _ => None,
    }
}

fn shortcut_help(action: &str) -> &'static str {
    match action {
        "Abort" => {
            "Interrupts the current turn while keeping the conversation and composer available for the next action."
        }
        "ToggleExpand" => "Expands or collapses the selected feed content using the shared feed projection.",
        "ForceQuit" => "Immediately exits Runie and restores the terminal to its original mode.",
        "ToggleCommandPalette" => "Opens the shared command palette; slash and Ctrl+P use the same dialog surface.",
        "SendNow" => "Sends the current draft immediately, even while another turn or queued prompt is active.",
        "Quit" => "Clears the active draft first; with an empty composer, exits Runie cleanly.",
        "CursorEnd" => "Moves the composer cursor to the end of the current line.",
        "CursorStart" => "Moves the composer cursor to the start of the current line.",
        "Newline" => "Inserts a newline into the multi-line composer without submitting.",
        "ToggleTasksPane" => "Opens or closes the background tasks pane.",
        "OpenBlockViewer" => "Opens the selected feed block in the shared detail viewer.",
        "DeleteWord" => "Deletes the word immediately before the composer cursor.",
        "DeleteToEnd" => "Deletes from the cursor to the end of the current line.",
        "DeleteToStart" => "Deletes from the start of the current line to the cursor.",
        "KillChar" => "Deletes the character at the composer cursor.",
        "Suspend" => "Suspends the process and restores the terminal when resumed.",
        "Redo" => "Reapplies the most recently undone composer edit.",
        "ToggleQueuePane" => "Opens or closes the queued-prompts pane.",
        "OpenExternalEditor" => "Opens the current draft in the configured external editor.",
        "CopyLastResponse" => "Copies the most recent assistant response to the clipboard.",
        "NewSession" => "Starts a new session while preserving the session history.",
        "ResumeSession" => "Opens the session picker to resume a previous session.",
        "CycleModelNext" => "Switches to the next configured model.",
        "CycleModelPrev" => "Switches to the previous configured model.",
        "FollowUp" => "Adds the current draft as a follow-up while the turn is active.",
        "Dequeue" => "Removes the selected queued prompt without submitting it.",
        "CycleThinkingLevel" => "Cycles the model reasoning level for the next turn.",
        "PageUp" => "Scrolls the feed up by one viewport.",
        "PageDown" => "Scrolls the feed down by one viewport.",
        _ => "This shortcut is available in the current Runie screen and is handled by the shared input router.",
    }
}

pub fn handle_prompt(_state: &mut AppState, args: &str) -> CommandResult {
    CommandResult::Event(crate::Event::RunPromptCommand { name: args.trim().to_owned() })
}

/// Open the MCP servers management dialog.
pub fn handle_mcp_servers(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::OpenDialog(DialogType::McpServers)
}

/// Open the skills management dialog.
pub fn handle_skills_dialog(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::OpenDialog(DialogType::Skills)
}
