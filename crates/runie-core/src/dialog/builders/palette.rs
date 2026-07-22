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
        panel = panel.command_with_aliases(row.name, row.desc, row.aliases, ItemAction::Emit(row.event));
    }
    PanelStack::new(panel)
}

/// Build a model selector panel with provider-grouped items.
///
/// `groups` is a list of `(provider_name, [(model_name, event_to_emit_on_select)])`.
/// Build a model label with optional star, thinking-level suffix, and
/// orchestration role tag (lead / worker).
fn model_label(
    full_name: &str,
    current: &str,
    thinking: &std::collections::HashMap<String, crate::model::ThinkingLevel>,
    lead_model: Option<&str>,
    worker_model: Option<&str>,
) -> String {
    let (provider, model_name) = full_name.split_once('/').unwrap_or(("", full_name));
    let is_current = full_name == current;
    #[allow(clippy::unnecessary_map_or)]
    let is_lead = lead_model.map_or(false, |m| m == full_name);
    #[allow(clippy::unnecessary_map_or)]
    let is_worker = worker_model.map_or(false, |m| m == full_name);

    let mut label = if is_current {
        format!("★ {}/{}", provider, model_name)
    } else {
        full_name.to_string()
    };

    // Orchestration role tag — shown after the star so it reads clearly.
    if is_lead {
        label.push_str("  [lead]");
    } else if is_worker {
        label.push_str("  [worker]");
    }

    if let Some(level) = thinking.get(full_name) {
        label.push_str(&format!("  · {}", level.as_str()));
    }

    label
}

/// `thinking` holds per-model thinking-level overrides keyed `"provider/model"`;
/// models with an override show the level as a dim suffix on their row.
/// `lead_model` and `worker_model`, when set, append a role tag to the
/// matching entry so the user can see which model is assigned in the
/// current orchestration config.
pub fn model_selector(
    recent: Vec<String>,
    groups: Vec<(String, Vec<(String, Event)>)>,
    current: &str,
    thinking: &std::collections::HashMap<String, crate::model::ThinkingLevel>,
    role: Option<&str>,
    lead_model: Option<String>,
    worker_model: Option<String>,
) -> PanelStack {
    let title = if let Some(r) = role {
        format!(" Select {} Model ", r.to_uppercase())
    } else {
        " Select Model ".into()
    };
    let mut panel = Panel::new("model", title).with_filter();
    let lead = lead_model.as_deref();
    let worker = worker_model.as_deref();

    if !recent.is_empty() {
        panel = panel.header("Recent");
        for model in recent {
            let evt = Event::SelectModel {
                provider: model
                    .split_once('/')
                    .map(|p| p.0.to_string())
                    .unwrap_or_default(),
                model: model
                    .split_once('/')
                    .map(|m| m.1.to_string())
                    .unwrap_or_else(|| model.clone()),
            };
            let label = model_label(&model, current, thinking, lead, worker);
            panel = panel.item(label, ItemAction::Emit(evt));
        }
        panel = panel.separator();
    }

    for (provider, models) in groups {
        panel = panel.header(provider.clone());
        for (name, evt) in models {
            let full_name = format!("{}/{}", provider, name);
            let label = model_label(&full_name, current, thinking, lead, worker);
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
#[allow(clippy::too_many_lines)]
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
        ItemAction::Emit(Event::SwitchModelWithLevel { provider: provider.into(), model: model.into(), level: None }),
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
/// When `current` is `"swarm"`, additional rows show the configured
/// lead (coordinator) and worker (task-executor) models, if any.
#[allow(clippy::too_many_lines)]
pub fn mode_selector(
    current: &str,
    current_variant: Option<&str>,
    lead_model: Option<String>,
    worker_model: Option<String>,
) -> PanelStack {
    let mut panel = Panel::new("mode", " Select Mode ").header("Agent orchestration pattern");

    panel = panel.item(
        mode_item_label("single", "Direct execution", current, None, current_variant),
        ItemAction::Emit(Event::SetMode { active: "single".into(), workers: None }),
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
        ItemAction::Emit(Event::SetMode { active: "improve".into(), workers: None }),
    );

    // ── Lead / Worker model configuration (shown in swarm mode) ─────────────────
    let is_swarm = current == "swarm";
    if is_swarm {
        panel = panel.separator();
        panel = panel.header("Swarm Models");

        let lead_display = lead_model.unwrap_or("(uses current)".to_string());
        panel = panel.item(
            format!("  Lead Model: {lead_display}"),
            ItemAction::Emit(Event::RunPaletteCommand { name: "select-lead-model".into(), args: "".into() }),
        );

        let worker_display = worker_model.unwrap_or("(uses current)".to_string());
        panel = panel.item(
            format!("  Worker Model: {worker_display}"),
            ItemAction::Emit(Event::RunPaletteCommand { name: "select-worker-model".into(), args: "".into() }),
        );
    }

    // Pre-select the current choice so pressing Enter without moving is a no-op
    // that still dismisses the dialog and re-applies the current mode.
    // `Panel.selected` is an index into navigable items, so skip headers/separators.
    let selected = panel
        .items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.is_navigable())
        .position(|(_, item)| match item {
            crate::dialog::PanelItem::Action { label, .. } => label.starts_with("★ ") || label.starts_with("★"),
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
        Some(v) => current == "swarm" && (current_variant == Some(v) || (current_variant.is_none() && v == "parallel")),
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
            None,
            None,
            None,
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
            Event::SelectModel { provider: "minimax".into(), model: "M3".into() },
        );
    }
}
