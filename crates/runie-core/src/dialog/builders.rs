//! High-level panel builders for common dialog patterns.
//!
//! These builders replace custom `DialogState` variants with a unified
//! `Panel` + `PanelStack` API. Each builder returns a `PanelStack` ready
//! to be assigned to `AppState::open_dialog`.

use super::{ItemAction, Panel, PanelItem, PanelStack};
use crate::event::{ControlEvent, ModelConfigEvent};
use crate::settings::SettingValue;
use crate::Event;

// ============================================================================
// Command Palette
// ============================================================================

/// Build a command palette panel from structured command rows.
///
/// Mirrors [`settings`](fn@settings): the caller passes typed rows and the
/// builder turns them into panel items, avoiding stringly-typed labels.
pub fn command_palette(items: Vec<crate::commands::CommandRow>) -> PanelStack {
    // keep_open_on_activate is required for Android-like back-stack
    // navigation: selecting a command from the palette pushes the
    // palette onto the global back stack, so Esc on the sub-dialog
    // returns to the palette instead of closing the whole bar.
    let mut panel = Panel::new("palette", " Commands ")
        .with_filter()
        .keep_open();
    let mut last_category = String::new();
    for row in items {
        if row.category != last_category {
            if !last_category.is_empty() {
                panel = panel.separator();
            }
            panel = panel.header(row.category.clone());
            last_category = row.category;
        }
        panel = panel.command(row.name, row.desc, ItemAction::Emit(row.event));
    }
    PanelStack::new(panel)
}

// ============================================================================
// Model Selector
// ============================================================================

/// Build a model selector panel with provider-grouped items.
///
/// `groups` is a list of `(provider_name, [(model_name, event_to_emit_on_select)])`.
pub fn model_selector(
    recent: Vec<String>,
    groups: Vec<(String, Vec<(String, Event)>)>,
    current: &str,
) -> PanelStack {
    let mut panel = Panel::new("model", " Select Model ").with_filter();

    if !recent.is_empty() {
        panel = panel.header("Recent");
        for model in recent {
            let evt = ModelConfigEvent::SwitchModel {
                provider: model.split('/').next().unwrap_or("").into(),
                model: model.clone(),
            };
            let label = if model == current {
                format!("★ {}", model)
            } else {
                model
            };
            panel = panel.item(label, ItemAction::Emit(evt));
        }
        panel = panel.separator();
    }

    for (provider, models) in groups {
        panel = panel.header(provider);
        for (name, evt) in models {
            let label = if name == current {
                format!("★ {}", name)
            } else {
                name
            };
            panel = panel.item(label, ItemAction::Emit(evt));
        }
    }
    PanelStack::new(panel)
}

// ============================================================================
// Settings Dialog
// ============================================================================

/// One row in the settings dialog.
#[derive(Debug, Clone)]
pub struct SettingsRow {
    pub label: String,
    pub key: String,
    pub kind: SettingValue,
}

/// Build a settings dialog panel with category headers.
pub fn settings(categories: Vec<(String, Vec<SettingsRow>)>) -> PanelStack {
    let mut panel = Panel::new("settings", " Settings ");
    let _cat_count = categories.len();
    for (i, (cat_name, rows)) in categories.into_iter().enumerate() {
        if i > 0 {
            panel = panel.separator();
        }
        panel = panel.header(cat_name);
        for row in rows {
            let item = match row.kind {
                SettingValue::Bool(value) => PanelItem::Toggle {
                    label: row.label,
                    value,
                    action: ItemAction::Toggle(row.key.clone()),
                },
                SettingValue::Cycle { current, options } => PanelItem::Select {
                    label: row.label,
                    current,
                    options,
                    key: row.key,
                },
                SettingValue::Action(evt) => PanelItem::Action {
                    label: row.label,
                    action: ItemAction::Emit(evt),
                },
            };
            panel.items.push(item);
        }
    }
    PanelStack::new(panel)
}

// ============================================================================
// Scoped Models
// ============================================================================

/// Build a scoped models panel with provider-grouped toggle items.
pub fn scoped_models(
    models: Vec<(String, String, bool)>, // (provider, name, enabled)
) -> PanelStack {
    let mut panel = Panel::new("scoped", " Scoped Models ").keep_open();
    let mut last_provider = String::new();
    for (provider, name, enabled) in models {
        if provider != last_provider {
            if !last_provider.is_empty() {
                panel = panel.separator();
            }
            panel = panel.header(provider.clone());
            last_provider = provider;
        }
        // Emit a toggle event for each model — the state will mutate.
        let evt = ModelConfigEvent::ScopedModelToggle { name: name.clone() };
        let label = format!("{} {}", if enabled { "[x]" } else { "[ ]" }, name);
        panel = panel.item(label, ItemAction::Emit(evt));
    }
    PanelStack::new(panel)
}

// ============================================================================
// Session Tree
// ============================================================================

