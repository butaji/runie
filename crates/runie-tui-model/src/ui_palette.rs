fn is_overlay_message(msg: &UiMsg) -> bool {
    matches!(
        msg,
        UiMsg::CommandPaletteChar(_)
            | UiMsg::CommandPaletteBackspace
            | UiMsg::CommandPaletteMove(_)
            | UiMsg::CommandPaletteEscape
            | UiMsg::ActivateCommandPalette
            | UiMsg::ModelSelectorChar(_)
            | UiMsg::ModelSelectorBackspace
            | UiMsg::ModelSelectorMove(_)
            | UiMsg::ModelSelectorEscape
            | UiMsg::ModelSelectorToggleScope
            | UiMsg::ActivateModelSelector
            | UiMsg::PaletteParameterChar(_)
            | UiMsg::PaletteParameterBackspace
            | UiMsg::PaletteParameterMove(_)
            | UiMsg::PaletteParameterPreview
            | UiMsg::PaletteParameterSubmit
            | UiMsg::UserQuestionMove(_)
            | UiMsg::SubmitUserQuestion
    )
}

pub fn palette_labels(query: &str, skills: &[String]) -> Vec<String> {
    let query = query.trim().to_ascii_lowercase();
    if query.starts_with("skills:") {
        let skill_query = query.trim_start_matches("skills:").trim();
        return skills
            .iter()
            .filter(|skill| fuzzy_match(skill, skill_query))
            .map(|skill| format!("/skills:{skill}"))
            .collect();
    }
    let mut matches: Vec<_> = PaletteAction::labels()
        .iter()
        .copied()
        .filter(|label| {
            fuzzy_match(label, &query)
                || PaletteAction::from_label(label).is_some_and(|action| {
                    fuzzy_match(action.slash_command().trim_start_matches('/'), &query)
                })
        })
        .map(str::to_owned)
        .collect();
    matches.sort_by_key(|label| {
        let action = PaletteAction::from_label(label);
        let command = action.map(|action| action.slash_command().trim_start_matches('/'));
        (
            command != Some(query.as_str()),
            !label.to_ascii_lowercase().starts_with(&query),
        )
    });
    matches
}

fn fuzzy_match(candidate: &str, query: &str) -> bool {
    let lower_candidate = candidate.to_ascii_lowercase();
    let mut chars = lower_candidate.chars();
    query
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .all(|needle| chars.by_ref().any(|ch| ch == needle))
}
