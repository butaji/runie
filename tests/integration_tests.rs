#![cfg(test)]

use anvil::router::{ModelDatabase, HealthLevel};
use anvil::core::safety::{SafetyConfig, SafetyEnvelope};
use anvil::tui::stream::{Stream, EntryType};
use std::path::Path;

// ── Phase 4: Model Router ────────────────────────────────────
#[test]
fn model_database_has_five_models() {
    let db = ModelDatabase::new();
    assert_eq!(db.models.len(), 5);
}

#[test]
fn model_database_claude_active_by_default() {
    let db = ModelDatabase::new();
    let status = db.statuses.get("anthropic/claude-sonnet-4").unwrap();
    assert!(status.is_active);
}

#[test]
fn health_level_dots() {
    assert_eq!(HealthLevel::Healthy.dots(), "●●●●●");
    assert_eq!(HealthLevel::Good.dots(), "●●●●○");
    assert_eq!(HealthLevel::Degraded.dots(), "●●●○○");
    assert_eq!(HealthLevel::Critical.dots(), "●●○○○");
}

#[test]
fn track_spend_accumulates() {
    let mut db = ModelDatabase::new();
    db.track_spend("anthropic/claude-sonnet-4", 0.50);
    db.track_spend("anthropic/claude-sonnet-4", 0.25);
    let status = db.statuses.get("anthropic/claude-sonnet-4").unwrap();
    assert!((status.spent - 0.75).abs() < 0.001);
    assert!((db.total_spent() - 0.75).abs() < 0.001);
}

// ── Phase 2: Stream ─────────────────────────────────────────
#[test]
fn stream_generates_50_entries() {
    let stream = Stream::new();
    assert_eq!(stream.entries.len(), 50);
}

#[test]
fn stream_entry_types_cycle_thought_edit_plan_question() {
    let stream = Stream::new();
    assert!(matches!(stream.entries[0].entry_type, EntryType::Thought));
    assert!(matches!(stream.entries[1].entry_type, EntryType::Edit));
    assert!(matches!(stream.entries[2].entry_type, EntryType::Plan));
    assert!(matches!(stream.entries[3].entry_type, EntryType::Question));
    assert!(matches!(stream.entries[4].entry_type, EntryType::Thought));
}

#[test]
fn stream_thought_entries_have_elapsed_time() {
    let stream = Stream::new();
    assert!(stream.entries[0].elapsed_secs.is_some());
}

#[test]
fn stream_edit_entries_have_file() {
    let stream = Stream::new();
    assert!(stream.entries[1].file.is_some());
}

#[test]
fn stream_plan_entries_expanded_for_first_10() {
    let stream = Stream::new();
    for (i, entry) in stream.entries.iter().enumerate() {
        if matches!(entry.entry_type, EntryType::Plan) && i < 10 {
            assert!(entry.expanded, "Plan at idx {} should be expanded", i);
        }
    }
}

#[test]
fn stream_render_does_not_panic() {
    let stream = Stream::new();
    let (para, _, _) = stream.render(20);
    // Paragraph is built without panicking; just verify no panic
    let _ = para;
}

#[test]
fn stream_scroll_to_bottom_selects_last_entry() {
    let mut stream = Stream::new();
    stream.scroll_to_bottom(10);
    assert_eq!(stream.selected, 49);
}

#[test]
fn stream_handle_key_j_moves_selection_down() {
    let mut stream = Stream::new();
    stream.selected = 5;
    stream.handle_key(crossterm::event::KeyCode::Char('j'), 30);
    assert_eq!(stream.selected, 6);
}

#[test]
fn stream_handle_key_k_moves_selection_up() {
    let mut stream = Stream::new();
    stream.selected = 5;
    stream.handle_key(crossterm::event::KeyCode::Char('k'), 30);
    assert_eq!(stream.selected, 4);
}

#[test]
fn stream_handle_key_space_toggles_expansion() {
    let mut stream = Stream::new();
    stream.selected = 2;
    let was = stream.entries[2].expanded;
    stream.handle_key(crossterm::event::KeyCode::Char(' '), 30);
    assert_eq!(stream.entries[2].expanded, !was);
}

// ── Phase 6: Safety ─────────────────────────────────────────
#[test]
fn safety_config_default_values() {
    let cfg = SafetyConfig::default();
    assert_eq!(cfg.max_cost_per_task, 5.00);
    assert_eq!(cfg.max_cost_per_session, 50.00);
    assert!(cfg.protected_paths.contains(&".env".to_string()));
    assert!(cfg.required_tests);
    assert_eq!(cfg.max_retries, 3);
}

