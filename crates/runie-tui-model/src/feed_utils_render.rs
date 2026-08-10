use super::super::*;

/// Render the elapsed-time suffix for completed background work. `None`
/// resolves to an empty fragment so the host string stays identical to the
/// pre-elapsed form.
pub fn format_elapsed(elapsed_ms: Option<u64>) -> String {
    elapsed_ms
        .map(|millis| format!(" in {:.1}s", millis as f64 / 1_000.0))
        .unwrap_or_default()
}

/// Render the trailing error fragment for background-work completions. The
/// suffix is suppressed when the work did not error, so success messages
/// stay identical regardless of whether an error payload is present.
pub fn format_error(is_error: bool, error: Option<&str>) -> String {
    if is_error {
        error.map(|value| format!(" ({value})")).unwrap_or_default()
    } else {
        String::new()
    }
}

/// Fallback thinking-window duration used when the status actor never
/// observed a reasoning turn. Pinned here so the "Thought for X.Xs" line
/// stays renderer-independent and reproducible across replay paths.
pub const DEFAULT_THINKING_ELAPSED_MS: u64 = 900;

/// Render the Grok "Thought for …" summary line. `None` resolves to the
/// pinned [`DEFAULT_THINKING_ELAPSED_MS`] so callers can rely on a stable
/// label regardless of whether the status actor observed a reasoning turn.
pub fn thinking_summary(elapsed_ms: Option<u64>) -> String {
    let elapsed_ms = elapsed_ms.unwrap_or(DEFAULT_THINKING_ELAPSED_MS);
    format!("◆ Thought for {:.1}s", elapsed_ms as f64 / 1_000.0)
}

/// Animation frames for the running tool bullet. The first three characters
/// are non-breaking whitespace followed by a single trailing space so the
/// bullet occupies the same terminal width as Grok's source-backed default
/// prefix; the fourth frame is a Braille dot-cluster for the same width.
pub const RUNNING_BULLETS: [&str; 4] = ["⋅ ", ": ", "⸬ ", "⁙ "];

/// Render the running tool bullet for a given animation frame. Centralized
/// here so the actor-owned animation frame and any replay path share one
/// vocabulary; the frame index wraps via modular arithmetic.
pub fn running_bullet(frame: usize) -> &'static str {
    RUNNING_BULLETS[frame % RUNNING_BULLETS.len()]
}

/// Detect CommonMark fenced code blocks in assistant text. The recognized
/// opening fence is three backticks after the renderer prefix (`┃ `) so the
/// Grok transcript parses a code block opened in the same line that the
/// renderer already prefixed. Centralized here so the markdown classifier
/// stays renderer-independent and reproducible across replay paths.
pub fn is_fence(text: &str) -> bool {
    text.trim_start()
        .strip_prefix("┃ ")
        .unwrap_or(text)
        .starts_with("```")
}

/// Detect a Grok-flavored table row. A row starts and ends with `|` and
/// contains at least two `|` separators so the renderer can split a header
/// from a body row without ambiguity.
pub fn is_table_row(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.matches('|').count() >= 2
}

/// Detect the separator row beneath a Grok table header. The cells must be
/// non-empty and contain only `-`, `:`, or whitespace; this matches the
/// `<cells>` slice shown after `is_table_row` for a header line.
pub fn is_table_separator(text: &str) -> bool {
    text.trim()
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .all(|cell| !cell.is_empty() && cell.chars().all(|ch| matches!(ch, '-' | ':' | ' ')))
}

/// Extract the heading title from a CommonMark ATX heading, returning only
/// the body text after the leading `#` run and one optional space. Levels
/// are clamped to `1..=6` to match the CommonMark specification.
pub fn atx_heading(text: &str) -> Option<&str> {
    let hashes = text.chars().take_while(|ch| *ch == '#').count();
    (1..=6)
        .contains(&hashes)
        .then(|| text.get(hashes..)?.strip_prefix(' '))
        .flatten()
}

/// Render the Grok bottom border row that closes a markdown table. The
/// column widths are derived from the cell characters plus two padding
/// cells on each side, matching the renderer's existing border shape.
pub fn table_bottom_border(text: &str) -> String {
    let widths = text
        .trim()
        .trim_matches('|')
        .split('|')
        .map(|cell| "─".repeat(cell.trim().chars().count() + 2))
        .collect::<Vec<_>>();
    format!("└{}┘", widths.join("┴"))
}

/// Append a wrapped line of text to the row buffer, splitting at the
/// given `width` so the renderer can project the result onto a wider
/// terminal geometry. The `code` flag lets callers mark the row as a
/// formatted code block (`true`) or normal text (`false`). Centralized
/// here so the actor-owned text projection and the renderer share one
/// wrapping rule.
pub fn append_wrapped(
    rows: &mut Vec<(LineKind, String, bool)>,
    kind: LineKind,
    text: String,
    code: bool,
    width: usize,
) {
    if width == 0 || text.chars().count() <= width {
        rows.push((kind, text, code));
        return;
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        let head: String = chars.drain(..width.min(chars.len())).collect();
        rows.push((kind, head, code));
    }
}

