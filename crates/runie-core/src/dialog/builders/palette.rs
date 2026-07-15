//! Command palette and model selector builders.

use super::{ItemAction, Panel, PanelStack};
use crate::Event;

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
        panel = panel.command_with_aliases(
            row.name,
            row.desc,
            row.aliases,
            ItemAction::Emit(row.event),
        );
    }
    PanelStack::new(panel)
}

/// Build a model selector panel with provider-grouped items.
///
/// `groups` is a list of `(provider_name, [(model_name, event_to_emit_on_select)])`.
/// `thinking` holds per-model thinking-level overrides keyed `"provider/model"`;
/// models with an override show the level as a dim suffix on their row.
pub fn model_selector(
    recent: Vec<String>,
    groups: Vec<(String, Vec<(String, Event)>)>,
    current: &str,
    thinking: &std::collections::HashMap<String, crate::model::ThinkingLevel>,
) -> PanelStack {
    let mut panel = Panel::new("model", " Select Model ").with_filter();

    if !recent.is_empty() {
        panel = panel.header("Recent");
        for model in recent {
            let (provider, model_name) = model.split_once('/').unwrap_or(("", &model));
            let evt = Event::SelectModel {
                provider: provider.into(),
                model: model_name.into(),
            };
            let mut label = if model == current {
                format!("★ {}", model)
            } else {
                model.clone()
            };
            if let Some(level) = thinking.get(&model) {
                label.push_str(&format!("  · {}", level.as_str()));
            }
            panel = panel.item(label, ItemAction::Emit(evt));
        }
        panel = panel.separator();
    }

    for (provider, models) in groups {
        panel = panel.header(provider);
        for (name, evt) in models {
            let mut label = if name == current {
                format!("★ {}", name)
            } else {
                name.clone()
            };
            if let Some(level) = thinking.get(&name) {
                label.push_str(&format!("  · {}", level.as_str()));
            }
            panel = panel.item(label, ItemAction::Emit(evt));
        }
    }
    PanelStack::new(panel)
}

/// Build the per-model reasoning-level panel shown after picking a model in
/// the selector.
///
/// `global` is the global thinking level; `override_level` is the stored
/// per-model choice (if any). The effective choice — the override when set,
/// otherwise the "default" row — is marked `(current)`.
pub fn model_reasoning_panel(
    provider: &str,
    model: &str,
    global: crate::model::ThinkingLevel,
    override_level: Option<crate::model::ThinkingLevel>,
) -> PanelStack {
    let title = format!(" Reasoning · {provider}/{model} ");
    let mut panel = Panel::new("model-reasoning", title).header("Thinking level for this model");

    let default_label = if override_level.is_none() {
        format!("default (global: {}) (current)", global.as_str())
    } else {
        format!("default (global: {})", global.as_str())
    };
    panel = panel.item(
        default_label,
        ItemAction::Emit(Event::SwitchModelWithLevel {
            provider: provider.into(),
            model: model.into(),
            level: None,
        }),
    );

    for &level in crate::model::ThinkingLevel::all() {
        let label = if override_level == Some(level) {
            format!("{} (current)", level.as_str())
        } else {
            level.as_str().to_string()
        };
        panel = panel.item(
            label,
            ItemAction::Emit(Event::SwitchModelWithLevel {
                provider: provider.into(),
                model: model.into(),
                level: Some(level),
            }),
        );
    }
    // Pre-select the effective choice so accepting with Enter is a no-op:
    // the override row when set, otherwise the "default" row.
    let selected = match override_level {
        None => 0,
        Some(level) => {
            crate::model::ThinkingLevel::all()
                .iter()
                .position(|&l| l == level)
                .unwrap_or(0)
                + 1
        }
    };
    panel.selected = selected;
    PanelStack::new(panel)
}

/// Build the `/mode` interactive selector.
///
/// Shows all pattern choices in one panel: `single`, the three `swarm`
/// variants, and `improve`. The current choice is marked with `★`.
pub fn mode_selector(current: &str, current_variant: Option<&str>) -> PanelStack {
    let mut panel = Panel::new("mode", " Select Mode ").header("Agent orchestration pattern");

    panel = panel.item(
        mode_item_label("single", "Direct execution", current, None, current_variant),
        ItemAction::Emit(Event::SetMode {
            active: "single".into(),
            workers: None,
        }),
    );

    for (variant_name, desc) in [
        ("parallel", "Fan-out to all workers at once"),
        ("delegation", "Leader assigns tasks to specialists"),
        ("dag", "Workers form dependency waves"),
    ] {
        panel = panel.item(
            mode_item_label(
                &format!("swarm / {variant_name}"),
                desc,
                current,
                Some(variant_name),
                current_variant,
            ),
            ItemAction::Emit(Event::SetModeAndSwarmVariant {
                active: "swarm".into(),
                swarm_variant: variant_name.into(),
            }),
        );
    }

    panel = panel.item(
        mode_item_label(
            "improve",
            "Generate → review → revise loop",
            current,
            None,
            current_variant,
        ),
        ItemAction::Emit(Event::SetMode {
            active: "improve".into(),
            workers: None,
        }),
    );

    // Pre-select the current choice so pressing Enter without moving is a no-op
    // that still dismisses the dialog and re-applies the current mode.
    // `Panel.selected` is an index into navigable items, so skip headers/separators.
    let selected = panel
        .items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.is_navigable())
        .position(|(_, item)| match item {
            crate::dialog::PanelItem::Action { label, .. } => {
                label.starts_with("★ ") || label.starts_with("★")
            }
            _ => false,
        })
        .unwrap_or(0);
    panel.selected = selected;

    PanelStack::new(panel)
}

fn mode_item_label(
    name: &str,
    desc: &str,
    current: &str,
    variant: Option<&str>,
    current_variant: Option<&str>,
) -> String {
    let is_current = match variant {
        Some(v) => {
            current == "swarm"
                && (current_variant == Some(v)
                    || (current_variant.is_none() && v == "parallel"))
        }
        None => current == name,
    };
    if is_current {
        format!("★ {name} — {desc}")
    } else {
        format!("  {name} — {desc}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog::PanelItem;
    #[test]
    fn recent_model_event_splits_provider_and_model() {
        let stack = model_selector(
            vec!["minimax/M3".into()],
            vec![],
            "minimax/M3",
            &std::collections::HashMap::new(),
        );
        let panel = stack.current().unwrap();
        let action = panel
            .items
            .iter()
            .find_map(|i| match i {
                PanelItem::Action { action, .. } => Some(action.clone()),
                _ => None,
            })
            .expect("recent model should be an action item");

        let ItemAction::Emit(evt) = action else {
            panic!("expected emit action");
        };
        assert_eq!(
            evt,
            Event::SelectModel {
                provider: "minimax".into(),
                model: "M3".into(),
            },
        );
    }
}