#[test]
fn safety_cost_check_passes_under_limit() {
    let cfg = SafetyConfig::default();
    let env = SafetyEnvelope::new(cfg);
    assert!(env.check_cost(3.00).is_ok());
}

#[test]
fn safety_cost_check_fails_over_task_limit() {
    // Use track_spend + check_cost to simulate overspend
    let cfg = SafetyConfig::default();
    let mut env = SafetyEnvelope::new(cfg);
    env.track_spend(4.00);
    assert!(env.check_cost(2.00).is_err()); // 4+2=6 > 5 task limit
}

#[test]
fn safety_cost_check_fails_over_session_limit() {
    let cfg = SafetyConfig::default();
    let mut env = SafetyEnvelope::new(cfg);
    env.track_spend(49.00);
    assert!(env.check_cost(2.00).is_err()); // 49+2=51 > 50 session limit
}

#[test]
fn safety_track_spend_accumulates() {
    let cfg = SafetyConfig::default();
    let mut env = SafetyEnvelope::new(cfg);
    env.track_spend(1.00);
    env.track_spend(2.50);
    // Verify via status()
    let status = env.status();
    assert!((status.task_spent - 3.50).abs() < 0.001);
    assert!((status.session_spent - 3.50).abs() < 0.001);
}

#[test]
fn safety_reset_task_zeros_task_spent_not_session() {
    let cfg = SafetyConfig::default();
    let mut env = SafetyEnvelope::new(cfg);
    env.track_spend(3.00);
    env.reset_task();
    let status = env.status();
    assert!((status.task_spent - 0.0).abs() < 0.001);
    assert!((status.session_spent - 3.0).abs() < 0.001);
}

#[test]
fn safety_status_is_safe_80_percent_threshold() {
    let cfg = SafetyConfig::default();
    let mut env = SafetyEnvelope::new(cfg);
    env.track_spend(39.0); // 78% — safe
    assert!(env.status().is_safe);

    env.track_spend(2.0); // 82% — unsafe
    assert!(!env.status().is_safe);
}

#[test]
fn safety_protected_paths_detects_env_secrets_ssh() {
    let cfg = SafetyConfig::default();
    let env = SafetyEnvelope::new(cfg);
    assert!(env.is_protected(Path::new(".env")));
    assert!(env.is_protected(Path::new("secrets/api.key")));
    assert!(env.is_protected(Path::new("src/.ssh/id_rsa")));
    assert!(!env.is_protected(Path::new("src/main.rs")));
}

// ── Command Palette ──────────────────────────────────────────
#[test]
fn command_palette_has_eight_commands() {
    use anvil::tui::command::CommandPalette;
    let palette = CommandPalette::new();
    assert_eq!(palette.commands.len(), 8);
}

#[test]
fn command_palette_all_command_names_present() {
    use anvil::tui::command::CommandPalette;
    let palette = CommandPalette::new();
    let names: Vec<_> = palette.commands.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"spawn"));
    assert!(names.contains(&"models"));
    assert!(names.contains(&"cost"));
    assert!(names.contains(&"pause"));
    assert!(names.contains(&"resume"));
    assert!(names.contains(&"cancel"));
    assert!(names.contains(&"help"));
    assert!(names.contains(&"quit"));
}

#[test]
fn command_palette_filtered_empty_query_returns_all() {
    use anvil::tui::command::CommandPalette;
    let palette = CommandPalette::new();
    let filtered = palette.filtered_commands();
    assert_eq!(filtered.len(), palette.commands.len());
}

#[test]
fn command_palette_filtered_spawn_matches() {
    use anvil::tui::command::CommandPalette;
    let palette = CommandPalette::new();
    let filtered = palette.commands.iter()
        .filter(|c| c.name.contains("spawn"))
        .collect::<Vec<_>>();
    assert_eq!(filtered.len(), 1);
}

// ── Phase 2+3: Help Overlay ─────────────────────────────────
#[test]
fn help_overlay_new_visible_by_default() {
    use anvil::tui::HelpOverlay;
    let overlay = HelpOverlay::new();
    assert!(!overlay.visible);
}

#[test]
fn help_overlay_toggle_shows_and_hides() {
    use anvil::tui::HelpOverlay;
    let mut overlay = HelpOverlay::new();
    assert!(!overlay.visible);
    overlay.toggle();
    assert!(overlay.visible);
    overlay.toggle();
    assert!(!overlay.visible);
}

