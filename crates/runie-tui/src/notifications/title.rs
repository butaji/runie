//! Pure Grok-style terminal title composition.

const SPINNER: [char; 8] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TitleState {
    pub session_name: Option<String>,
    pub model: Option<String>,
    pub activity: Option<String>,
    pub cwd: Option<String>,
    pub busy: bool,
    pub pending_permissions: bool,
    pub focused: bool,
    pub frame: u64,
}

/// Compose a sanitized title using the Grok ordering: action, spinner,
/// activity, session, model, cwd. Empty idle titles fall back to `runie`.
pub fn compose_title(state: &TitleState) -> String {
    let mut items = Vec::new();
    if state.pending_permissions {
        items.push("⚠ Action Required".to_owned());
    }
    if state.busy {
        items.push(SPINNER[(state.frame as usize / 8) % SPINNER.len()].to_string());
        items.push(
            state
                .activity
                .as_deref()
                .map(sanitize)
                .unwrap_or_else(|| "Waiting".to_owned()),
        );
    }
    for value in [&state.session_name, &state.model, &state.cwd] {
        if let Some(value) = value.as_deref().filter(|v| !v.is_empty()) {
            items.push(sanitize(value));
        }
    }
    items.push("runie".to_owned());
    items.join(" - ")
}

#[derive(Debug, Default)]
pub struct TitleManager {
    last_title: Option<String>,
}

impl TitleManager {
    /// Return an OSC-0 title escape only when the composed title changes.
    pub fn update(&mut self, state: &TitleState) -> Option<String> {
        let title = compose_title(state);
        if self.last_title.as_deref() == Some(&title) {
            return None;
        }
        self.last_title = Some(title.clone());
        Some(format!("\x1b]0;{title}\x07"))
    }

    /// Reset the title to the Runie identity and forget the cached value.
    pub fn reset(&mut self) -> String {
        self.last_title = Some("runie".to_owned());
        "\x1b]0;runie\x07".to_owned()
    }
}

fn sanitize(value: &str) -> String {
    value.chars().filter(|c| !c.is_control()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_title_falls_back_to_runie() {
        assert_eq!(compose_title(&TitleState::default()), "runie");
    }

    #[test]
    fn busy_title_includes_spinner_activity_and_session() {
        let state = TitleState {
            busy: true,
            activity: Some("Thinking".into()),
            session_name: Some("demo".into()),
            frame: 8,
            ..Default::default()
        };
        assert_eq!(compose_title(&state), "⠙ - Thinking - demo - runie");
    }

    #[test]
    fn permission_title_precedes_activity_and_strips_controls() {
        let state = TitleState {
            pending_permissions: true,
            busy: true,
            activity: Some("Run\ncommand".into()),
            session_name: Some("a\u{1b}[31m".into()),
            ..Default::default()
        };
        let title = compose_title(&state);
        assert!(title.starts_with("⚠ Action Required - ⠋ - Runcommand"));
        assert!(!title.contains('\n'));
        assert!(!title.contains('\u{1b}'));
    }

    #[test]
    fn manager_deduplicates_and_resets_title() {
        let mut manager = TitleManager::default();
        let state = TitleState::default();
        assert_eq!(manager.update(&state), Some("\x1b]0;runie\x07".into()));
        assert_eq!(manager.update(&state), None);
        assert_eq!(manager.reset(), "\x1b]0;runie\x07");
    }
}
