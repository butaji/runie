//! Status bar rendering — left ( ⠋ · Working… 1.2s ) and right ( ↑1.2k ↓4.8k 42/s 12k/128k 12% ⛀ )

use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::theme::{
    blend_color, color_bg, color_bg_panel, color_error, color_fg, color_fg_mid, color_warning, monitor_glyph,
    style_status_idle, GLYPH_MONITOR_FRAMES, GLYPH_PENDING, MONITOR_PULSE_DIVISOR,
};
use crate::ui::{estimate_element_tokens, hstack, progress_bar_spans};
use runie_core::Snapshot;
use unicode_width::UnicodeWidthStr;

/// Render the context header row (cwd left, token usage right).
///
/// Grok parity: top row above the feed showing current directory and context
/// window usage. Updates live as context grows.
/// Example: `/private/tmp …                              18k/500k 3%`
pub fn render_context_header(f: &mut Frame, snap: &Snapshot, area: Rect) {
    if !snap.has_models || area.width < 10 || area.height == 0 {
        return;
    }
    let usage = context_usage(snap);
    let limit = usage.limit_k();
    let used_k = format_k(usage.used);

    // Left: cwd (truncated with … if needed)
    let cwd = &snap.cwd_name;
    let left = format!("{}", cwd);

    // Right: token usage
    let right = format!("{}/{} {}%", used_k, limit, usage.percent);

    let line = Line::from(vec![
        Span::styled(left, style_status_idle()),
        Span::raw(" "),
        Span::styled(right, style_status_idle()),
    ]);
    f.render_widget(Paragraph::new(line).style(style_status_idle()), area);
}
pub fn render(f: &mut Frame, snap: &Snapshot, area: Rect) {
    if !snap.has_models || area.width < 10 || area.height == 0 {
        return;
    }
    // Right side is capped at half the width so the activity label always has
    // room to truncate (grok parity: only the label truncates).
    let right_spans = build_right_spans(snap);
    let right_text = right_spans
        .iter()
        .map(|s| s.content.as_ref())
        .collect::<String>();
    let right_width = UnicodeWidthStr::width(right_text.as_str()) as u16;
    let capped = right_width.min(area.width / 2);

    let h = hstack(area, &[Constraint::Min(0), Constraint::Length(capped)]);

    render_left(f, snap, h[0]);
    f.render_widget(
        Paragraph::new(Line::from(right_spans)).style(style_status_idle()),
        h[1],
    );
}

/// Render the left side of the status bar. The spinner frame is taken from
/// the snapshot and only shown while a turn is active; when idle the left
/// area shows only the git/folder status and badges.
///
/// When a permission request is pending (`is_pending_user_input`), a pulsing
/// diamond replaces the spinner — same cadence as Grok's drain-blocked and
/// plan-approval "your turn" indicators.
#[allow(clippy::too_many_lines)]
fn render_left(f: &mut Frame, snap: &Snapshot, area: Rect) {
    let idle = style_status_idle();

    if !snap.turn_active {
        let text_parts = build_left_text_parts(snap);
        if text_parts.is_empty() {
            return;
        }
        let left_text = text_parts
            .into_iter()
            .map(|s| s.content.clone())
            .collect::<Vec<_>>()
            .join(" · ");
        let line = Line::from(left_text);
        f.render_widget(Paragraph::new(line).style(idle), area);
        return;
    }

    // Build the left status line using spans so the indicator glyph can be
    // colored independently (pulsing diamond when pending, spinner otherwise).
    let text_parts = build_left_text_parts(snap);

    let body_str = if text_parts.is_empty() {
        String::new()
    } else {
        // Join ALL parts — the activity label (`Thinking… 0.4s`, `Running
        // {tool}…`, `Cancelling…`) is a normal body span, not skipped.
        text_parts
            .into_iter()
            .map(|s| s.content.clone())
            .collect::<Vec<_>>()
            .join(" · ")
    };

    let line = if snap.is_pending_user_input {
        // Pulsing diamond: blend accent toward bg using sin² pulse (grok parity).
        let spinner = Span::styled(format!("{} ", GLYPH_PENDING), idle);
        let body_span = if body_str.is_empty() {
            vec![spinner]
        } else {
            vec![spinner, Span::styled(body_str, idle)]
        };
        Line::from(body_span)
    } else {
        let spinner = Span::styled(format!("{} ", snap.spinner_frame), idle);
        let body_span = if body_str.is_empty() {
            vec![spinner]
        } else {
            vec![spinner, Span::styled(body_str, idle)]
        };
        Line::from(body_span)
    };

    f.render_widget(Paragraph::new(line).style(idle), area);
}

