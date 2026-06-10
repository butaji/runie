use crate::model::AppState;

/// State for cycling through tab-completion matches.
#[derive(Debug, Clone, Default)]
pub struct TabCompleteState {
    /// The prefix that was being completed when matches were found.
    pub prefix: String,
    /// All matches for the current prefix.
    pub matches: Vec<String>,
    /// Current index in the matches cycle.
    pub index: usize,
    /// Start byte position of the token in the input.
    pub token_start: usize,
    /// End byte position of the token in the input.
    pub token_end: usize,
}

impl AppState {
    /// Handle Tab key in the input box — shell-like file completion.
    /// Completes relative file/folder paths after @, ./, or ../.
    pub(crate) fn tab_complete(&mut self) {
        let input = &self.input.input;
        let cursor = self.input.cursor_pos;

        // Find the token being completed: text after last @, ./, or ../
        let (token_start, token_end, kind) = match find_completion_token(input, cursor) {
            Some(t) => t,
            None => {
                self.input.input_flash = 3;
                return;
            }
        };

        let prefix = &input[token_start..token_end];
        let base = std::env::current_dir().unwrap_or_default();

        // Check if we have an existing completion state for the same prefix
        if let Some(ref state) = self.input.tab_complete {
            if state.prefix == prefix && state.token_start == token_start {
                // Cycle to next match
                let next_idx = (state.index + 1) % state.matches.len();
                let replacement = &state.matches[next_idx];
                self.apply_completion(token_start, token_end, replacement, next_idx);
                return;
            }
        }

        // Find new matches
        let matches = find_file_matches(prefix, &base, kind);
        if matches.is_empty() {
            self.input.input_flash = 3;
            return;
        }

        let replacement = matches[0].clone();
        self.input.tab_complete = Some(TabCompleteState {
            prefix: prefix.to_string(),
            matches,
            index: 0,
            token_start,
            token_end,
        });
        self.apply_completion(token_start, token_end, &replacement, 0);
    }

    fn apply_completion(
        &mut self,
        start: usize,
        end: usize,
        replacement: &str,
        index: usize,
    ) {
        let mut new_input = self.input.input[..start].to_string();
        new_input.push_str(replacement);
        new_input.push_str(&self.input.input[end..]);
        self.input.input = new_input;
        self.input.cursor_pos = start + replacement.len();
        if let Some(ref mut state) = self.input.tab_complete {
            state.index = index;
            state.token_end = start + replacement.len();
        }
        self.mark_dirty();
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TokenKind {
    At,      // @filename
    Relative, // ./path or ../path
}

/// Find the completion token at the cursor position.
/// Returns (start, end, kind) if a completable token is found.
fn find_completion_token(input: &str, cursor: usize) -> Option<(usize, usize, TokenKind)> {
    // Look for @ before cursor
    if let Some(at_pos) = input[..cursor.min(input.len())].rfind('@') {
        let start = at_pos + 1; // position after @
        if start <= cursor {
            return Some((start, cursor, TokenKind::At));
        }
    }

    // Look for ./ or ../ before cursor
    let before = &input[..cursor.min(input.len())];
    for prefix in ["../", "./"] {
        if let Some(pos) = before.rfind(prefix) {
            let start = pos + prefix.len();
            if start <= cursor {
                return Some((start, cursor, TokenKind::Relative));
            }
        }
    }

    None
}

/// Find files/folders in base directory matching the prefix.
fn find_file_matches(prefix: &str, base: &std::path::Path, _kind: TokenKind) -> Vec<String> {
    let mut results = Vec::new();
    let prefix_lower = prefix.to_lowercase();

    let Ok(entries) = std::fs::read_dir(base) else { return results };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') && !prefix.starts_with('.') {
            continue; // skip hidden unless explicitly searched
        }
        if name.to_lowercase().starts_with(&prefix_lower) {
            let with_slash = if entry.file_type().map_or(false, |t| t.is_dir()) {
                format!("{}/", name)
            } else {
                name
            };
            results.push(with_slash);
        }
    }
    results.sort();
    results
}
