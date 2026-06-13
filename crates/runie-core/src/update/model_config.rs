//! Model & Config Event Handler

use crate::model::AppState;
use crate::Event;

pub fn model_config_event(state: &mut AppState, event: Event) {
    let mut invalidate_settings = false;
    match event {
        Event::SwitchModel { provider, model } => {
            state.switch_model(provider, model);
            invalidate_settings = true;
        }
        Event::SwitchTheme { name } => {
            state.switch_theme(name);
            invalidate_settings = true;
        }
        Event::CycleModelNext => state.cycle_model(1),
        Event::CycleModelPrev => state.cycle_model(-1),
        Event::CycleThinkingLevel => {
            state.cycle_thinking_level();
            invalidate_settings = true;
        }
        Event::SetThinkingLevel(level) => {
            state.set_thinking_level(level);
            invalidate_settings = true;
        }
        Event::ToggleReadOnly => {
            state.toggle_read_only();
            invalidate_settings = true;
        }
        Event::TrustProject => state.trust_project(),
        Event::UntrustProject => state.untrust_project(),
        Event::FollowUp => state.queue_follow_up(),
        Event::Dequeue => state.dequeue(),
        _ => {}
    }
    if invalidate_settings {
        state.view.cached_settings_valid = false;
    }
}
