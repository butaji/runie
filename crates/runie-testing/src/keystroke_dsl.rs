//! Keystroke DSL → Runie Event translator for deterministic TUI comparisons.
//!
//! This module provides a small string-based DSL for representing keyboard input
//! that maps directly to `runie_core::Event` variants. Useful for test fixtures
//! and for bridging captured tmux pane dumps to Runie test events.
//!
//! ## DSL Syntax
//!
//! | Input | Result |
//! |-------|--------|
//! | `"a"`, `"hello"` | Character input via `Event::Input(c)` |
//! | `"enter"` | Submit via `Event::Newline` |
//! | `"escape"` | Escape via `Event::Escape` |
//! | `"backspace"` | Backspace via `Event::Backspace` |
//! | `"up"` / `"down"` / `"left"` / `"right"` | Arrow navigation |
//! | `"ctrl+c"` | Quit via `Event::Quit` |
//! | `"ctrl+o"` | Toggle expand |
//! | `"ctrl+l"` | Clear screen |
//! | `"ctrl+u"` | Clear line |
//! | `"ctrl+p"` / `"ctrl+n"` | History prev/next |
//! | `"alt+enter"` | Follow up |
//! | `"pageup"` / `"pagedown"` | Page scrolling |
//! | `"home"` / `"end"` | Cursor start/end |
//!
//! ## Usage
//!
//! ```ignore
//! use runie_testing::keystroke_dsl::{parse_keystroke, parse_sequence};
//!
//! let event = parse_keystroke("ctrl+c").unwrap();
//! assert!(matches!(event, runie_core::Event::Quit));
//!
//! let events = parse_sequence("hello<enter>ctrl+c");
//! ```

use runie_core::event::Event as CoreEvent;

