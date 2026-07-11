//! Status bar rendering — left ( git/folder · Working... ) and right ( ↑1.2k ↓4.8k 42/s 12k/128k 12% ⛀ )

use ratatui::{
    layout::{Constraint, Rect},
    widgets::Paragraph,
    Frame,
};
use throbber_widgets_tui::{symbols::throbber::BRAILLE_SIX, Throbber, ThrobberState};

use crate::theme::{style_status_idle, style_timestamp};
use crate::ui::{estimate_element_tokens, hstack};
use runie_core::Snapshot;
use unicode_width::UnicodeWidthStr;

/// Render the status bar. The spinner is rendered as a throbber widget overlay
/// at the start of the left area, driven by the provided `ThrobberState`.
pub fn render(f: &mut Frame, snap: &Snapshot, area: Rect, throbber: &mut ThrobberState) {
    if !snap.has_models {
        return;
    }
    let right_text = format!("{} ", build_right_status(snap));
    let right_width = UnicodeWidthStr::width(right_text.as_str()) as u16;

    let h = hstack(area, &[Constraint::Min(0), Constraint::Length(right_width)]);

    render_left_with_throbber(f, snap, h[0], throbber);
    f.render_widget(Paragraph::new(right_text).style(style_timestamp()), h[1]);
}

/// Render the left side of the status bar. The spinner uses the Throbber
/// widget directly from throbber-widgets-tui, replacing the hand-rolled
/// symbol-overlay approach. The spinner is only shown while a turn is active;
/// when idle the left area shows only the git/folder status and badges.
fn render_left_with_throbber(
    f: &mut Frame,
    snap: &Snapshot,
    area: Rect,
    throbber: &mut ThrobberState,
) {
    // Build text parts.
    let text_parts = build_left_text_parts(snap);

    if !snap.turn_active {
        // Idle: no spinner, just the status text.
        let left_text = text_parts.join(" · ");
        f.render_widget(
            Paragraph::new(left_text).style(style_status_idle()),
            area,
        );
        return;
    }

    // Split area: [spinner][status_text]
    // Spinner takes 2 cells (symbol + trailing space), rest is text.
    let spinner_width = 2;
    let (spinner_area, text_area) = {
        let splits = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([Constraint::Length(spinner_width), Constraint::Min(0)])
            .split(area);
        (splits[0], splits[1])
    };

    // Render the throbber widget directly (no manual symbol extraction).
    let throbber_widget = Throbber::default()
        .throbber_set(BRAILLE_SIX)
        .style(style_status_idle());
    f.render_stateful_widget(throbber_widget, spinner_area, throbber);

    // Render status text after the spinner.
    let left_text = format!(" · {}", text_parts.join(" · "));
    f.render_widget(
        Paragraph::new(left_text).style(style_status_idle()),
        text_area,
    );
}

/// Build status bar text parts without the spinner char.
/// The spinner is rendered as a throbber widget overlay.
pub(crate) fn build_left_text_parts(snap: &Snapshot) -> Vec<String> {
    let mut parts = Vec::new();
    push_git_or_folder(&mut parts, snap);
    push_turn_status_text(&mut parts, snap);
    push_thinking(&mut parts, snap);
    push_pending_edits(&mut parts, snap);
    push_read_only(&mut parts, snap);
    parts
}

/// Build the left status bar text as a joined string (without the spinner char).
/// Used by tests that only need the text content.
#[cfg(test)]
pub(crate) fn build_left_text(snap: &Snapshot) -> String {
    build_left_text_parts(snap).join(" · ")
}

fn push_git_or_folder(parts: &mut Vec<String>, snap: &Snapshot) {
    if snap.turn_active {
        return;
    }
    let git_or_folder = snap
        .git_info
        .as_ref()
        .map(|g| g.format_right(&snap.cwd_name))
        .unwrap_or_else(|| format!("{}/", snap.cwd_name));
    parts.push(git_or_folder);
}

/// Build the "Working..." status text without the spinner char (throbber overlays it).
fn push_turn_status_text(parts: &mut Vec<String>, snap: &Snapshot) {
    if !snap.turn_active {
        return;
    }
    let text = if let Some(elapsed) = snap.turn_elapsed_secs {
        // No spinner char — it's rendered by the throbber overlay.
        if "Working".ends_with("ing") {
            format!("Working... {:.1}s", elapsed)
        } else {
            format!("Working {:.1}s", elapsed)
        }
    } else {
        "Working...".to_owned()
    };
    let mut full = text;
    if snap.queue_count > 0 {
        full.push_str(&format!(" ({} queued)", snap.queue_count));
    }
    parts.push(full);
}

fn push_thinking(parts: &mut Vec<String>, snap: &Snapshot) {
    if snap.thinking_level == runie_core::model::ThinkingLevel::Off {
        return;
    }
    parts.push(format!("Think: {}", snap.thinking_level.as_str()));
}

fn push_pending_edits(parts: &mut Vec<String>, snap: &Snapshot) {
    if snap.pending_edits.is_empty() {
        return;
    }
    parts.push(format!("{} pending", snap.pending_edits.len()));
}

fn push_read_only(parts: &mut Vec<String>, snap: &Snapshot) {
    if snap.read_only {
        parts.push("🔒 RO".to_owned());
    }
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
            "-".to_owned()
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
        let used_k = format_k(usage.used);
        format!("{}/{} {}% {}", used_k, limit, usage.percent, piece)
    }
}

/// Format a possibly-animated (floating point) token count for display.
fn format_k(n: usize) -> String {
    if n >= 1_000 {
        format!("{}k", n / 1_000)
    } else {
        n.to_string()
    }
}

fn format_k_animated(n: f64) -> String {
    let n = n.round().max(0.0);
    if n >= 1_000.0 {
        format!("{:.1}k", n / 1_000.0)
    } else {
        (n as usize).to_string()
    }
}

pub(crate) struct ContextUsage {
    pub(crate) used: usize,
    pub(crate) limit: usize,
    pub(crate) percent: usize,
}

pub(crate) fn context_usage(snap: &Snapshot) -> ContextUsage {
    let limit = runie_core::model_catalog::context_window_for(&snap.provider, &snap.model);
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

#[cfg(test)]
mod tests {
    use runie_core::model_catalog::{context_window_for, DEFAULT_CONTEXT_WINDOW};

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
    fn status_bar_context_window_minimax() {
        assert_eq!(context_window_for("minimax", "MiniMax-M2.7"), 256_000);
        assert_eq!(context_window_for("minimax", "MiniMax-M3"), 256_000);
        // "MiniMax-M2" is not in the registry -> shared 128k default.
        assert_eq!(context_window_for("minimax", "MiniMax-M2"), 128_000);
    }

    #[test]
    fn status_bar_context_window_falls_back_to_default() {
        assert_eq!(context_window_for("unknown", "model"), DEFAULT_CONTEXT_WINDOW);
    }

    #[test]
    fn status_bar_shows_worktree_label() {
        let snap = runie_core::Snapshot {
            git_info: Some(runie_core::snapshot::GitInfo {
                repo_name: Some("runie".to_string()),
                branch: Some("main".to_string()),
                is_worktree: true,
                worktree_source: Some("/Users/admin/Code/GitHub/runie".to_string()),
            }),
            cwd_name: "agent-impl".to_string(),
            ..Default::default()
        };
        let left = super::build_left_text(&snap);
        assert!(
            left.contains("worktree"),
            "left text should contain worktree: {left}"
        );
    }
}