/// Append word-wrapped text to the row buffer. Whitespace acts as the
/// break point; the leading whitespace of the source line is preserved
/// on each emitted row so the projected widget keeps its original
/// indentation.
pub fn append_wrapped_words(
    rows: &mut Vec<(LineKind, String, bool)>,
    kind: LineKind,
    text: String,
    width: usize,
) {
    let leading: String = text.chars().take_while(|ch| ch.is_whitespace()).collect();
    let mut line = leading.clone();
    for word in text.split_whitespace() {
        let candidate = if line.trim().is_empty() {
            word.to_owned()
        } else {
            format!("{line} {word}")
        };
        if !line.trim().is_empty() && candidate.chars().count() > width {
            rows.push((kind, std::mem::replace(&mut line, leading.clone()), false));
        }
        if line.trim().is_empty() {
            line.push_str(word);
        } else {
            line.push(' ');
            line.push_str(word);
        }
    }
    if !line.is_empty() {
        rows.push((kind, line, false));
    }
}

/// Position variant for the Grok welcome surface version badge. The full
/// badge is the long `v0.1.0 · Beta` label, the hero footer appears as
/// the right-aligned footer on the wide hero, and the inline variant is
/// the compact `v0.1.0` form used in compact widgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionBadgeVariant {
    Full,
    HeroFooter,
    HeroInline,
}

/// Render the welcome version badge for the given variant. Centralized
/// here so the actor-owned welcome payload and the renderer agree on the
/// exact `runie v{version} · Beta` shapes.
pub fn version_badge(variant: VersionBadgeVariant) -> String {
    let version = env!("CARGO_PKG_VERSION");
    match variant {
        VersionBadgeVariant::Full => format!("runie v{version} · Beta"),
        VersionBadgeVariant::HeroFooter => format!("runie Beta · v{version}"),
        VersionBadgeVariant::HeroInline => format!("runie v{version}"),
    }
}

/// Whether a submitted prompt text is an immediate quit command. The
/// trim/lowercase normalization matches the Grok-style `exit` / `quit`
/// / `:q` vocabulary so the keymap and any replay path share one
/// definition.
pub fn is_quit_command(text: &str) -> bool {
    matches!(
        text.trim().to_ascii_lowercase().as_str(),
        "exit" | "quit" | ":q"
    )
}

/// Render the welcome modal chrome as a sequence of `LineKind::System`
/// rows. Centralized here so the actor-owned welcome payload and the
/// renderer share the same idle chrome projection; the `env!` macro
/// resolves to the workspace version at compile time.
pub fn welcome_modal_lines() -> Vec<Line> {
    let version = env!("CARGO_PKG_VERSION");
    vec![
        Line::new(LineKind::System, format!("╭─ Runie  v{version} ─")),
        Line::new(LineKind::System, String::from("│ main runie")),
        Line::new(LineKind::System, String::from("│ Model · runie-core")),
        Line::new(LineKind::System, String::from("│ /help for commands")),
        Line::new(LineKind::System, String::from("╰─")),
        Line::new(LineKind::System, String::from("◆ session_start")),
    ]
}

/// Wrapping scrollback messages that bracket the `◆ session_start`
/// marker. Centralized here so the actor-owned session-start projection
/// and the renderer share the same `[hooks: 1]` count and the
/// surrounding separator rows.
pub fn session_start_messages() -> Vec<ScrollbackMsg> {
    vec![
        ScrollbackMsg::Append(Line::new(LineKind::Separator, "")),
        ScrollbackMsg::Append(Line::new(
            LineKind::SessionStart,
            "◆ session_start  [hooks: 1]",
        )),
        ScrollbackMsg::Append(Line::new(LineKind::Separator, "")),
    ]
}

