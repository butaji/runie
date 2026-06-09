use crate::model::ThinkingLevel;
use crate::session::{Session, Store};
use crate::{Event, AppState};

#[test]
fn cycle_rotates() {
    assert_eq!(ThinkingLevel::Off.cycle(), ThinkingLevel::Low);
    assert_eq!(ThinkingLevel::Low.cycle(), ThinkingLevel::Medium);
    assert_eq!(ThinkingLevel::Medium.cycle(), ThinkingLevel::High);
    assert_eq!(ThinkingLevel::High.cycle(), ThinkingLevel::Off);
}

#[test]
fn prompt_suffix_matches() {
    assert_eq!(ThinkingLevel::Off.prompt_suffix(), "");
    assert_eq!(ThinkingLevel::Low.prompt_suffix(), "\nThink briefly before responding.");
    assert_eq!(ThinkingLevel::Medium.prompt_suffix(), "\nThink step by step before responding.");
    assert_eq!(ThinkingLevel::High.prompt_suffix(), "\nThink deeply and thoroughly. Consider edge cases and alternatives.");
}

#[test]
fn from_str_parses_levels() {
    assert_eq!("off".parse::<ThinkingLevel>().unwrap(), ThinkingLevel::Off);
    assert_eq!("low".parse::<ThinkingLevel>().unwrap(), ThinkingLevel::Low);
    assert_eq!("medium".parse::<ThinkingLevel>().unwrap(), ThinkingLevel::Medium);
    assert_eq!("high".parse::<ThinkingLevel>().unwrap(), ThinkingLevel::High);
    assert!("unknown".parse::<ThinkingLevel>().is_err());
}

#[test]
fn session_persists_thinking_level() {
    let dir = std::env::temp_dir().join(format!("runie_think_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let store = Store::new(dir);

    let session = Session {
        name: "think_test".to_string(),
        created_at: 1.0,
        updated_at: 2.0,
        messages: vec![],
        provider: "mock".into(),
        model: "echo".into(),
        theme_name: "default".into(),
        thinking_level: ThinkingLevel::Medium,
        read_only: false,
    };

    store.save("think_test", &session).unwrap();
    let loaded = store.load("think_test").unwrap();
    assert_eq!(loaded.thinking_level, ThinkingLevel::Medium);
}

#[test]
fn shift_tab_cycles() {
    let mut state = AppState::default();
    assert_eq!(state.thinking_level, ThinkingLevel::Off);

    state.update(Event::CycleThinkingLevel);
    assert_eq!(state.thinking_level, ThinkingLevel::Low);

    state.update(Event::CycleThinkingLevel);
    assert_eq!(state.thinking_level, ThinkingLevel::Medium);

    state.update(Event::CycleThinkingLevel);
    assert_eq!(state.thinking_level, ThinkingLevel::High);

    state.update(Event::CycleThinkingLevel);
    assert_eq!(state.thinking_level, ThinkingLevel::Off);
}

#[test]
fn slash_thinking_sets() {
    let mut state = AppState::default();
    state.input.push_str("/thinking high");
    state.update(Event::Submit);
    assert_eq!(state.thinking_level, ThinkingLevel::High);

    let sys_msgs: Vec<_> = state.messages.iter().filter(|m| m.role == crate::model::Role::System).collect();
    assert!(sys_msgs.iter().any(|m| m.content.contains("Thinking level set to: high")));
}

#[test]
fn slash_thinking_no_args_shows_current() {
    let mut state = AppState::default();
    state.thinking_level = ThinkingLevel::Medium;
    state.input.push_str("/thinking");
    state.update(Event::Submit);

    let sys_msgs: Vec<_> = state.messages.iter().filter(|m| m.role == crate::model::Role::System).collect();
    assert!(sys_msgs.iter().any(|m| m.content.contains("Current thinking level: medium")));
}

#[test]
fn set_thinking_level_event_updates_state() {
    let mut state = AppState::default();
    state.update(Event::SetThinkingLevel(ThinkingLevel::High));
    assert_eq!(state.thinking_level, ThinkingLevel::High);
}
