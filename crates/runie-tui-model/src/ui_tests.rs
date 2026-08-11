use super::{palette_labels, PaletteAction, UiMsg, UiState};

#[test]
fn skills_prefix_filters_and_namespaces_skill_rows() {
    let rows = palette_labels("skills:browser", &["browser-safe".into(), "docs".into()]);
    assert_eq!(rows, vec!["/skills:browser-safe"]);
}

#[test]
fn palette_uses_fuzzy_matching_but_keeps_raw_labels_for_execution() {
    assert_eq!(palette_labels("nws", &[]), vec!["New Session"]);
    assert!(palette_labels("hotkeys", &[]).contains(&"Keyboard Shortcuts".into()));
    assert!(palette_labels("scoped-models", &[]).contains(&"Scoped Models".into()));
    let rows = super::palette_display_rows("export", &[]);
    assert!(rows[0].contains("<Path (.jsonl)>"));
    assert!(rows[0].contains("builtin"));
}

#[test]
fn exact_slash_command_match_precedes_incidental_fuzzy_matches() {
    assert_eq!(
        super::palette_labels("clone", &[]).first(),
        Some(&"Clone Session".into())
    );
    assert_eq!(
        super::palette_labels("model", &[]).first(),
        Some(&"Select Model".into())
    );
}

#[test]
fn palette_registry_covers_every_mappable_builtin_command() {
    assert_eq!(PaletteAction::labels().len(), 74);
    for label in PaletteAction::labels() {
        assert!(PaletteAction::from_label(label).is_some(), "{label}");
    }
}

#[test]
#[allow(
    clippy::cognitive_complexity,
    reason = "This is one declarative palette metadata matrix"
)]
fn parameterized_palette_actions_are_marked_for_nested_forms() {
    assert!(PaletteAction::SetSessionName.requires_parameters());
    assert!(PaletteAction::UndoSession.requires_parameters());
    assert!(PaletteAction::ExportSession.requires_parameters());
    assert!(PaletteAction::SelectTheme.requires_parameters());
    assert!(!PaletteAction::SelectModel.requires_parameters());
    assert!(!PaletteAction::NewSession.requires_parameters());
    assert_eq!(PaletteAction::ForkSession.slash_command(), "/fork");
    assert_eq!(PaletteAction::GitStatus.slash_command(), "/git status");
    assert_eq!(PaletteAction::GitDiff.slash_command(), "/git diff");
    assert_eq!(PaletteAction::GitReview.slash_command(), "/git review");
    assert_eq!(
        PaletteAction::GitWorktrees.slash_command(),
        "/git worktrees"
    );
    assert_eq!(
        PaletteAction::GitConflicts.slash_command(),
        "/git conflicts"
    );
    assert_eq!(
        PaletteAction::CancelAllJobs.slash_command(),
        "/jobs cancel all"
    );
    assert_eq!(
        PaletteAction::ClearFinishedJobs.slash_command(),
        "/jobs clear finished"
    );
    assert_eq!(
        PaletteAction::McpReady.slash_command(),
        "/mcps status=ready"
    );
    assert_eq!(
        PaletteAction::McpFailed.slash_command(),
        "/mcps status=failed"
    );
    assert_eq!(PaletteAction::McpStdio.slash_command(), "/mcps stdio");
    assert_eq!(PaletteAction::McpHttp.slash_command(), "/mcps http");
    assert_eq!(PaletteAction::UndoSession.slash_command(), "/undo");
}

#[test]
fn command_result_is_renderable_as_a_stack_owned_dialog() {
    let state = UiState::new().update(UiMsg::ShowCommandResult("Approval mode: Auto".into()));
    assert_eq!(state.dialog_stack.top_id(), Some("command-result"));
    assert_eq!(state.command_result.as_deref(), Some("Approval mode: Auto"));
    let state = state.update(UiMsg::DialogEscape);
    assert!(state.dialog_stack.is_empty());
    assert!(state.command_result.is_none());
}

