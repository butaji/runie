//! Model & Config Event Handler

use crate::model::AppState;
use crate::Event;

pub fn model_config_event(state: &mut AppState, event: Event) {
    match event {
        Event::SwitchModel { provider, model } => state.switch_model(provider, model),
        Event::SwitchTheme { name } => state.switch_theme(name),
        Event::CycleModelNext => state.cycle_model(1),
        Event::CycleModelPrev => state.cycle_model(-1),
        Event::CycleThinkingLevel => state.cycle_thinking_level(),
        Event::SetThinkingLevel(level) => state.set_thinking_level(level),
        Event::ToggleReadOnly => state.toggle_read_only(),
        Event::TrustProject => state.trust_project(),
        Event::UntrustProject => state.untrust_project(),
        Event::FollowUp => state.queue_follow_up(),
        Event::Dequeue => state.dequeue(),
        _ => {}
    }
}
