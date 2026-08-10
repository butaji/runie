use super::*;
pub(super) fn palette_command_for(state: &UiState, message: &UiMsg) -> Option<UiCommand> {
    if !matches!(message, UiMsg::ActivateCommandPalette) {
        return None;
    }
    if let Some(skill) = state.last_skill_command.as_deref() {
        return Some(UiCommand::ActivateSkill(
            skill.trim_start_matches("/skills:").to_owned(),
        ));
    }
    runie_tui_model::palette_labels(&state.command_palette_query, &state.skill_rows)
        .get(state.command_palette_index)
        .and_then(|entry| palette_action_for(entry))
        .map(|action| {
            if action.requires_parameters() && action != PaletteAction::ManageProviders {
                UiCommand::OpenPaletteParameters(action)
            } else {
                UiCommand::ActivatePaletteEntry(action)
            }
        })
}

pub(super) fn ui_command_for(state: &UiState, message: &UiMsg) -> Option<UiCommand> {
    match message {
        UiMsg::CopyText(text) => Some(UiCommand::CopyText(text.clone())),
        UiMsg::PaletteParameterSubmit => parameter_command(state),
        UiMsg::PaletteParameterPreview => parameter_command(state),
        UiMsg::ActivateCommandPalette => palette_command_for(state, message),
        UiMsg::HideWelcome
        | UiMsg::ToggleShortcuts
        | UiMsg::ToggleCommandPalette
        | UiMsg::CommandPaletteChar(_)
        | UiMsg::CommandPaletteBackspace
        | UiMsg::CommandPaletteMove(_)
        | UiMsg::CommandPaletteEscape
        | UiMsg::DialogEscape
        | UiMsg::CloseDialogs
        | UiMsg::OpenFileDialog
        | UiMsg::OpenPaletteParameters(_)
        | UiMsg::PaletteParameterChar(_)
        | UiMsg::PaletteParameterBackspace
        | UiMsg::PaletteParameterMove(_)
        | UiMsg::ToggleModelSelector
        | UiMsg::ModelSelectorChar(_)
        | UiMsg::ModelSelectorBackspace
        | UiMsg::ModelSelectorMove(_)
        | UiMsg::ModelSelectorEscape
        | UiMsg::ModelSelectorToggleScope
        | UiMsg::ActivateModelSelector
        | UiMsg::SetModelSelectorResultCount(_)
        | UiMsg::SetModelSelectorRows(_)
        | UiMsg::SetSkillRows(_)
        | UiMsg::ShowCommandResult(_)
        | UiMsg::ToggleSessionInfo
        | UiMsg::ToggleChangelog
        | UiMsg::Reset => None,
    }
}

fn parameter_command(state: &UiState) -> Option<UiCommand> {
    let action = state.palette_parameter_action.as_ref()?;
    let query = state.dialog_stack.top()?.query.trim();
    if *action == PaletteAction::SelectTheme {
        let value = if query.is_empty() {
            runie_tui_model::theme_labels()
                .get(state.dialog_stack.top()?.selected)
                .copied()?
        } else {
            query
        };
        return Some(UiCommand::SelectTheme(value.to_owned()));
    }
    if *action == PaletteAction::ManageProviders {
        return Some(UiCommand::ProviderAction {
            action: action.clone(),
            value: query.to_owned(),
        });
    }
    let input = format!("{} {query}", action.slash_command());
    Some(UiCommand::ExecuteMappable(
        runie_core::commands::parse_mappable_builtin_command(&input).unwrap_or_else(|| {
            runie_core::commands::MappableBuiltinCommand::Extended {
                name: action.slash_command().trim_start_matches('/').to_owned(),
                args: query.to_owned(),
            }
        }),
    ))
}

pub(super) fn palette_action_for(entry: &str) -> Option<PaletteAction> {
    PaletteAction::from_label(entry)
}

pub(super) fn initial_ui_state(show_welcome: bool) -> UiState {
    if show_welcome {
        UiState::with_welcome()
    } else {
        UiState::new()
    }
}

pub(super) fn model_selector_rows(
    snapshot: &runie_core::model_catalog::ModelCatalogSnapshot,
) -> Vec<String> {
    runie_tui_model::model_selector_rows(snapshot)
}
