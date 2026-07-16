//! Status bar rendering — left ( ⠋ · Working… 1.2s ) and right ( ↑1.2k ↓4.8k 42/s 12k/128k 12% ⛀ )

use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::theme::{
    blend_color, color_accent, color_bg, pulse_brightness, style_status_idle, style_timestamp,
    GLYPH_PENDING,
};
use crate::ui::{estimate_element_tokens, hstack};
use runie_core::labels::format_elapsed_secs;
use runie_core::Snapshot;
use unicode_width::UnicodeWidthStr;

/// Render the status bar. The spinner comes from the snapshot (wall-clock
/// driven in core), so it animates at a steady cadence regardless of render
/// rate — the previous ThrobberState widget never advanced in production.
pub fn render(f: &mut Frame, snap: &Snapshot, area: Rect) {
    if !snap.has_models {
        return;
    }
    let right_text = format!("{} ", build_right_status(snap));
    let right_width = UnicodeWidthStr::width(right_text.as_str()) as u16;

    let h = hstack(area, &[Constraint::Min(0), Constraint::Length(right_width)]);

    render_left(f, snap, h[0]);
    f.render_widget(Paragraph::new(right_text).style(style_timestamp()), h[1]);
}

/// Render the left side of the status bar. The spinner frame is taken from
/// the snapshot and only shown while a turn is active; when idle the left
/// area shows only the git/folder status and badges.
///
/// When a permission request is pending (`is_pending_user_input`), a pulsing
/// diamond replaces the spinner — same cadence as Grok's drain-blocked and
/// plan-approval "your turn" indicators.
fn render_left(f: &mut Frame, snap: &Snapshot, area: Rect) {
    let text_parts = build_left_text_parts(snap);

    if !snap.turn_active {
        let left_text = text_parts.join(" · ");
        f.render_widget(Paragraph::new(left_text).style(style_status_idle()), area);
        return;
    }

    // Build the left status line using spans so the indicator glyph can be
    // colored independently (pulsing diamond when pending, spinner otherwise).
    let body = text_parts.join(" · ");
    let line = if snap.is_pending_user_input {
        // Pulsing diamond: blend accent toward bg using sin² pulse (grok parity).
        let pulse = pulse_brightness(snap.animation_frame, USER_WAITING_PULSE_SPEED);
        let color = blend_color(color_bg(), color_accent(), 0.3 + pulse * 0.7)
            .unwrap_or_else(color_accent);
        Line::from(vec![
            Span::styled(format!("{} · ", GLYPH_PENDING), Style::new().fg(color)),
            Span::styled(body, style_status_idle()),
        ])
    } else {
        Line::from(vec![
            Span::styled(format!("{} · ", snap.spinner_frame), style_status_idle()),
            Span::styled(body, style_status_idle()),
        ])
    };

    f.render_widget(Paragraph::new(line).style(style_status_idle()), area);
}

/// Pulse speed for every "waiting on you" diamond (grok parity).
/// `pulse_brightness` returns `sin²(tick*speed)` with period π, so at ~30fps
/// this gives a ~1.3s cycle (`π / (0.08 * 30) ≈ 1.31`).
const USER_WAITING_PULSE_SPEED: f32 = 0.08;

/// Build status bar text parts without the spinner char.
/// The spinner is rendered as a throbber widget overlay.
pub(crate) fn build_left_text_parts(snap: &Snapshot) -> Vec<String> {
    let mut parts = Vec::new();
    push_git_or_folder(&mut parts, snap);
    push_turn_status_text(&mut parts, snap);
    push_running_subagents(&mut parts, snap);
    push_thinking(&mut parts, snap);
    push_pending_edits(&mut parts, snap);
    push_read_only(&mut parts, snap);
    push_auto_mode(&mut parts, snap);
    parts
}

fn push_running_subagents(parts: &mut Vec<String>, snap: &Snapshot) {
    let count = snap
        .pattern_workers
        .iter()
        .filter(|w| w.status == runie_core::model::PatternWorkerStatus::Running)
        .count();
    if snap.turn_active && count > 0 {
        // Grok-style subagent spinner frames (match the tasks pane).
        let frames = [':', '\u{2e2c}', '\u{22c5}'];
        let idx = runie_core::labels::BRAILLE_SIX
            .iter()
            .position(|&c| c == snap.spinner_frame)
            .unwrap_or(0);
        let glyph = frames[idx % frames.len()];
        parts.push(format!("{} {}", glyph, count));
    }
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

/// Build the "Working…" status text (spinner char comes from the snapshot).
fn push_turn_status_text(parts: &mut Vec<String>, snap: &Snapshot) {
    if !snap.turn_active {
        return;
    }
    let text = if let Some(elapsed) = snap.turn_elapsed_secs {
        format!("Working… {}", format_elapsed_secs(elapsed))
    } else {
        "Working…".to_owned()
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

fn push_auto_mode(parts: &mut Vec<String>, snap: &Snapshot) {
    if snap.auto_mode {
        parts.push("⚡ Auto".to_owned());
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
        assert_eq!(context_window_for("google", "gemini-3.5-flash"), 1_000_000);
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
        assert_eq!(
            context_window_for("unknown", "model"),
            DEFAULT_CONTEXT_WINDOW
        );
    }

    #[test]
    fn status_bar_shows_auto_badge_when_enabled() {
        let snap = runie_core::Snapshot {
            auto_mode: true,
            ..Default::default()
        };
        let left = super::build_left_text(&snap);
        assert!(
            left.contains("⚡ Auto"),
            "left text should contain the auto badge: {left}"
        );
    }

    #[test]
    fn status_bar_hides_auto_badge_when_disabled() {
        let snap = runie_core::Snapshot::default();
        let left = super::build_left_text(&snap);
        assert!(
            !left.contains("⚡ Auto"),
            "left text should not contain the auto badge: {left}"
        );
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