/// Build a session tree panel from a list of (depth, content, event).
pub fn session_tree(items: Vec<(usize, String, Event)>) -> PanelStack {
    let mut panel = Panel::new("session-tree", " Session Tree ").with_filter();
    if items.is_empty() {
        panel = panel.header("No session tree. Use /fork or /clone to create branches.");
    }
    for (_depth, content, evt) in items {
        let truncated: String = content.chars().take(50).collect();
        let label = if content.chars().count() > 50 {
            format!("{}…", truncated)
        } else {
            content
        };
        panel = panel.item(label, ItemAction::Emit(evt));
    }
    PanelStack::new(panel)
}

// ============================================================================
// Session List
// ============================================================================

/// A single session row for the session list dialog.
pub struct SessionRow {
    pub id: String,
    pub display_name: String,
    pub summary: Option<String>,
    pub message_count: usize,
    pub is_starred: bool,
    pub is_system: bool,
}

/// Build a session list panel with fuzzy search, star/unstar, rename, delete, and resume.
pub fn session_list(sessions: Vec<SessionRow>) -> PanelStack {
    if sessions.is_empty() {
        let panel = Panel::new("session-list", " Sessions ")
            .with_filter()
            .header("No sessions yet. Start a new conversation to create one.");
        return PanelStack::new(panel);
    }

    let mut panel = Panel::new("session-list", " Sessions ").with_filter().keep_open();
    panel = add_session_section(panel, &sessions, true, false, Some("System"));
    panel = add_session_section(panel, &sessions, false, true, Some("Starred"));
    let show_recent_header = sessions.iter().any(|s| s.is_system) || sessions.iter().any(|s| s.is_starred && !s.is_system);
    panel = add_session_section(panel, &sessions, false, false, if show_recent_header { Some("Recent") } else { None });

    PanelStack::new(panel)
}

fn add_session_section(panel: Panel, sessions: &[SessionRow], is_system: bool, is_starred: bool, header: Option<&str>) -> Panel {
    let mut panel = panel;
    let mut first = true;
    for session in sessions {
        let matches = if is_system {
            session.is_system
        } else if is_starred {
            session.is_starred && !session.is_system
        } else {
            !session.is_starred && !session.is_system
        };

        if matches {
            if first {
                if let Some(h) = header {
                    panel = panel.header(h);
                }
                first = false;
            }
            add_session_item(&mut panel, session);
        }
    }
    if !first && (is_system || is_starred) {
        panel = panel.separator();
    }
    panel
}

fn add_session_item(panel: &mut Panel, session: &SessionRow) {
    let star = if session.is_starred { "★" } else { "☆" };
    let count_label = format!("[{} msgs]", session.message_count);

    // Format: ☆ name [N msgs] — summary
    let label = if let Some(summary) = &session.summary {
        if summary.is_empty() {
            format!("{} {} {}", star, session.display_name, count_label)
        } else {
            format!("{} {} {} — {}", star, session.display_name, count_label, summary)
        }
    } else {
        format!("{} {} {}", star, session.display_name, count_label)
    };

    let id = session.id.clone();
    let evt = ControlEvent::SelectSession { id };
    panel.items.push(PanelItem::Action {
        label,
        action: ItemAction::Emit(evt),
    });
}

// ============================================================================
// Theme Picker (uses keep_open for live preview)
// ============================================================================

/// Build a theme picker panel that applies the theme on Enter without
/// closing the dialog (live preview).
pub fn theme_picker(themes: Vec<(String, Event)>) -> PanelStack {
    let mut panel = Panel::new("theme", " Choose Theme ").keep_open();
    panel = panel.header("available themes — press Enter to preview");
    for (name, evt) in themes {
        panel = panel.item(name, ItemAction::Emit(evt));
    }
    PanelStack::new(panel)
}

// ============================================================================
// File Picker (already filterable, just a wrapper for consistency)
// ============================================================================