#[test]
fn select_model_palette_entry_opens_a_picker_instead_of_a_text_form() {
    let rows = super::palette_display_rows("model", &[]);
    assert!(rows.iter().any(|row| row.starts_with("Select Model  · ")));
    assert!(rows
        .iter()
        .filter(|row| row.starts_with("Select Model"))
        .all(|row| !row.contains("<provider/model>")));

    let state = UiState::new()
        .update(UiMsg::ToggleCommandPalette)
        .update(UiMsg::CommandPaletteChar('m'))
        .update(UiMsg::CommandPaletteChar('o'))
        .update(UiMsg::CommandPaletteChar('d'))
        .update(UiMsg::CommandPaletteChar('e'))
        .update(UiMsg::CommandPaletteChar('l'))
        .update(UiMsg::ActivateCommandPalette);
    assert_eq!(state.last_palette_command.as_deref(), Some("Select Model"));
    assert!(state.palette_parameter_action.is_none());
}

#[test]
fn parameter_selection_pushes_shared_form_and_submit_pops_it() {
    let state = UiState::new().update(UiMsg::OpenPaletteParameters(PaletteAction::ExportSession));
    assert_eq!(state.dialog_stack.top_id(), Some("palette-parameters"));
    assert_eq!(
        state.palette_parameter_action,
        Some(PaletteAction::ExportSession)
    );
    let state = state
        .update(UiMsg::PaletteParameterChar('x'))
        .update(UiMsg::PaletteParameterSubmit);
    assert!(state.dialog_stack.is_empty());
    assert!(state.palette_parameter_action.is_none());
}

#[test]
fn parameter_dialog_escape_pops_back_even_when_query_has_text() {
    let state = UiState::new()
        .update(UiMsg::OpenPaletteParameters(PaletteAction::SetSessionName))
        .update(UiMsg::PaletteParameterChar('x'))
        .update(UiMsg::DialogEscape);
    assert!(state.dialog_stack.is_empty());
}

#[test]
fn escape_pops_nested_parameter_dialog_back_to_command_palette() {
    let mut state = UiState::new()
        .update(UiMsg::ToggleCommandPalette)
        .update(UiMsg::CommandPaletteChar('n'))
        .update(UiMsg::CommandPaletteChar('a'))
        .update(UiMsg::CommandPaletteChar('m'))
        .update(UiMsg::CommandPaletteChar('e'))
        .update(UiMsg::ActivateCommandPalette)
        .update(UiMsg::OpenPaletteParameters(PaletteAction::SetSessionName));
    assert_eq!(state.dialog_stack.top_id(), Some("palette-parameters"));
    assert_eq!(state.dialog_stack.depth(), 2);

    state = state.update(UiMsg::DialogEscape);
    assert_eq!(state.dialog_stack.top_id(), Some("commands"));
    assert!(state.command_palette_open);
}

#[test]
fn escaping_theme_parameters_restores_the_parent_filter_until_root_escape() {
    let state = UiState::new()
        .update(UiMsg::ToggleCommandPalette)
        .update(UiMsg::CommandPaletteChar('t'))
        .update(UiMsg::CommandPaletteChar('h'))
        .update(UiMsg::CommandPaletteChar('e'))
        .update(UiMsg::CommandPaletteChar('m'))
        .update(UiMsg::CommandPaletteChar('e'))
        .update(UiMsg::ActivateCommandPalette)
        .update(UiMsg::OpenPaletteParameters(PaletteAction::SelectTheme));
    assert_eq!(state.dialog_stack.top_id(), Some("palette-parameters"));
    assert_eq!(state.command_palette_query, "theme");
    let state = state.update(UiMsg::DialogEscape);
    assert_eq!(state.dialog_stack.top_id(), Some("commands"));
    assert_eq!(state.command_palette_query, "theme");
    let state = state.update(UiMsg::DialogEscape);
    assert_eq!(state.dialog_stack.top_id(), Some("commands"));
    assert!(state.command_palette_query.is_empty());
    let state = state.update(UiMsg::DialogEscape);
    assert!(state.dialog_stack.is_empty());
}

