//! Settings dialog tests (Layer 2 + Layer 3)

use crate::commands::DialogState;
use crate::event::Event;
use crate::model::{AppState, DeliveryMode};
use crate::settings::SettingsCategory;

#[test]
fn settings_opens_dialog() {
    let mut state = AppState::default();
    for c in "/settings".chars() {
        state.update(Event::Input(c));
    }
    state.update(Event::Submit);
    assert!(
        matches!(state.open_dialog, Some(DialogState::Settings { .. })),
        "Expected Settings dialog, got {:?}",
        state.open_dialog
    );
}

#[test]
fn settings_navigates_up() {
    let mut state = AppState::default();
    state.open_dialog = Some(DialogState::Settings {
        category: SettingsCategory::Models,
        selected: 1,
    });
    state.update(Event::SettingsUp);
    if let Some(DialogState::Settings { selected, .. }) = state.open_dialog {
        assert_eq!(selected, 0);
    } else {
        panic!("Dialog should still be open");
    }
}

#[test]
fn settings_navigates_down_wraps() {
    let mut state = AppState::default();
    state.open_dialog = Some(DialogState::Settings {
        category: SettingsCategory::Safety,
        selected: 0,
    });
    state.update(Event::SettingsDown);
    if let Some(DialogState::Settings { selected, .. }) = state.open_dialog {
        assert_eq!(selected, 0, "Single item should stay at 0");
    } else {
        panic!("Dialog should still be open");
    }
}

#[test]
fn settings_left_changes_category() {
    let mut state = AppState::default();
    state.open_dialog = Some(DialogState::Settings {
        category: SettingsCategory::Models,
        selected: 0,
    });
    state.update(Event::SettingsLeft);
    if let Some(DialogState::Settings { category, selected }) = state.open_dialog {
        assert_eq!(category, SettingsCategory::Safety, "Left from first goes to last");
        assert_eq!(selected, 0);
    } else {
        panic!("Dialog should still be open");
    }
}

#[test]
fn settings_right_changes_category() {
    let mut state = AppState::default();
    state.open_dialog = Some(DialogState::Settings {
        category: SettingsCategory::Safety,
        selected: 0,
    });
    state.update(Event::SettingsRight);
    if let Some(DialogState::Settings { category, selected }) = state.open_dialog {
        assert_eq!(category, SettingsCategory::Models, "Right from last goes to first");
        assert_eq!(selected, 0);
    } else {
        panic!("Dialog should still be open");
    }
}

#[test]
fn settings_select_toggles_read_only() {
    let mut state = AppState::default();
    state.read_only = false;
    state.open_dialog = Some(DialogState::Settings {
        category: SettingsCategory::Safety,
        selected: 0,
    });
    state.update(Event::SettingsSelect);
    assert!(state.read_only);
}

#[test]
fn settings_esc_closes() {
    let mut state = AppState::default();
    state.open_dialog = Some(DialogState::Settings {
        category: SettingsCategory::Models,
        selected: 0,
    });
    state.update(Event::Abort);
    assert!(state.open_dialog.is_none());
}

#[test]
fn settings_select_toggles_steering_mode() {
    let mut state = AppState::default();
    state.open_dialog = Some(DialogState::Settings {
        category: SettingsCategory::Behavior,
        selected: 1,
    });
    assert!(matches!(state.steering_mode, DeliveryMode::OneAtATime));
    state.update(Event::SettingsSelect);
    assert!(matches!(state.steering_mode, DeliveryMode::All));
}
