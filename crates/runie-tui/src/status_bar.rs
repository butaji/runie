//! Status bar rendering — left ( Working... ) and right ( ↑1.2k ↓4.8k 42/s 12%/128k ○ )

use ratatui::{
    layout::{Constraint, Rect},
    widgets::Paragraph,
    Frame,
};
use runie_core::Snapshot;

use crate::theme::{style_status_idle, style_timestamp};
use crate::ui::{hstack, estimate_element_tokens};

pub fn render(f: &mut Frame, snap: &Snapshot, area: Rect) {
    let left_text = format!(" {}", build_left_text(snap));
    let right_text = format!("{} ", build_right_status(snap));
    let right_width = display_width(&right_text) as u16;

    let h = hstack(area, &[
        Constraint::Min(0),
        Constraint::Length(right_width),
    ]);

    f.render_widget(Paragraph::new(left_text).style(style_status_idle()), h[0]);
    f.render_widget(Paragraph::new(right_text).style(style_timestamp()), h[1]);
}

/// Count display columns (char count = width for our single-width glyphs).
fn display_width(s: &str) -> usize {
    s.chars().count()
}

fn build_left_text(snap: &Snapshot) -> String {
    let mut parts = Vec::new();
    if snap.turn_active {
        let mut text = if let Some(elapsed) = snap.turn_elapsed_secs {
            runie_core::labels::action_text(
                snap.spinner_frame, "Working", elapsed,
            )
        } else {
            format!("{} Working...", snap.spinner_frame)
        };
        if snap.queue_count > 0 {
            text.push_str(&format!(" ({} queued)", snap.queue_count));
        }
        parts.push(text);
    }
    if snap.thinking_level != runie_core::model::ThinkingLevel::Off {
        parts.push(format!("Think: {}", snap.thinking_level.as_str()));
    }
    if !snap.pending_edits.is_empty() {
        parts.push(format!("{} pending", snap.pending_edits.len()));
    }
    if snap.read_only {
        parts.push("🔒 RO".to_string());
    }
    if !snap.auth_providers.is_empty() {
        parts.push(format!("🔑 {}", snap.auth_providers.join(", ")));
    }
    parts.join(" · ")
}

// =============================================================================
// Right side: context usage + radial bar
// =============================================================================

pub(crate) fn build_right_status(snap: &Snapshot) -> String {
    let ctx = context_usage(snap);
    let bar = radial_bar(ctx.percent);

    if snap.turn_active {
        let speed = if snap.speed_tps >= 1.0 {
            format!("{:.0}", snap.speed_tps)
        } else if snap.speed_tps > 0.0 {
            format!("{:.1}", snap.speed_tps)
        } else {
            "-".to_string()
        };
        // Use animated display values for smooth transitions
        let tokens_in_display = snap.tokens_in_display;
        let tokens_out_display = snap.tokens_out_display;
        format!(
            "↑{} ↓{} {}/s {}%/{} {}",
            format_k_animated(tokens_in_display),
            format_k_animated(tokens_out_display),
            speed,
            ctx.percent,
            ctx.limit_k(),
            bar
        )
    } else {
        let git_or_folder = snap.git_info.as_ref()
            .map(|g| g.format_right(&snap.cwd_name))
            .unwrap_or_else(|| format!("{}/", snap.cwd_name));
        format!("{} {}%/{} {}", git_or_folder, ctx.percent, ctx.limit_k(), bar)
    }
}

fn format_k(n: usize) -> String {
    if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// Format a possibly-animated (floating point) token count for display.
fn format_k_animated(n: f64) -> String {
    if n >= 1_000.0 {
        format!("{:.1}k", n / 1_000.0)
    } else {
        // Show integer part only, removing decimal noise during animation
        n.round().max(0.0) as usize as f64;
        let rounded = n.round() as usize;
        if rounded >= 1_000 {
            format!("{:.1}k", rounded as f64 / 1_000.0)
        } else {
            rounded.to_string()
        }
    }
}

pub(crate) fn radial_bar(percent: usize) -> char {
    match percent {
        0..=12 => '○',
        13..=37 => '◔',
        38..=62 => '◑',
        63..=87 => '◕',
        _ => '●',
    }
}

pub(crate) struct ContextUsage {
    pub(crate) used: usize,
    pub(crate) limit: usize,
    pub(crate) percent: usize,
}

pub(crate) fn context_usage(snap: &Snapshot) -> ContextUsage {
    let limit = context_window_for(&snap.provider, &snap.model);
    let used: usize = snap.elements.iter()
        .filter(|e| matches!(e,
            runie_core::Element::UserMessage { .. }
            | runie_core::Element::AgentMessage { .. }
        ))
        .map(estimate_element_tokens)
        .sum();
    let percent = if limit > 0 {
        (used * 100 / limit).min(100)
    } else {
        0
    };
    ContextUsage { used, limit, percent }
}

impl ContextUsage {
    pub(crate) fn limit_k(&self) -> String {
        if self.limit >= 1_000_000 {
            format!("{}M", self.limit / 1_000_000)
        } else if self.limit >= 1_000 {
            format!("{}k", self.limit / 1_000)
        } else {
            format!("{}", self.limit)
        }
    }
}

pub(crate) fn context_window_for(provider: &str, model: &str) -> usize {
    match provider {
        "openai" => match model {
            "o1" | "o3" | "o4-mini" => 200_000,
            _ => 128_000,
        },
        "anthropic" => 200_000,
        "google" => 1_000_000,
        "deepseek" => 64_000,
        "mistral" => 128_000,
        "groq" => 128_000,
        "xai" => 128_000,
        "together" => 128_000,
        "fireworks" => 128_000,
        "openrouter" => 128_000,
        "moonshotai" | "kimi-coding" => 256_000,
        "zai" => 128_000,
        "minimax" => 256_000,
        "xiaomi" => 128_000,
        "opencode" => 128_000,
        "azure-openai-responses" => 128_000,
        "amazon-bedrock" => 200_000,
        "cerebras" => 128_000,
        "github-copilot" => 128_000,
        "huggingface" => 128_000,
        "nvidia" => 128_000,
        "ollama" => 128_000,
        _ => 128_000,
    }
}