/// Build an @-file picker panel.
pub fn file_picker(entries: Vec<(String, bool, Event)>) -> PanelStack {
    // (name, is_dir, event_to_emit)
    let mut panel = Panel::new("at-files", " Files ").with_filter();
    if entries.is_empty() {
        panel = panel.header("No files found");
    } else {
        panel = panel.header(format!("{} files", entries.len()));
    }
    for (name, is_dir, evt) in entries {
        let label = if is_dir { format!("{}/", name) } else { name };
        panel = panel.item(label, ItemAction::Emit(evt));
    }
    PanelStack::new(panel)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_evt() -> Event {
        ControlEvent::Abort
    }

    #[test]
    fn command_palette_builds() {
        use crate::commands::CommandRow;
        let stack = command_palette(vec![
            CommandRow::new("Commands", "/save", "Save session", dummy_evt()),
            CommandRow::new("Commands", "/load", "Load session", dummy_evt()),
            CommandRow::new("Skills", "test-skill", "A skill", dummy_evt()),
        ]);
        assert_eq!(stack.len(), 1);
        let panel = stack.current().unwrap();
        assert!(panel.filterable);
        assert!(panel.items.len() > 2);
        // Launcher: must keep_open_on_activate so Esc returns to palette.
        assert!(
            panel.keep_open_on_activate,
            "command palette must keep open on activation for back-stack navigation"
        );
        assert!(
            panel
                .items
                .iter()
                .any(|i| matches!(i, PanelItem::Command { .. })),
            "command palette items should be Command variants"
        );
    }

    #[test]
    fn model_selector_groups_by_provider() {
        let stack = model_selector(
            vec![],
            vec![
                ("openai".into(), vec![("gpt-4o".into(), dummy_evt())]),
                ("anthropic".into(), vec![("claude-3".into(), dummy_evt())]),
            ],
            "gpt-4o",
        );
        let panel = stack.current().unwrap();
        // Has headers for both providers
        let headers: Vec<_> = panel
            .items
            .iter()
            .filter(|i| matches!(i, PanelItem::Header(_)))
            .collect();
        assert_eq!(headers.len(), 2);
    }

    #[test]
    fn settings_builds_with_categories() {
        let stack = settings(vec![
            (
                "Models".into(),
                vec![SettingsRow {
                    label: "Provider".into(),
                    key: "provider".into(),
                    kind: SettingValue::Cycle {
                        current: "mock".into(),
                        options: vec!["mock".into(), "openai".into()],
                    },
                }],
            ),
            (
                "Appearance".into(),
                vec![SettingsRow {
                    label: "Theme".into(),
                    key: "theme".into(),
                    kind: SettingValue::Cycle {
                        current: "runie".into(),
                        options: vec!["runie".into(), "dracula".into()],
                    },
                }],
            ),
        ]);
        let panel = stack.current().unwrap();
        let headers: Vec<_> = panel
            .items
            .iter()
            .filter(|i| matches!(i, PanelItem::Header(_)))
            .collect();
        assert_eq!(headers.len(), 2);
    }

    #[test]
    fn scoped_models_groups_by_provider() {
        let stack = scoped_models(vec![
            ("openai".into(), "gpt-4o".into(), true),
            ("openai".into(), "gpt-4o-mini".into(), false),
            ("anthropic".into(), "claude-3".into(), true),
        ]);
        let panel = stack.current().unwrap();
        // 2 provider headers + 1 separator + 3 model items = 6
        assert_eq!(panel.items.len(), 6);
    }

    #[test]
    fn theme_picker_keeps_dialog_open() {
        let stack = theme_picker(vec![
            ("runie".into(), dummy_evt()),
            ("dracula".into(), dummy_evt()),
        ]);
        let panel = stack.current().unwrap();
        assert!(panel.keep_open_on_activate);
    }

    #[test]
    fn session_tree_handles_empty() {
        let stack = session_tree(vec![]);
        let panel = stack.current().unwrap();
        assert!(panel.filterable);
        // Has a header about no tree
        let has_header = panel
            .items
            .iter()
            .any(|i| matches!(i, PanelItem::Header(_)));
        assert!(has_header);
    }

    #[test]
    fn session_list_builds_with_sections() {
        let sessions = vec![
            SessionRow {
                id: "sys1".into(),
                display_name: "Scheduled Tasks".into(),
                summary: Some("Periodic automation".into()),
                message_count: 10,
                is_starred: false,
                is_system: true,
            },
            SessionRow {
                id: "star1".into(),
                display_name: "Important Chat".into(),
                summary: Some("Key discussion".into()),
                message_count: 5,
                is_starred: true,
                is_system: false,
            },
            SessionRow {
                id: "reg1".into(),
                display_name: "Regular Session".into(),
                summary: Some("Just a chat".into()),
                message_count: 3,
                is_starred: false,
                is_system: false,
            },
        ];

        let stack = session_list(sessions);
        let panel = stack.current().unwrap();
        assert!(panel.filterable);
        assert!(panel.keep_open_on_activate);

        // Has 3 headers: System, Starred, Recent
        let headers: Vec<_> = panel
            .items
            .iter()
            .filter(|i| matches!(i, PanelItem::Header(_)))
            .collect();
        assert_eq!(headers.len(), 3);

        // Has 3 session items (Action variants)
        let actions: Vec<_> = panel
            .items
            .iter()
            .filter(|i| matches!(i, PanelItem::Action { .. }))
            .collect();
        assert_eq!(actions.len(), 3);
    }

    #[test]
    fn session_list_empty_shows_message() {
        let stack = session_list(vec![]);
        let panel = stack.current().unwrap();
        assert!(panel.filterable);
        let has_header = panel
            .items
            .iter()
            .any(|i| matches!(i, PanelItem::Header(_)));
        assert!(has_header);
    }
}
