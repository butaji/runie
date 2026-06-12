//! High-level panel builders for common dialog patterns.
//!
//! These builders replace custom `DialogState` variants with a unified
//! `Panel` + `PanelStack` API. Each builder returns a `PanelStack` ready
//! to be assigned to `AppState::open_dialog`.

use super::{ItemAction, Panel, PanelItem, PanelStack};
use crate::Event;

// ============================================================================
// Command Palette
// ============================================================================

/// Build a command palette panel.
///
/// `items` is a list of `(category, label, event_to_emit)`.
pub fn command_palette(items: Vec<(String, String, Event)>) -> PanelStack {
    // keep_open_on_activate is required for Android-like back-stack
    // navigation: selecting a command from the palette pushes the
    // palette onto the global back stack, so Esc on the sub-dialog
    // returns to the palette instead of closing the whole bar.
    let mut panel = Panel::new("palette", " Commands ")
        .with_filter()
        .keep_open();
    let mut last_category = String::new();
    for (category, label, evt) in items {
        if category != last_category {
            if !last_category.is_empty() {
                panel = panel.separator();
            }
            panel = panel.header(category.clone());
            last_category = category;
        }
        panel = panel.item(label, ItemAction::Emit(evt));
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
            let evt = Event::SwitchModel {
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
    pub kind: SettingsRowKind,
}

#[derive(Debug, Clone)]
pub enum SettingsRowKind {
    Bool(bool),
    Cycle {
        current: String,
        options: Vec<String>,
    },
    Action(Event),
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
                SettingsRowKind::Bool(value) => PanelItem::Toggle {
                    label: row.label,
                    value,
                    action: ItemAction::Toggle(row.key.clone()),
                },
                SettingsRowKind::Cycle {
                    current, options, ..
                } => PanelItem::Select {
                    label: row.label,
                    current,
                    options,
                    key: row.key,
                },
                SettingsRowKind::Action(evt) => PanelItem::Action {
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
        let evt = Event::ScopedModelToggle { name: name.clone() };
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
        Event::Abort
    }

    #[test]
    fn command_palette_builds() {
        let stack = command_palette(vec![
            ("Commands".into(), "/save".into(), dummy_evt()),
            ("Commands".into(), "/load".into(), dummy_evt()),
            ("Skills".into(), "test-skill".into(), dummy_evt()),
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
                    kind: SettingsRowKind::Cycle {
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
                    kind: SettingsRowKind::Cycle {
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
}