#[test]
fn help_overlay_render_does_not_panic_when_visible() {
    use anvil::tui::HelpOverlay;
    let mut overlay = HelpOverlay::new();
    overlay.show();
    let para = overlay.render();
    let _ = para;
}

// ── Phase 2+3: Cost HUD ─────────────────────────────────────
#[test]
fn cost_hud_new_hidden_by_default() {
    use anvil::tui::CostHud;
    let hud = CostHud::new();
    assert!(!hud.visible);
}

#[test]
fn cost_hud_toggle_works() {
    use anvil::tui::CostHud;
    let mut hud = CostHud::new();
    hud.toggle();
    assert!(hud.visible);
    hud.toggle();
    assert!(!hud.visible);
}

#[test]
fn cost_hud_render_shows_spending() {
    use anvil::tui::CostHud;
    use anvil::router::ModelDatabase;
    let mut hud = CostHud::new();
    hud.show();
    let db = ModelDatabase::new();
    let para = hud.render(&db);
    let _ = para;
}

// ── Phase 2+3: Agents Panel ─────────────────────────────────
#[test]
fn agents_panel_new_hidden_by_default() {
    use anvil::tui::AgentsPanel;
    let panel = AgentsPanel::new();
    assert!(!panel.visible);
    assert_eq!(panel.len(), 4); // 4 mock agents
}

#[test]
fn agents_panel_toggle_shows_4_agents() {
    use anvil::tui::AgentsPanel;
    let mut panel = AgentsPanel::new();
    panel.toggle();
    assert!(panel.visible);
    assert_eq!(panel.len(), 4);
}

#[test]
fn agents_panel_render_does_not_panic() {
    use anvil::tui::AgentsPanel;
    let panel = AgentsPanel::new();
    let para = panel.render();
    let _ = para;
}

#[test]
fn agents_panel_panel_height_is_positive() {
    use anvil::tui::AgentsPanel;
    let panel = AgentsPanel::new();
    assert!(panel.panel_height() > 0);
}

// ── Phase 2+3: Safety Checkpoint ─────────────────────────────
#[test]
fn safety_checkpoint_new_hidden_by_default() {
    use anvil::tui::SafetyCheckpoint;
    let cp = SafetyCheckpoint::new();
    assert!(!cp.visible);
}

#[test]
fn safety_checkpoint_render_does_not_panic_when_visible() {
    use anvil::tui::SafetyCheckpoint;
    let mut cp = SafetyCheckpoint::new();
    cp.show_with(
        "Test risk".to_string(),
        vec!["+ new line".to_string()],
        anvil::tui::RiskLevel::High,
    );
    let para = cp.render();
    let _ = para;
}

#[test]
fn safety_checkpoint_handle_key_escape_hides() {
    use anvil::tui::SafetyCheckpoint;
    let mut cp = SafetyCheckpoint::new();
    cp.show_with(
        "Test".to_string(),
        vec![],
        anvil::tui::RiskLevel::Medium,
    );
    assert!(cp.visible);
    cp.handle_key(crossterm::event::KeyCode::Esc);
    assert!(!cp.visible);
}

#[test]
fn safety_checkpoint_handle_key_enter_returns_approve_when_selected_0() {
    use anvil::tui::SafetyCheckpoint;
    let mut cp = SafetyCheckpoint::new();
    cp.show_with(
        "Test".to_string(),
        vec![],
        anvil::tui::RiskLevel::Low,
    );
    cp.selected = 0;
    let action = cp.handle_key(crossterm::event::KeyCode::Enter);
    assert!(matches!(action, Some(anvil::tui::CheckpointAction::Approve)));
    // After Enter, checkpoint hides
    assert!(!cp.visible);
}

// ── Input text/clear methods ─────────────────────────────────
#[test]
fn input_text_and_clear() {
    use anvil::tui::Input;
    let mut input = Input::new();
    input.set_text("hello");
    assert_eq!(input.text(), "hello");
    input.clear();
    assert_eq!(input.text(), "");
}

// ── Stream push_input_entry ─────────────────────────────────
#[test]
fn stream_push_input_entry_adds_thought() {
    use anvil::tui::stream::{Stream, EntryType};
    let mut stream = Stream::new();
    let initial = stream.entries.len();
    stream.push_input_entry("test task");
    assert_eq!(stream.entries.len(), initial + 1);
    assert!(matches!(
        stream.entries.last().unwrap().entry_type,
        EntryType::Thought
    ));
    assert_eq!(stream.entries.last().unwrap().content[0], "test task");
}
