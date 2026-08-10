//! Renderer-independent prompt state vocabulary.

use runie_core::types::ThemeKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptOutcome {
    Submitted(String),
    Edited,
    Ignored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    #[default]
    Normal,
    Alternate,
    Plan,
    FileSearch,
    FileViewer,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PromptSnapshot {
    pub text: String,
    pub focused: bool,
    pub history: Vec<String>,
    pub history_index: Option<usize>,
    pub history_search: bool,
    pub mode: InputMode,
    pub model_caption: String,
    pub show_placeholder: bool,
    pub file_candidates: Vec<String>,
    pub file_candidate_index: usize,
    pub selected_file: Option<String>,
    pub viewer_lines: Vec<String>,
    pub theme: ThemeKind,
}

/// Pure rotation of the renderer-agnostic prompt input state.
///
/// The embedded cycle moves Normal → Alternate → Plan → Normal. The file
/// modes are pins: they are owned by async actors and the widget must not
/// silently rotate out of them, so they self-loop and only an explicit
/// transition (handled by the widget) leaves them.
pub fn cycle_input_mode(mode: InputMode) -> InputMode {
    match mode {
        InputMode::Normal => InputMode::Alternate,
        InputMode::Alternate => InputMode::Plan,
        InputMode::Plan => InputMode::Normal,
        InputMode::FileSearch => InputMode::FileSearch,
        InputMode::FileViewer => InputMode::FileViewer,
    }
}

impl PromptSnapshot {
    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }

    pub fn render_height(&self) -> u16 {
        if self.mode == InputMode::FileViewer {
            return self.viewer_lines.len().saturating_add(2) as u16;
        }
        let prompt_lines = self.text.lines().count();
        let candidate_lines = if self.mode == InputMode::FileSearch {
            self.file_candidates
                .iter()
                .filter(|candidate| candidate.contains(&self.text))
                .count()
                .min(5)
        } else {
            0
        };
        prompt_lines
            .saturating_add(candidate_lines)
            .saturating_add(2) as u16
    }

    /// Pure caption projection shared by paint and terminal adapters.
    pub fn caption(&self) -> String {
        let mode = match self.mode {
            InputMode::Normal => self.model_caption.clone(),
            InputMode::Alternate => format!("alternate · {}", self.model_caption),
            InputMode::Plan => format!("plan · {}", self.model_caption),
            InputMode::FileSearch => format!("file search · {}", self.model_caption),
            InputMode::FileViewer => format!("file viewer · {}", self.model_caption),
        };
        if self.history_search {
            format!("history search · {mode}")
        } else if self.history_index.is_some() {
            format!("history · {mode}")
        } else if self.text.contains('\n') {
            format!("multiline · {mode}")
        } else {
            mode
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{cycle_input_mode, InputMode, PromptSnapshot};

    #[test]
    fn cycle_input_mode_pins_trio_and_file_self_loops() {
        // Trio: Normal → Alternate → Plan → Normal rotation.
        let mut mode = InputMode::Normal;
        mode = cycle_input_mode(mode);
        assert_eq!(mode, InputMode::Alternate);
        mode = cycle_input_mode(mode);
        assert_eq!(mode, InputMode::Plan);
        mode = cycle_input_mode(mode);
        assert_eq!(mode, InputMode::Normal);

        // FileSearch and FileViewer are owned by async actors; the cycle
        // must pin them so the widget cannot silently rotate out.
        assert_eq!(
            cycle_input_mode(InputMode::FileSearch),
            InputMode::FileSearch
        );
        assert_eq!(
            cycle_input_mode(InputMode::FileViewer),
            InputMode::FileViewer
        );

        // Snapshot round-trip: carrying the cycled mode through
        // PromptSnapshot preserves the agreed value.
        let snapshot = PromptSnapshot {
            mode: cycle_input_mode(InputMode::Alternate),
            ..PromptSnapshot::default()
        };
        assert_eq!(snapshot.mode, InputMode::Plan);
    }

    #[test]
    fn caption_is_a_single_pure_projection_of_prompt_state() {
        let snapshot = PromptSnapshot {
            mode: InputMode::Plan,
            model_caption: "model".into(),
            history_search: true,
            text: "draft\ncontinued".into(),
            ..PromptSnapshot::default()
        };
        assert_eq!(snapshot.caption(), "history search · plan · model");
    }
}