#[test]
fn command_palette_backspace_removes_the_last_filter_character() {
    let state = UiState::new()
        .update(UiMsg::ToggleCommandPalette)
        .update(UiMsg::CommandPaletteChar('t'))
        .update(UiMsg::CommandPaletteChar('h'))
        .update(UiMsg::CommandPaletteBackspace);
    assert_eq!(state.command_palette_query, "t");
}

#[test]
fn escape_from_model_selector_returns_to_command_palette() {
    let state = UiState::new()
        .update(UiMsg::ToggleCommandPalette)
        .update(UiMsg::CommandPaletteChar('m'))
        .update(UiMsg::CommandPaletteChar('o'))
        .update(UiMsg::CommandPaletteChar('d'))
        .update(UiMsg::CommandPaletteChar('e'))
        .update(UiMsg::CommandPaletteChar('l'))
        .update(UiMsg::ActivateCommandPalette)
        .update(UiMsg::ToggleModelSelector);
    assert_eq!(state.dialog_stack.top_id(), Some("model"));
    assert_eq!(state.dialog_stack.depth(), 2);
    let state = state.update(UiMsg::DialogEscape);
    assert_eq!(state.dialog_stack.top_id(), Some("commands"));
    assert!(state.command_palette_open);
}

#[test]
fn enter_on_model_selection_applies_and_pops_the_model_dialog() {
    let state = UiState::new()
        .update(UiMsg::ToggleCommandPalette)
        .update(UiMsg::ToggleModelSelector)
        .update(UiMsg::ActivateModelSelector);
    assert!(!state.model_selector_open);
    assert!(!state.command_palette_open);
    assert!(state.dialog_stack.is_empty());
}

#[test]
fn every_parameterized_palette_action_completes_shared_form_flow() {
    for action in parameterized_palette_actions() {
        let state = UiState::new()
            .update(UiMsg::OpenPaletteParameters(action.clone()))
            .update(UiMsg::PaletteParameterChar('x'))
            .update(UiMsg::PaletteParameterSubmit);
        assert!(
            state.dialog_stack.is_empty(),
            "{action:?} left a dialog open"
        );
        assert!(
            state.palette_parameter_action.is_none(),
            "{action:?} left form state behind"
        );
    }
}

fn parameterized_palette_actions() -> Vec<PaletteAction> {
    vec![
        PaletteAction::SetSessionName,
        PaletteAction::CompactContext,
        PaletteAction::ForkSession,
        PaletteAction::SelectTreeEntry,
        PaletteAction::ExportSession,
        PaletteAction::ImportSession,
        PaletteAction::CloneSession,
        PaletteAction::ResumeSession,
        PaletteAction::Help,
        PaletteAction::Settings,
        PaletteAction::Doctor,
        PaletteAction::RewindSession,
        PaletteAction::PromptHistory,
        PaletteAction::FindTranscript,
        PaletteAction::JumpTranscript,
        PaletteAction::SetEffort,
        PaletteAction::AlwaysApprove,
        PaletteAction::AutoApprove,
        PaletteAction::PlanMode,
        PaletteAction::Login,
        PaletteAction::Logout,
        PaletteAction::TrustProject,
        PaletteAction::Remember,
        PaletteAction::Goal,
        PaletteAction::Workflow,
        PaletteAction::Loop,
        PaletteAction::DeepResearch,
        PaletteAction::Feedback,
        PaletteAction::Usage,
    ]
}

#[test]
fn selecting_skill_records_typed_skill_command() {
    let mut state = UiState::new()
        .update(UiMsg::SetSkillRows(vec!["browser-safe".into()]))
        .update(UiMsg::ToggleCommandPalette);
    for ch in "skills:".chars() {
        state = state.update(UiMsg::CommandPaletteChar(ch));
    }
    state = state.update(UiMsg::ActivateCommandPalette);
    assert_eq!(
        state.last_skill_command.as_deref(),
        Some("/skills:browser-safe")
    );
    assert_eq!(state.last_palette_command, None);
}