/// Build status bar text parts (spans) without the spinner char.
/// The spinner is rendered as a colored glyph in `render_left`.
/// Returns `Vec<Span>` so individual parts can carry their own style
/// (e.g. the activity label "Running {tool}…" is green).
pub(crate) fn build_left_text_parts(snap: &Snapshot) -> Vec<Span<'static>> {
    let idle = style_status_idle();
    let mut parts: Vec<Span<'static>> = Vec::new();

    if let Some(part) = push_git_or_folder(snap) {
        parts.push(part);
    }
    if let Some(part) = push_turn_status_text(snap, idle) {
        parts.push(part);
    }
    if let Some(part) = push_running_subagents(snap) {
        parts.push(part);
    }
    if let Some(part) = push_watching_label(snap, idle) {
        parts.push(part);
    }
    if let Some(part) = push_thinking(snap, idle) {
        parts.push(part);
    }
    if let Some(part) = push_pending_edits(snap) {
        parts.push(part);
    }
    if let Some(part) = push_read_only(snap) {
        parts.push(part);
    }
    if let Some(part) = push_auto_mode(snap) {
        parts.push(part);
    }
    if let Some(part) = push_mcp_status(snap) {
        parts.push(part);
    }
    if let Some(part) = push_circuit_breaker(snap) {
        parts.push(part);
    }
    parts
}

/// Build the left status bar text as a joined string (without the spinner char).
/// Used by tests that only need the text content.
#[cfg(test)]
pub(crate) fn build_left_text(snap: &Snapshot) -> String {
    build_left_text_parts(snap)
        .into_iter()
        .map(|s| s.content.clone())
        .collect::<Vec<_>>()
        .join(" · ")
}

fn push_running_subagents(snap: &Snapshot) -> Option<Span<'static>> {
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
        Some(Span::raw(format!("{} {}", glyph, count)))
    } else {
        None
    }
}

fn push_git_or_folder(snap: &Snapshot) -> Option<Span<'static>> {
    if snap.turn_active {
        return None;
    }
    let git_or_folder = snap
        .git_info
        .as_ref()
        .map(|g| g.format_right(&snap.cwd_name))
        .unwrap_or_else(|| format!("{}/", snap.cwd_name));
    Some(Span::raw(git_or_folder))
}

/// Build the "○ ◉ watching · N workers" label for idle pattern workers.
/// Shows when the agent is idle but background workers are still running (grok parity).
fn push_watching_label(snap: &Snapshot, idle: Style) -> Option<Span<'static>> {
    // Only show when idle and workers exist
    if snap.turn_active {
        return None;
    }

    let running = snap
        .pattern_workers
        .iter()
        .filter(|w| w.status == runie_core::model::PatternWorkerStatus::Running)
        .count();

    if running == 0 {
        return None;
    }

    // Get the animated monitor glyph frame
    let frame_idx = ((snap.animation_frame / MONITOR_PULSE_DIVISOR) as usize) % GLYPH_MONITOR_FRAMES.len();
    let monitor_glyph_str = monitor_glyph(frame_idx);

    // Render as: "○ ◉ watching · N workers"
    let noun = if running == 1 { "worker" } else { "workers" };
    Some(Span::styled(
        format!("{} watching · {} {noun}", monitor_glyph_str, running),
        idle,
    ))
}

