use crate::model::AppState;

use super::{add_minimax_provider, clean_config, select_minimax_model};

#[test]
fn providers_select_model_switches_active_model() {
    clean_config();
    let mut state = AppState::default();

    add_minimax_provider(&mut state);
    select_minimax_model(&mut state);

    assert_eq!(state.config.current_provider, "minimax");
    assert_eq!(state.config.current_model, "MiniMax-M3");
}

#[test]
fn providers_select_model_closes_dialog() {
    clean_config();
    let mut state = AppState::default();

    add_minimax_provider(&mut state);
    select_minimax_model(&mut state);

    assert!(
        state.open_dialog.is_none(),
        "selecting a model should close the dialog"
    );
}

#[test]
fn providers_select_model_records_usage() {
    clean_config();
    let mut state = AppState::default();

    add_minimax_provider(&mut state);
    select_minimax_model(&mut state);

    assert!(
        state
            .config
            .recent_models
            .iter()
            .any(|m| m.contains("minimax")),
        "model usage should be recorded in recent_models"
    );
}
