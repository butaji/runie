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
        UiMsg::SubmitUserQuestion => question_command(state),
        UiMsg::ActivateCommandPalette => palette_command_for(state, message),
        _ => None,
    }
}

fn question_command(state: &UiState) -> Option<UiCommand> {
    let question = state.user_question.as_ref()?;
    let selected = state
        .dialog_stack
        .top()
        .map(|frame| frame.selected)
        .unwrap_or_default();
    let selected = if question.request.allow_multiple && !state.user_question_selected.is_empty() {
        state.user_question_selected.clone()
    } else {
        vec![selected]
    };
    Some(UiCommand::AnswerUserQuestion {
        id: question.id.clone(),
        answer: serde_json::json!({"answers": selected.into_iter().filter_map(|index| question.request.options.get(index).map(|option| option.label.clone())).collect::<Vec<_>>() }),
    })
}

fn parameter_command(state: &UiState) -> Option<UiCommand> {
    let action = state.palette_parameter_action.as_ref()?;
    let frame = state.dialog_stack.top()?;
    let query = frame.query.trim();
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
    if *action == PaletteAction::SetEffort && query.is_empty() {
        return Some(UiCommand::ExecuteMappable(
            runie_core::commands::MappableBuiltinCommand::Extended {
                name: "effort".into(),
                args: state.palette_parameter_options.get(frame.selected)?.clone(),
            },
        ));
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