/// Build the activity label driven by `snap.turn_activity` (grok parity).
///
/// - `ToolRunning`: `Running {tool}…` in full accent_success green, followed
///   by a gray phase timer (`Running list_dir… 0.2s`).
/// - `Thinking…` / `Responding…` in text_secondary.
/// - `Cancelling…` in accent_error (stop button is hidden meanwhile).
/// - Fallback `Working…` with the turn elapsed timer.
fn push_turn_status_text(snap: &Snapshot, idle: Style) -> Option<Span<'static>> {
    use runie_core::snapshot::TurnActivityKind;
    if !snap.turn_active {
        return None;
    }
    let phase = snap
        .activity_elapsed_secs
        .map(std::time::Duration::from_secs_f64)
        .map(runie_core::labels::format_turn_timer)
        .unwrap_or_else(|| "0.0s".to_owned());

    let (label, activity_style) = match snap.turn_activity {
        TurnActivityKind::ToolRunning => {
            let tool = snap
                .current_tool_name
                .clone()
                .unwrap_or_else(|| "tool".to_owned());
            (
                format!("Running {tool}…"),
                idle.fg(crate::theme::color_success()),
            )
        }
        TurnActivityKind::Cancelling => (
            "Cancelling…".to_owned(),
            idle.fg(crate::theme::color_error()),
        ),
        TurnActivityKind::Thinking => ("Thinking…".to_owned(), idle),
        TurnActivityKind::Responding => ("Responding…".to_owned(), idle),
        TurnActivityKind::Working => ("Working…".to_owned(), idle),
    };

    let mut full = format!("{} {}", label, phase);
    if snap.queue_count > 0 {
        full.push_str(&format!(" ({} queued)", snap.queue_count));
    }
    Some(Span::styled(full, activity_style))
}

fn push_thinking(snap: &Snapshot, idle: Style) -> Option<Span<'static>> {
    if snap.thinking_level == runie_core::model::ThinkingLevel::Off {
        return None;
    }
    Some(Span::styled(
        format!("Think: {}", snap.thinking_level.as_str()),
        idle,
    ))
}

fn push_pending_edits(snap: &Snapshot) -> Option<Span<'static>> {
    if snap.pending_edits.is_empty() {
        return None;
    }
    Some(Span::raw(format!("{} pending", snap.pending_edits.len())))
}

fn push_read_only(snap: &Snapshot) -> Option<Span<'static>> {
    if snap.read_only {
        Some(Span::raw("🔒 RO"))
    } else {
        None
    }
}

fn push_auto_mode(snap: &Snapshot) -> Option<Span<'static>> {
    if snap.auto_mode {
        Some(Span::raw("⚡ Auto"))
    } else {
        None
    }
}

/// MCP server status indicator: shows the count of configured MCP servers
/// (e.g. `⌘ 2 mcp`), styled dim. Hidden when none are configured.
fn push_mcp_status(snap: &Snapshot) -> Option<Span<'static>> {
    let count = snap.mcp_servers.len();
    if count == 0 {
        return None;
    }
    let noun = if count == 1 {
        "mcp server"
    } else {
        "mcp servers"
    };
    Some(Span::styled(
        format!("⌘ {count} {noun}"),
        style_status_idle().fg(crate::theme::color_dim()),
    ))
}

/// Build the circuit breaker status indicator for the status bar.
/// Shows "⚡ CB: N" when the circuit breaker has tripped, where N is the threshold.
fn push_circuit_breaker(snap: &Snapshot) -> Option<Span<'static>> {
    if snap.circuit_breaker_tripped {
        Some(Span::styled(
            format!("⚡ CB: {}", snap.circuit_breaker_threshold),
            style_status_idle(),
        ))
    } else {
        None
    }
}

// =============================================================================
// Right side: token throughput + context usage chess piece
// =============================================================================

pub(crate) fn build_right_status(snap: &Snapshot) -> String {
    build_right_spans(snap)
        .into_iter()
        .map(|s| s.content.to_string())
        .collect()
}

/// Get coin-stack glyph for context usage percentage.
pub(crate) fn context_piece(percent: usize) -> char {
    match percent {
        0..=25 => '⛀',
        26..=50 => '⛁',
        51..=75 => '⛂',
        _ => '⛃',
    }
}