#[test]
fn skill_palette_selection_uses_dynamic_skill_count() {
    let skills = (0..20).map(|index| format!("skill-{index:02}")).collect();
    let state = UiState::new()
        .update(UiMsg::SetSkillRows(skills))
        .update(UiMsg::ToggleCommandPalette);
    let state = "skills:".chars().fold(state, |state, ch| {
        state.update(UiMsg::CommandPaletteChar(ch))
    });
    let state = state.update(UiMsg::CommandPaletteMove(19));
    assert_eq!(state.command_palette_index, 19);
}

#[test]
fn dialog_arrow_navigation_wraps_at_both_boundaries() {
    let palette = UiState::new().update(UiMsg::ToggleCommandPalette);
    let palette = palette.update(UiMsg::CommandPaletteMove(-1));
    let palette_count = palette_labels("", &[]).len();
    assert_eq!(palette.command_palette_index, palette_count - 1);
    let palette = palette.update(UiMsg::CommandPaletteMove(1));
    assert_eq!(palette.command_palette_index, 0);

    let model = UiState::new()
        .update(UiMsg::SetModelSelectorResultCount(3))
        .update(UiMsg::ToggleModelSelector)
        .update(UiMsg::ModelSelectorMove(-1));
    assert_eq!(model.model_selector_index, 2);
    let model = model.update(UiMsg::ModelSelectorMove(1));
    assert_eq!(model.model_selector_index, 0);

    let themes = UiState::new()
        .update(UiMsg::OpenPaletteParameters(PaletteAction::SelectTheme))
        .update(UiMsg::PaletteParameterMove(-1));
    assert_eq!(
        themes.dialog_stack.top().expect("theme dialog").selected,
        crate::theme_labels().len() - 1
    );
}

#[test]
fn every_overlay_toggle_uses_the_dialog_stack() {
    let state = UiState::new().update(UiMsg::ToggleShortcuts);
    assert_eq!(state.dialog_stack.top_id(), Some("shortcuts"));
    let state = state.update(UiMsg::DialogEscape);
    assert!(state.dialog_stack.is_empty());
    assert!(!state.shortcuts_open);

    let state = UiState::new().update(UiMsg::ToggleSessionInfo);
    assert_eq!(state.dialog_stack.top_id(), Some("session"));
    let state = state.update(UiMsg::ToggleChangelog);
    assert_eq!(state.dialog_stack.top_id(), Some("changelog"));
    assert!(state.session_info_open);
    assert!(state.changelog_open);
}

#[test]
fn user_question_dialog_selects_and_projects_answer() {
    let question = runie_core::tools::PendingUserQuestion {
        id: "q1".into(),
        request: runie_core::tools::UserQuestionRequest {
            question: "Continue?".into(),
            header: None,
            body: None,
            options: vec![
                runie_core::tools::UserQuestionOption {
                    id: None,
                    label: "Yes".into(),
                    description: String::new(),
                },
                runie_core::tools::UserQuestionOption {
                    id: None,
                    label: "No".into(),
                    description: String::new(),
                },
            ],
            allow_multiple: false,
        },
    };
    let state = UiState::new()
        .update(UiMsg::OpenUserQuestion(question))
        .update(UiMsg::UserQuestionMove(1));
    assert_eq!(state.dialog_stack.top_id(), Some("user-question"));
    assert_eq!(state.dialog_stack.top().unwrap().selected, 1);
}

#[test]
fn multi_select_question_toggles_selected_options() {
    let question = runie_core::tools::PendingUserQuestion {
        id: "q2".into(),
        request: runie_core::tools::UserQuestionRequest {
            question: "Which?".into(),
            header: None,
            body: None,
            options: vec![
                runie_core::tools::UserQuestionOption {
                    id: None,
                    label: "A".into(),
                    description: String::new(),
                },
                runie_core::tools::UserQuestionOption {
                    id: None,
                    label: "B".into(),
                    description: String::new(),
                },
            ],
            allow_multiple: true,
        },
    };
    let state = UiState::new()
        .update(UiMsg::OpenUserQuestion(question))
        .update(UiMsg::ToggleUserQuestionSelection)
        .update(UiMsg::UserQuestionMove(1))
        .update(UiMsg::ToggleUserQuestionSelection);
    assert_eq!(state.user_question_selected, vec![0, 1]);
}
