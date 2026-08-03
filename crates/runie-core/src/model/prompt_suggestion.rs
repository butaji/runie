//! Grok-compatible prompt suggestion controller.
//!
//! This state is deliberately independent from file completion: it owns the
//! full predicted prompt, filters it against the current draft, and rejects
//! stale or malformed provider responses.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptSuggestionState {
    pub full_text: String,
    pub generation: u64,
    pub dismissed: bool,
    pub enabled: bool,
}

impl Default for PromptSuggestionState {
    fn default() -> Self {
        Self {
            full_text: String::new(),
            generation: 0,
            dismissed: false,
            enabled: std::env::var("RUNIE_PROMPT_SUGGESTIONS")
                .map(|value| {
                    !matches!(
                        value.trim().to_ascii_lowercase().as_str(),
                        "0" | "false" | "off"
                    )
                })
                .unwrap_or(true),
        }
    }
}

impl PromptSuggestionState {
    /// Re-read the runtime opt-out, mirroring Grok's live settings gate.
    pub fn refresh_enabled_from_env(&mut self) {
        self.enabled = std::env::var("RUNIE_PROMPT_SUGGESTIONS")
            .map(|value| {
                !matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "0" | "false" | "off"
                )
            })
            .unwrap_or(true);
    }
}

impl PromptSuggestionState {
    pub fn begin_fetch(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.generation
    }

    /// Install only the response belonging to the current generation.
    /// Grok rejects empty, whitespace-only, and multiline suggestions.
    pub fn on_loaded(&mut self, suggestion: Option<String>, generation: u64) -> bool {
        if generation != self.generation {
            return false;
        }
        let Some(text) = suggestion else {
            self.full_text.clear();
            return false;
        };
        if text.trim().is_empty() || text.contains('\n') {
            self.full_text.clear();
            return false;
        }
        self.full_text = text;
        self.dismissed = false;
        true
    }

    /// Return only the untyped suffix when the draft is a proper prefix.
    pub fn ghost_for<'a>(&'a self, draft: &str) -> Option<&'a str> {
        if !self.enabled || self.dismissed || self.full_text.is_empty() {
            return None;
        }
        let remainder = self.full_text.strip_prefix(draft)?;
        (!remainder.is_empty()).then_some(remainder)
    }

    pub fn accept(&mut self, draft: &str) -> Option<String> {
        let remainder = self.ghost_for(draft)?.to_owned();
        self.full_text.clear();
        Some(remainder)
    }

    pub fn dismiss(&mut self) {
        self.dismissed = true;
    }

    pub fn clear(&mut self) {
        self.full_text.clear();
        self.dismissed = false;
        self.begin_fetch();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_current_generation_and_shrinks_prefix() {
        let mut state = PromptSuggestionState::default();
        let generation = state.begin_fetch();
        assert!(state.on_loaded(Some("list the files".into()), generation));
        assert_eq!(state.ghost_for("list"), Some(" the files"));
        assert_eq!(state.ghost_for("other"), None);
    }

    #[test]
    fn stale_empty_and_multiline_payloads_are_rejected() {
        let mut state = PromptSuggestionState::default();
        let generation = state.begin_fetch();
        assert!(!state.on_loaded(Some("stale".into()), generation - 1));
        assert!(!state.on_loaded(Some("  ".into()), generation));
        assert!(!state.on_loaded(Some("one\ntwo".into()), generation));
    }

    #[test]
    fn dismiss_and_accept_clear_visibility() {
        let mut state = PromptSuggestionState::default();
        let generation = state.begin_fetch();
        state.on_loaded(Some("hello world".into()), generation);
        state.dismiss();
        assert_eq!(state.ghost_for(""), None);
        state.dismissed = false;
        assert_eq!(state.accept("hello"), Some(" world".into()));
        assert_eq!(state.ghost_for(""), None);
    }
}