/// Build the right side of the status bar as styled spans (grok parity).
///
/// Turn-active arm: `{format_turn_timer} ⇣{format_tokens_short(tokens)} [stop]`
/// — the ` ⇣…` token arm is omitted entirely when the turn has received zero
/// tokens, and `[stop]` is hidden while the turn is cancelling. Idle arm keeps
/// the context gauge (`12K / 128K 3% ⛀`), swapping to a progress bar +
/// percentage when context detail is pinned (`/context-detail`).
pub(crate) fn build_right_spans(snap: &Snapshot) -> Vec<Span<'static>> {
    let idle_style = style_status_idle();
    if !snap.turn_active {
        if let Some(spans) = build_context_item(snap, snap.context_detail_pinned) {
            return spans;
        }
        // No context data (limit unknown): fall back to the plain gauge.
        let usage = context_usage(snap);
        let limit = usage.limit_k();
        let used_k = format_k(usage.used);
        return vec![Span::styled(
            format!(
                "{}/{} {}% {}",
                used_k,
                limit,
                usage.percent,
                context_piece(usage.percent)
            ),
            idle_style,
        )];
    }

    let timer_style = Style::new().fg(crate::theme::color_turn_timer());
    let timer = snap
        .turn_elapsed_secs
        .map(std::time::Duration::from_secs_f64)
        .map(runie_core::labels::format_turn_timer)
        .unwrap_or_else(|| "0.0s".to_owned());

    let mut spans: Vec<Span<'static>> = Vec::new();
    spans.push(Span::styled(timer, timer_style));

    // Token arm: ` ⇣{Nk}` — only when the current turn has received tokens.
    if snap.current_turn_tokens > 0 {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            format!(
                "⇣{}",
                runie_core::labels::format_tokens_short(snap.current_turn_tokens)
            ),
            timer_style,
        ));
    }

    // `[stop]` — literal text, gray at rest (accent_error red on future hover);
    // hidden while cancelling and never rendered on keyboard-only hosts.
    if !snap.turn_cancelling {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            "[stop]",
            Style::new().fg(crate::theme::color_stop_button()),
        ));
    }
    spans
}