/// Render a user prompt row with a right-aligned timestamp gutter. The
/// Grok transcript reserves `PROMPT_TIMESTAMP_WRAP_GUTTER` columns for
/// the timestamp before deciding where the prompt wraps, then the
/// timestamp is right-aligned to the feed's terminal edge. Centralized
/// here so the actor-owned user-prompt projection and the renderer
/// share one wrap rule.
pub fn append_user_with_timestamp(
    rows: &mut Vec<(LineKind, String, bool)>,
    text: String,
    timestamp: &str,
    width: usize,
) {
    // Grok reserves a timestamp gutter when deciding where long prompts wrap,
    // then right-aligns the timestamp to the feed's terminal edge.
    let timestamp_width = timestamp.chars().count();
    const PROMPT_TIMESTAMP_WRAP_GUTTER: usize = 8;
    let first_width = width.saturating_sub(timestamp_width + PROMPT_TIMESTAMP_WRAP_GUTTER);
    let mut chars: Vec<char> = text.chars().collect();
    let mut split = first_width.min(chars.len());
    while split > 0 && split < chars.len() && !chars[split].is_whitespace() {
        split -= 1;
    }
    let first: String = chars.drain(..split).collect();
    const TIMESTAMP_EDGE_OFFSET: usize = 2;
    let padding = width
        .saturating_sub(first.chars().count() + timestamp_width)
        .saturating_sub(TIMESTAMP_EDGE_OFFSET);
    rows.push((
        LineKind::User,
        format!("{first}{blank}{timestamp}", blank = " ".repeat(padding)),
        false,
    ));
    let indent = " ".repeat(USER_PREFIX_INDENT);
    let rest: String = chars.into_iter().collect();
    append_wrapped_words(
        rows,
        LineKind::User,
        format!("{indent}{}", rest.trim_start()),
        first_width,
    );
}

/// Minimum unix-timestamp value (seconds) treated as a live prompt timestamp.
/// Values below this are either absent or fixtures; values at or above are
/// rendered with the short clock format. Centralized here so the renderer
/// and any replay path share one threshold.
pub const PROMPT_TIMESTAMP_LIVE_THRESHOLD: i64 = 1_000_000_000;

/// Number of columns the Grok user-prompt prefix occupies (`   ❯ ` —
/// three spaces, the `❯` glyph, and one trailing space). Centralized
/// here so the actor-owned user-prompt wrap helper and the renderer
/// share one indent width.
pub const USER_PREFIX_INDENT: usize = 5;

/// Strip an absolute `workspace` prefix from a tool-supplied path so
/// the rendered header shows a workspace-relative path. The relative
/// path is normalized to a single leading separator and the empty
/// case collapses to `.` so the renderer never sees `<workspace>/`.
/// Centralized here so the actor-owned workspace anchor and the
/// renderer share one path-projection rule.
pub fn make_relative_path(workspace: &str, path: &str) -> String {
    let path_string = path.strip_prefix(workspace).map_or_else(
        || path.to_owned(),
        |relative| relative.strip_prefix('/').unwrap_or(relative).to_owned(),
    );
    if path_string.is_empty() || path_string == "." {
        ".".to_owned()
    } else {
        path_string
    }
}

/// Largest terminal height Grok treats as automatic compact mode (rows
/// `<= GROK_AUTO_COMPACT_MAX_ROWS`). Centralized here so the
/// model/view boundary agrees on the canonical threshold.
pub const GROK_AUTO_COMPACT_MAX_ROWS: u16 = 20;

/// Largest terminal height at which Grok still shows the small-screen
/// tip. The band `(GROK_AUTO_COMPACT_MAX_ROWS, GROK_SMALL_SCREEN_TIP_MAX_ROWS]`
/// is the pre-compact ambient window. Centralized here so the
/// visibility predicate and the renderer share one source-backed
/// threshold.
pub const GROK_SMALL_SCREEN_TIP_MAX_ROWS: u16 = 30;

/// Grok derives compact mode from full terminal height; an unmeasured
/// height must not force compact mode. Centralized here so the
/// actor-owned layout projection and the renderer share one
/// compact-mode decision.
pub const fn grok_effective_compact(user_compact: bool, terminal_rows: u16) -> bool {
    user_compact || (terminal_rows > 0 && terminal_rows <= GROK_AUTO_COMPACT_MAX_ROWS)
}

/// Grok keeps the compact-mode tip in the small-screen band immediately
/// above auto-compact. The predicate is pure so event/replay renderers
/// can make the same decision as the live terminal renderer.
pub const fn grok_small_screen_tip_visible(terminal_rows: u16) -> bool {
    terminal_rows > GROK_AUTO_COMPACT_MAX_ROWS && terminal_rows <= GROK_SMALL_SCREEN_TIP_MAX_ROWS
}

/// Render the model-selector row labels for a `ModelCatalogSnapshot`.
/// Each row is the canonical `provider/model` shape, so the actor-owned
/// selector projection and the renderer agree on the displayed text.
pub fn model_selector_rows(
    snapshot: &runie_core::model_catalog::ModelCatalogSnapshot,
) -> Vec<String> {
    snapshot
        .results
        .iter()
        .map(|model| format!("{}/{}", model.provider, model.id))
        .collect()
}

/// Format a repository directory as a `~/relative` label when the path
/// lives under the user's home, and as the full path otherwise.
/// Centralized here so the actor-owned repository projection and the
/// renderer agree on the displayed repository label.
pub fn repository_label(path: &std::path::Path, home: Option<&std::path::Path>) -> String {
    if let Some(home) = home {
        if let Ok(relative) = path.strip_prefix(home) {
            return format!("~/{}", relative.display());
        }
    }
    path.display().to_string()
}
