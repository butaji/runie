//! Status bar rendering — left ( git/folder · Working... ) and right ( ↑1.2k ↓4.8k 42/s 12%/128k ○ )

use ratatui::{
    layout::{Constraint, Rect},
    widgets::Paragraph,
    Frame,
};
use runie_core::Snapshot;

use crate::theme::{style_status_idle, style_timestamp};
use crate::ui::{estimate_element_tokens, hstack};

pub fn render(f: &mut Frame, snap: &Snapshot, area: Rect) {
    let left_text = format!(" {}", build_left_text(snap));
    let right_text = format!("{} ", build_right_status(snap));
    let right_width = display_width(&right_text) as u16;

    let h = hstack(area, &[Constraint::Min(0), Constraint::Length(right_width)]);

    f.render_widget(Paragraph::new(left_text).style(style_status_idle()), h[0]);
    f.render_widget(Paragraph::new(right_text).style(style_timestamp()), h[1]);
}

/// Count display columns (char count = width for our single-width glyphs).
fn display_width(s: &str) -> usize {
    s.chars().count()
}

pub(crate) fn build_left_text(snap: &Snapshot) -> String {
    let mut parts = Vec::new();
    // When idle, show git repo/branch or current folder name
    if !snap.turn_active {
        let git_or_folder = snap
            .git_info
            .as_ref()
            .map(|g| g.format_right(&snap.cwd_name))
            .unwrap_or_else(|| format!("{}/", snap.cwd_name));
        parts.push(git_or_folder);
    }
    if snap.turn_active {
        let mut text = if let Some(elapsed) = snap.turn_elapsed_secs {
            runie_core::labels::action_text(snap.spinner_frame, "Working", elapsed)
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
    parts.join(" · ")
}

// =============================================================================
// Right side: token throughput + context usage chess piece
// =============================================================================

/// Get chess piece for context usage percentage.
/// 0-25% ⛀ | 26-50% ⛁ | 51-75% ⛂ | 76-100% ⛃
pub(crate) fn context_piece(percent: usize) -> char {
    match percent {
        0..=25 => '⛀',
        26..=50 => '⛁',
        51..=75 => '⛂',
        _ => '⛃',
    }
}

pub(crate) fn build_right_status(snap: &Snapshot) -> String {
    let usage = context_usage(snap);
    let piece = context_piece(usage.percent);
    let limit = usage.limit_k();

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
            usage.percent,
            limit,
            piece
        )
    } else {
        format!("{}%/{} {}", usage.percent, limit, piece)
    }
}

/// Format a possibly-animated (floating point) token count for display.
fn format_k_animated(n: f64) -> String {
    let n = n.round().max(0.0);
    if n >= 1_000.0 {
        format!("{:.1}k", n / 1_000.0)
    } else {
        (n as usize).to_string()
    }
}

pub(crate) struct ContextUsage {
    #[allow(dead_code)]
    pub(crate) used: usize,
    pub(crate) limit: usize,
    pub(crate) percent: usize,
}

pub(crate) fn context_usage(snap: &Snapshot) -> ContextUsage {
    let limit = context_window_for(&snap.provider, &snap.model);
    let used: usize = snap
        .elements
        .iter()
        .filter(|e| {
            matches!(
                e,
                runie_core::Element::UserMessage { .. } | runie_core::Element::AgentMessage { .. }
            )
        })
        .map(estimate_element_tokens)
        .sum();
    let percent = used
        .checked_mul(100)
        .and_then(|x| x.checked_div(limit))
        .unwrap_or(0)
        .min(100);
    ContextUsage {
        used,
        limit,
        percent,
    }
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

const DEFAULT_CONTEXT_WINDOW: usize = 128_000;

pub(crate) fn context_window_for(provider: &str, model: &str) -> usize {
    runie_core::provider_registry::find_provider(provider)
        .and_then(|p| p.models.iter().find(|m| m.name == model))
        .and_then(|m| m.context_window)
        .unwrap_or(DEFAULT_CONTEXT_WINDOW)
}

#[cfg(test)]
mod tests {
    use super::context_window_for;

    #[test]
    fn status_bar_context_window_matches_registry() {
        assert_eq!(context_window_for("openai", "gpt-4o"), 128_000);
        assert_eq!(
            context_window_for("anthropic", "claude-sonnet-4-6"),
            200_000
        );
        assert_eq!(context_window_for("google", "gemini-2.5-pro"), 1_000_000);
    }

    #[test]
    fn status_bar_context_window_falls_back_to_default() {
        assert_eq!(
            context_window_for("unknown", "model"),
            super::DEFAULT_CONTEXT_WINDOW
        );
    }
}