/// Format a possibly-animated (floating point) token count for display.
fn format_k(n: usize) -> String {
    if n >= 1_000 {
        format!("{}k", n / 1_000)
    } else {
        n.to_string()
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

// =============================================================================
// Context detail item (grok parity: context_bar.rs + progress_bar.rs)
// =============================================================================

/// Gap between the bar and the percentage, plus the fixed percentage width.
/// `" "` + `"XX.X%"` (5 cols) = 6 columns; the default text is right-padded
/// to at least this width so toggling never shifts neighboring status items.
const BAR_PCT_GAP: u16 = 1;
const PCT_WIDTH: u16 = 5;

/// Format a token count like Grok's `fmt_tokens` (always ≤4 chars):
/// `0`–`999` raw, `1.2K`, `12K`, `999K`, `1.2M`, `12M`.
fn fmt_tokens(n: u64) -> String {
    if n < 1_000 {
        format!("{n}")
    } else if n < 10_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else if n < 1_000_000 {
        format!("{}K", n / 1_000)
    } else if n < 10_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else {
        format!("{}M", n / 1_000_000)
    }
}

/// Format a percentage exactly 5 chars wide: `0.00%`, `5.12%`, `20.1%`,
/// `99.9%`, or `MAX %` (anything that would round to ≥100.0% clamps).
fn fmt_pct5(pct: f64) -> String {
    if pct >= 100.0 {
        return "MAX %".to_string();
    }
    let s = if pct < 10.0 {
        format!("{pct:.2}%")
    } else {
        format!("{pct:.1}%")
    };
    if s.len() <= 5 {
        s
    } else {
        "MAX %".to_string()
    }
}

/// Usage percentage as f64 (limit>0 guard; 0 when the limit is unknown).
fn context_usage_pct_f64(usage: &ContextUsage) -> f64 {
    if usage.limit == 0 {
        0.0
    } else {
        usage.used as f64 / usage.limit as f64 * 100.0
    }
}

/// Usage-urgency gradient color (grok parity breakpoints):
/// 0% → primary text, 50% → muted text, 75% → warning, 95% → error.
/// Between breakpoints the color is lerped in RGB.
fn context_gradient_color(pct: f64) -> Color {
    let bps: [(f64, Color); 4] =
        [(0.0, color_fg()), (50.0, color_fg_mid()), (75.0, color_warning()), (95.0, color_error())];
    let pct = pct.clamp(0.0, 100.0);
    for (i, (bp_pct, bp_color)) in bps.iter().enumerate() {
        if pct <= *bp_pct {
            return *bp_color;
        }
        if let Some((next_pct, next_color)) = bps.get(i + 1) {
            if pct <= *next_pct {
                let t = ((pct - bp_pct) / (next_pct - bp_pct)) as f32;
                return blend_color(*bp_color, *next_color, t).unwrap_or(*bp_color);
            }
        }
    }
    color_error()
}

/// Build the idle right-side context item.
///
/// - `expanded = false` (default): `{fmt_tokens(used)} / {fmt_tokens(limit)}`
///   in the urgency gradient, right-padded to ≥6 columns.
/// - `expanded = true` (pinned): a `bar_width`-cell 1/8-block progress bar +
///   ` ` + `fmt_pct5(pct)` + a chess-piece indicator; the bar width reserves
///   the percentage gap and the two fixed suffix columns so the expanded line
///   is exactly as wide as the default line (width invariant).
/// - Returns `None` when the limit is unknown (item omitted, as in Grok).
pub(crate) fn build_context_item(snap: &Snapshot, expanded: bool) -> Option<Vec<Span<'static>>> {
    let usage = context_usage(snap);
    if usage.limit == 0 {
        return None;
    }
    let pct = context_usage_pct_f64(&usage);
    let gradient = context_gradient_color(pct);

    let text = format!(
        "{}/{} {}% {}",
        fmt_tokens(usage.used as u64).to_lowercase(),
        fmt_tokens(usage.limit as u64).to_lowercase(),
        usage.percent,
        context_piece(usage.percent)
    );
    let natural_width = UnicodeWidthStr::width(text.as_str()) as u16;
    const EXPANDED_SUFFIX_WIDTH: u16 = 2;
    let total_width = natural_width.max(BAR_PCT_GAP + PCT_WIDTH + EXPANDED_SUFFIX_WIDTH);

    if !expanded {
        let padded = format!("{text:<width$}", width = total_width as usize);
        return Some(vec![Span::styled(padded, style_status_idle())]);
    }

    let bar_width = total_width - (BAR_PCT_GAP + PCT_WIDTH + EXPANDED_SUFFIX_WIDTH);
    let track = color_bg_panel();
    let mut spans = progress_bar_spans(bar_width, (pct / 100.0) as f32, gradient, track);
    spans.push(Span::styled(" ", Style::new().bg(color_bg())));
    spans.push(Span::styled(fmt_pct5(pct), Style::new().fg(color_fg_mid())));
    spans.push(Span::raw(" "));
    spans.push(Span::raw(context_piece(usage.percent).to_string()));
    Some(spans)
}

#[cfg(test)]
mod tests {
    use runie_core::model_catalog::{context_window_for, DEFAULT_CONTEXT_WINDOW};
    use unicode_width::UnicodeWidthStr;

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
        let snap = runie_core::Snapshot { auto_mode: true, ..Default::default() };
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
    fn status_bar_renders_idle_context_once() {
        let snap = runie_core::Snapshot { cwd_name: "runie".to_string(), ..Default::default() };
        let left = super::build_left_text(&snap);
        assert_eq!(
            left.matches("runie/").count(),
            1,
            "idle context must not be duplicated: {left}"
        );
    }

    #[test]
    fn status_bar_restores_coin_stack_context_indicator() {
        let snap = runie_core::Snapshot::default();
        let rendered = super::build_right_status(&snap);
        assert!(
            rendered.contains('⛀'),
            "idle context should include the coin-stack indicator: {rendered}"
        );
        assert_eq!(super::context_piece(25), '⛀');
        assert_eq!(super::context_piece(100), '⛃');
    }

    #[test]
    fn status_bar_colors_tool_activity_with_success_theme_token() {
        let snap = runie_core::Snapshot {
            turn_active: true,
            turn_activity: runie_core::snapshot::TurnActivityKind::ToolRunning,
            current_tool_name: Some("list".to_string()),
            ..Default::default()
        };
        let parts = super::build_left_text_parts(&snap);
        let activity = parts
            .iter()
            .find(|span| span.content.contains("Running list"))
            .expect("activity span");
        assert_eq!(activity.style.fg, Some(crate::theme::color_success()));
    }

    #[test]
    fn status_bar_colors_cancelling_with_error_theme_token() {
        let snap = runie_core::Snapshot {
            turn_active: true,
            turn_activity: runie_core::snapshot::TurnActivityKind::Cancelling,
            ..Default::default()
        };
        let parts = super::build_left_text_parts(&snap);
        let activity = parts
            .iter()
            .find(|span| span.content.contains("Cancelling"))
            .expect("activity span");
        assert_eq!(activity.style.fg, Some(crate::theme::color_error()));
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

    #[test]
    fn status_bar_shows_mcp_count_when_configured() {
        let snap = runie_core::Snapshot {
            mcp_servers: vec![runie_core::dialog::builders::McpServerRow {
                name: "filesystem".to_string(),
                transport: "stdio".to_string(),
                connected: false,
                tool_count: 0,
            }]
            .into(),
            ..Default::default()
        };
        let left = super::build_left_text(&snap);
        assert!(
            left.contains("1 mcp server"),
            "left text should contain MCP count: {left}"
        );
    }

    #[test]
    fn status_bar_hides_mcp_when_none_configured() {
        let snap = runie_core::Snapshot::default();
        let left = super::build_left_text(&snap);
        assert!(
            !left.contains("mcp"),
            "left text should not contain MCP indicator: {left}"
        );
    }

    // ── fmt_tokens / fmt_pct5 (grok context_bar.rs ports) ────────────────

    #[test]
    fn fmt_tokens_ports_grok_breakpoints() {
        let f = super::fmt_tokens;
        assert_eq!(f(0), "0");
        assert_eq!(f(12), "12");
        assert_eq!(f(999), "999");
        assert_eq!(f(1_200), "1.2K");
        assert_eq!(f(9_960), "10.0K"); // rounds up within the 1K–9.9K band
        assert_eq!(f(12_000), "12K");
        assert_eq!(f(999_000), "999K");
        assert_eq!(f(1_200_000), "1.2M");
        assert_eq!(f(12_000_000), "12M");
        assert_eq!(f(123_000_000), "123M");
    }

    #[test]
    fn fmt_tokens_always_five_chars_or_less() {
        // Grok's band formats can reach 5 chars at the rounding boundary
        // (e.g. 9_999 → "10.0K"), matching the spec's own 9_960 → "10.0K" example.
        let f = super::fmt_tokens;
        for n in [0u64, 999, 1_000, 9_999, 10_000, 999_999, 1_000_000, 9_999_999, 10_000_000, 100_000_000] {
            assert!(
                f(n).len() <= 5,
                "fmt_tokens({n}) = '{}' exceeds 5 chars",
                f(n)
            );
        }
    }

    #[test]
    fn fmt_pct5_always_five_chars() {
        let f = super::fmt_pct5;
        assert_eq!(f(0.0), "0.00%");
        assert_eq!(f(5.12), "5.12%");
        assert_eq!(f(20.1), "20.1%");
        assert_eq!(f(99.9), "99.9%");
        assert_eq!(f(100.0), "MAX %");
        assert_eq!(f(150.0), "MAX %");
        for pct in [0.0, 0.5, 9.99, 10.0, 42.0, 99.99, 100.0, 250.0] {
            assert_eq!(
                f(pct).len(),
                5,
                "fmt_pct5({pct}) = '{}' not 5 chars",
                f(pct)
            );
        }
    }

    // ── context detail width invariant ───────────────────────────────────

    #[test]
    fn context_item_width_invariant_across_toggle() {
        let snap = runie_core::Snapshot { ..Default::default() };

        let default_wide = super::build_context_item(&snap, false)
            .expect("context item should render")
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<String>();
        let expanded_wide = super::build_context_item(&snap, true)
            .expect("context item should render")
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<String>();
        // Minimum width reserves the percentage and fixed indicator suffix.
        assert!(
            UnicodeWidthStr::width(default_wide.as_str()) >= 8,
            "default form must be padded to >= 6 cols; got '{default_wide}'"
        );
        assert_eq!(
            UnicodeWidthStr::width(default_wide.as_str()),
            UnicodeWidthStr::width(expanded_wide.as_str()),
            "expanded and default forms must have equal width (no layout shift); default={default_wide:?} expanded={expanded_wide:?}"
        );
    }
}