/// Parse a single keystroke DSL string to a `CoreEvent`.
///
/// Returns `None` for unknown DSL tokens.
///
/// # DSL Syntax
///
/// - Single characters: `"a"`, `"A"`, `"!"` → `Input(c)`
/// - Named keys: `"enter"`, `"escape"`, `"up"`, etc. → specific events
/// - Modifiers: `"ctrl+c"`, `"alt+enter"`, `"shift+tab"`
pub fn parse_keystroke(dsl: &str) -> Option<CoreEvent> {
    // Handle empty string
    if dsl.is_empty() {
        return None;
    }

    // Handle single space character directly (before trimming)
    if dsl == " " {
        return Some(CoreEvent::Input(' '));
    }

    // Handle modifier prefixes (case-insensitive) - trim only the modifier part
    let dsl_trimmed = dsl.trim();
    let dsl_lower = dsl_trimmed.to_lowercase();
    if dsl_lower.starts_with("ctrl+") || dsl_lower.starts_with("control+") {
        // Use original case for the key part
        let orig_rest = &dsl_trimmed[dsl_lower.find('+').map(|i| i + 1).unwrap_or(0)..];
        return parse_ctrl_combo(orig_rest);
    }
    if dsl_lower.starts_with("alt+") || dsl_lower.starts_with("meta+") {
        // Use original case for the key part
        let orig_rest = &dsl_trimmed[dsl_lower.find('+').map(|i| i + 1).unwrap_or(0)..];
        return parse_alt_combo(orig_rest);
    }
    if dsl_lower.starts_with("shift+") {
        // Use original case for the key part
        let orig_rest = &dsl_trimmed[dsl_lower.find('+').map(|i| i + 1).unwrap_or(0)..];
        return parse_shift_combo(orig_rest);
    }

    // No modifier - match based on lowercase form for named keys
    match dsl_lower.as_str() {
        // Named keys first (case-insensitive) - these must come before single-char patterns
        "enter" | "return" | "submit" => Some(CoreEvent::Newline),
        "escape" | "esc" => Some(CoreEvent::Escape),
        "backspace" | "bksp" | "delete" => Some(CoreEvent::Backspace),
        "tab" => Some(CoreEvent::Input('\t')),
        "space" => Some(CoreEvent::Input(' ')),
        "up" | "arrowup" => Some(CoreEvent::Up),
        "down" | "arrowdown" => Some(CoreEvent::Down),
        "left" | "arrowleft" => Some(CoreEvent::CursorLeft),
        "right" | "arrowright" => Some(CoreEvent::CursorRight),
        "pageup" | "pgup" => Some(CoreEvent::PageUp),
        "pagedown" | "pgdn" => Some(CoreEvent::PageDown),
        "home" => Some(CoreEvent::GoToTop),
        "end" => Some(CoreEvent::GoToBottom),
        "insert" | "ins" => Some(CoreEvent::Undo), // Map insert to undo as closest match
        // Special sequences
        "ctrl+c" | "^c" => Some(CoreEvent::Quit),
        "ctrl+d" | "^d" => Some(CoreEvent::Quit), // Also quit
        "ctrl+z" | "^z" => Some(CoreEvent::Suspend),
        // Single characters - preserve original case (must come after named keys)
        c if c.len() == 1 => {
            let ch = dsl.chars().next().unwrap();
            // Printable ASCII and common Unicode
            if ch.is_ascii_graphic() || ch == ' ' {
                Some(CoreEvent::Input(ch))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Parse a Ctrl+ key combination.
fn parse_ctrl_combo(rest: &str) -> Option<CoreEvent> {
    let rest_lower = rest.to_lowercase();
    match rest_lower.as_str() {
        // Named Ctrl combos first (case-insensitive)
        "c" => Some(CoreEvent::Quit),
        "d" => Some(CoreEvent::Quit),
        "z" => Some(CoreEvent::Suspend),
        "l" => Some(CoreEvent::ClearTransient), // ctrl+l clears
        "u" => Some(CoreEvent::CommandFormBackspace), // ctrl+u clears input
        "a" => Some(CoreEvent::CursorStart),
        "e" => Some(CoreEvent::CursorEnd),
        "k" => Some(CoreEvent::DeleteToEnd),
        "w" => Some(CoreEvent::DeleteWord),
        "p" => Some(CoreEvent::HistoryPrev),
        "n" => Some(CoreEvent::HistoryNext),
        "o" => Some(CoreEvent::ToggleExpand),
        "b" => Some(CoreEvent::CursorLeft),
        "f" => Some(CoreEvent::CursorRight),
        "[" => Some(CoreEvent::Escape),
        // Single letter fallback
        c if c.len() == 1 => {
            let ch = rest.chars().next().unwrap();
            // For Ctrl combos, case doesn't matter - map to the letter
            Some(CoreEvent::Input(ch.to_ascii_lowercase()))
        }
        _ => None,
    }
}

/// Parse an Alt+ key combination.
fn parse_alt_combo(rest: &str) -> Option<CoreEvent> {
    let rest_lower = rest.to_lowercase();
    match rest_lower.as_str() {
        "enter" | "return" => Some(CoreEvent::FollowUp),
        // Named Alt combos first
        "j" | "l" => Some(CoreEvent::CursorRight), // Alt+j/l for navigation in some configs
        "h" => Some(CoreEvent::CursorLeft),
        // Single character fallback
        c if c.len() == 1 => {
            let ch = rest.chars().next().unwrap();
            if ch.is_ascii_graphic() {
                Some(CoreEvent::Input(ch)) // Alt+char sends the char
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Parse a Shift+ key combination.
fn parse_shift_combo(rest: &str) -> Option<CoreEvent> {
    let rest_lower = rest.to_lowercase();
    match rest_lower.as_str() {
        // Named Shift combos first
        "tab" => Some(CoreEvent::PaletteUp), // Shift+tab goes up in dialogs
        "enter" | "return" => Some(CoreEvent::Newline), // Shift+enter is still newline
        // Single character fallback
        c if c.len() == 1 => {
            let ch = rest.chars().next().unwrap();
            // Shift+letter produces uppercase, shift+punctuation produces the char
            if ch.is_ascii_uppercase() || ch.is_ascii_punctuation() {
                Some(CoreEvent::Input(ch))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Parse a sequence of DSL keystrokes separated by delimiters.
///
/// Supports delimiters: `,`, `;`, ` ` (space), `|`, `>`, and literal `<key>` tags.
///
/// # Example
///
/// ```ignore
/// let events = parse_sequence("hello, world<enter>ctrl+c");
/// // Produces: Input('h'), Input('e'), ..., Input(' '), Input('w'), ..., Newline, Quit
/// ```
pub fn parse_sequence(dsl: &str) -> Vec<CoreEvent> {
    let mut events = Vec::new();
    let dsl = dsl.trim();

    // Process character by character
    let chars: Vec<char> = dsl.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        // Skip delimiters
        if ch == ',' || ch == ';' || ch == '|' || ch == '>' {
            i += 1;
            continue;
        }

        // Check for angle bracket sequences: <...>
        if ch == '<' {
            // Find the closing >
            if let Some(end) = chars[i..].iter().position(|&c| c == '>') {
                let inner: String = chars[i + 1..i + end].iter().collect();
                if let Some(evt) = parse_keystroke(&inner) {
                    events.push(evt);
                }
                i += end + 1;
                continue;
            }
        }

        // Check for modifier prefixes (ctrl+, alt+, shift+)
        let remaining = chars.len() - i;
        if remaining >= 5
            && chars[i] == 'c'
            && chars[i + 1] == 't'
            && chars[i + 2] == 'r'
            && chars[i + 3] == 'l'
            && chars[i + 4] == '+'
        {
            // Parse ctrl+ sequence
            let rest: String = chars[i + 5..]
                .iter()
                .take_while(|&&c| c != ' ' && c != ',' && c != ';' && c != '<')
                .collect();
            if let Some(evt) = parse_keystroke(&format!("ctrl+{}", rest)) {
                events.push(evt);
            }
            i += 5 + rest.len();
            continue;
        }

        if remaining >= 4
            && chars[i] == 'a'
            && chars[i + 1] == 'l'
            && chars[i + 2] == 't'
            && chars[i + 3] == '+'
        {
            let rest: String = chars[i + 4..]
                .iter()
                .take_while(|&&c| c != ' ' && c != ',' && c != ';' && c != '<')
                .collect();
            if let Some(evt) = parse_keystroke(&format!("alt+{}", rest)) {
                events.push(evt);
            }
            i += 4 + rest.len();
            continue;
        }

        if remaining >= 6
            && chars[i] == 's'
            && chars[i + 1] == 'h'
            && chars[i + 2] == 'i'
            && chars[i + 3] == 'f'
            && chars[i + 4] == 't'
            && chars[i + 5] == '+'
        {
            let rest: String = chars[i + 6..]
                .iter()
                .take_while(|&&c| c != ' ' && c != ',' && c != ';' && c != '<')
                .collect();
            if let Some(evt) = parse_keystroke(&format!("shift+{}", rest)) {
                events.push(evt);
            }
            i += 6 + rest.len();
            continue;
        }

        // Check for space - emit Input(' ') and skip
        if ch == ' ' {
            events.push(CoreEvent::Input(' '));
            i += 1;
            continue;
        }

        // Check for alphabetic sequences
        if ch.is_ascii_alphabetic() {
            // Check if the next character is also alphabetic (word continuation)
            let has_more_alpha = i + 1 < chars.len() && chars[i + 1].is_ascii_alphabetic();
            
            if has_more_alpha {
                // Multi-letter word - emit each character separately
                // This handles strings like "hello" as individual keystrokes
                let end = chars[i..]
                    .iter()
                    .position(|c| !c.is_ascii_alphabetic())
                    .unwrap_or(chars.len() - i);
                let word: String = chars[i..i + end].iter().collect();
                for c in word.chars() {
                    events.push(CoreEvent::Input(c));
                }
                i += end;
                continue;
            } else {
                // Single letter - emit it directly
                events.push(CoreEvent::Input(ch));
                i += 1;
                continue;
            }
        }

        // Check for digits and other word characters
        if ch.is_ascii_digit() || ch == '_' || ch == '-' || ch == '.' {
            // Collect the word
            let end = chars[i..]
                .iter()
                .position(|&c| !c.is_ascii_alphanumeric() && c != '_' && c != '-' && c != '.')
                .unwrap_or(chars.len() - i);
            let word: String = chars[i..i + end].iter().collect();

            // Check if it's a named key (like "enter")
            if let Some(evt) = parse_keystroke(&word) {
                events.push(evt);
            }
            i += end;
            continue;
        }

        // Single character
        if let Some(evt) = parse_keystroke(&ch.to_string()) {
            events.push(evt);
        }
        i += 1;
    }

    events
}

/// Parse a tmux-style key sequence (e.g., `"C-c"`, `"M-RET"`, `"M-Enter"`).
///
/// tmux notation: `C-` = Ctrl, `M-` = Alt, `-` = separator
pub fn parse_tmux_style(dsl: &str) -> Option<CoreEvent> {
    let dsl = dsl.trim();
    if dsl.starts_with("C-") {
        // Handle "C-" (tmux Ctrl) style
        let rest = &dsl[2..];
        parse_ctrl_combo(rest)
    } else if dsl.starts_with("M-") {
        let rest = &dsl[2..];
        // Handle tmux key names: RET, Enter, etc.
        let rest_lower = rest.to_lowercase();
        match rest_lower.as_str() {
            "ret" | "enter" => Some(CoreEvent::FollowUp),
            _ => parse_alt_combo(rest),
        }
    } else {
        parse_keystroke(dsl)
    }
}

/// Convert a crossterm `KeyEvent` to DSL string representation.
pub fn key_event_to_dsl(event: &crossterm::event::KeyEvent) -> String {
    use crossterm::event::{KeyCode, KeyModifiers};

    let mut parts = Vec::new();

    if event.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("ctrl".to_string());
    }
    if event.modifiers.contains(KeyModifiers::ALT) {
        parts.push("alt".to_string());
    }
    if event.modifiers.contains(KeyModifiers::SHIFT) && !matches!(event.code, KeyCode::Char(_)) {
        parts.push("shift".to_string());
    }

    let key = match event.code {
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "enter".to_string(),
        KeyCode::Backspace => "backspace".to_string(),
        KeyCode::Esc => "escape".to_string(),
        KeyCode::Up => "up".to_string(),
        KeyCode::Down => "down".to_string(),
        KeyCode::Left => "left".to_string(),
        KeyCode::Right => "right".to_string(),
        KeyCode::PageUp => "pageup".to_string(),
        KeyCode::PageDown => "pagedown".to_string(),
        KeyCode::Home => "home".to_string(),
        KeyCode::End => "end".to_string(),
        KeyCode::Tab => "tab".to_string(),
        KeyCode::Insert => "insert".to_string(),
        KeyCode::Delete => "delete".to_string(),
        KeyCode::F(n) => format!("f{}", n),
        KeyCode::Null => "null".to_string(),
        KeyCode::CapsLock => "capslock".to_string(),
        KeyCode::NumLock => "numlock".to_string(),
        KeyCode::ScrollLock => "scrolllock".to_string(),
        KeyCode::Pause => "pause".to_string(),
        KeyCode::Media(_) => "media".to_string(),
        KeyCode::Modifier(_) => "modifier".to_string(),
        KeyCode::KeypadBegin => "keypad".to_string(),
        KeyCode::BackTab => "backtab".to_string(),
        KeyCode::PrintScreen => "printscreen".to_string(),
        KeyCode::Menu => "menu".to_string(),
    };

    parts.push(key);
    parts.join("+")
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_input() {
        assert!(matches!(parse_keystroke("a"), Some(CoreEvent::Input('a'))));
        assert!(matches!(parse_keystroke("A"), Some(CoreEvent::Input('A'))));
        assert!(matches!(parse_keystroke(" "), Some(CoreEvent::Input(' '))));
        assert!(matches!(parse_keystroke("!"), Some(CoreEvent::Input('!'))));
    }

    #[test]
    fn test_named_keys() {
        assert!(matches!(parse_keystroke("enter"), Some(CoreEvent::Newline)));
        assert!(matches!(parse_keystroke("ESCAPE"), Some(CoreEvent::Escape)));
        assert!(matches!(parse_keystroke("Backspace"), Some(CoreEvent::Backspace)));
        assert!(matches!(parse_keystroke("up"), Some(CoreEvent::Up)));
        assert!(matches!(parse_keystroke("DOWN"), Some(CoreEvent::Down)));
        assert!(matches!(parse_keystroke("left"), Some(CoreEvent::CursorLeft)));
        assert!(matches!(parse_keystroke("right"), Some(CoreEvent::CursorRight)));
        assert!(matches!(parse_keystroke("pageup"), Some(CoreEvent::PageUp)));
        assert!(matches!(parse_keystroke("pagedown"), Some(CoreEvent::PageDown)));
        assert!(matches!(parse_keystroke("home"), Some(CoreEvent::GoToTop)));
        assert!(matches!(parse_keystroke("end"), Some(CoreEvent::GoToBottom)));
    }

    #[test]
    fn test_ctrl_combos() {
        assert!(matches!(parse_keystroke("ctrl+c"), Some(CoreEvent::Quit)));
        assert!(matches!(parse_keystroke("ctrl+C"), Some(CoreEvent::Quit)));
        assert!(matches!(parse_keystroke("control+c"), Some(CoreEvent::Quit)));
        assert!(matches!(parse_keystroke("ctrl+o"), Some(CoreEvent::ToggleExpand)));
        assert!(matches!(parse_keystroke("ctrl+l"), Some(CoreEvent::ClearTransient)));
        assert!(matches!(parse_keystroke("ctrl+p"), Some(CoreEvent::HistoryPrev)));
        assert!(matches!(parse_keystroke("ctrl+n"), Some(CoreEvent::HistoryNext)));
        assert!(matches!(parse_keystroke("ctrl+a"), Some(CoreEvent::CursorStart)));
        assert!(matches!(parse_keystroke("ctrl+e"), Some(CoreEvent::CursorEnd)));
    }

    #[test]
    fn test_alt_combos() {
        assert!(matches!(parse_keystroke("alt+enter"), Some(CoreEvent::FollowUp)));
        assert!(matches!(parse_keystroke("meta+enter"), Some(CoreEvent::FollowUp)));
    }

    #[test]
    fn test_shift_combo() {
        assert!(matches!(parse_keystroke("shift+tab"), Some(CoreEvent::PaletteUp)));
        assert!(matches!(parse_keystroke("shift+enter"), Some(CoreEvent::Newline)));
    }

    #[test]
    fn test_tmux_style() {
        assert!(matches!(parse_tmux_style("C-c"), Some(CoreEvent::Quit)));
        assert!(matches!(parse_tmux_style("M-RET"), Some(CoreEvent::FollowUp)));
        assert!(matches!(parse_tmux_style("C-M-c"), None)); // Not supported
    }

    #[test]
    fn test_sequence_basic() {
        let events = parse_sequence("hello");
        assert_eq!(events.len(), 5);
        assert!(events.iter().all(|e| matches!(e, CoreEvent::Input(_))));
        let chars: Vec<char> = events
            .iter()
            .filter_map(|e| match e {
                CoreEvent::Input(c) => Some(*c),
                _ => None,
            })
            .collect();
        assert_eq!(chars, ['h', 'e', 'l', 'l', 'o']);
    }

    #[test]
    fn test_sequence_with_delimiters() {
        let events = parse_sequence("hi, there<enter>");
        let expected = vec![
            CoreEvent::Input('h'),
            CoreEvent::Input('i'),
            CoreEvent::Input(' '),
            CoreEvent::Input('t'),
            CoreEvent::Input('h'),
            CoreEvent::Input('e'),
            CoreEvent::Input('r'),
            CoreEvent::Input('e'),
            CoreEvent::Newline,
        ];
        assert_eq!(events.len(), expected.len());
        for (got, want) in events.iter().zip(expected.iter()) {
            assert!(matches!((got, want), (CoreEvent::Input(g), CoreEvent::Input(w)) if g == w)
                || std::mem::discriminant(got) == std::mem::discriminant(want),
                "mismatch: {:?} vs {:?}", got, want);
        }
    }

    #[test]
    fn test_sequence_ctrl_enter() {
        let events = parse_sequence("type<space>something<alt+enter>");
        assert!(events.contains(&CoreEvent::FollowUp));
    }

    #[test]
    fn test_unknown_token() {
        assert!(parse_keystroke("unknown").is_none());
        assert!(parse_keystroke("").is_none());
    }

    #[test]
    fn test_key_event_roundtrip() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        // ctrl+c
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let dsl = key_event_to_dsl(&key);
        assert_eq!(dsl, "ctrl+c");

        // plain enter
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        let dsl = key_event_to_dsl(&key);
        assert_eq!(dsl, "enter");

        // alt+enter
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT);
        let dsl = key_event_to_dsl(&key);
        assert_eq!(dsl, "alt+enter");
    }
}
