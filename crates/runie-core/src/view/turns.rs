//! Turn model for conversation navigation.
//!
//! A `Turn` represents a user prompt and all subsequent responses until the
//! next user prompt. This enables turn-based navigation (h/l keys).

use std::ops::Range;

use crate::view::elements::Element;

/// A turn in the conversation: user prompt + all responses until next prompt.
#[derive(Debug, Clone)]
pub struct Turn {
    /// Index of the UserMessage entry that starts this turn.
    pub prompt_index: usize,
    /// Index past the last entry (exclusive, like Range).
    pub end_index: usize,
    /// Whether this turn is currently active.
    pub active: bool,
}

impl Turn {
    /// Get the range of entry indices in this turn.
    pub fn range(&self) -> Range<usize> {
        self.prompt_index..self.end_index
    }

    /// Number of entries in this turn.
    pub fn len(&self) -> usize {
        self.end_index - self.prompt_index
    }

    /// Whether this turn is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Status of a turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TurnStatus {
    #[default]
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// How the scrollback is displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    /// Show all turns in a single timeline.
    #[default]
    AllTurns,
    /// Show only a single turn.
    SingleTurn,
}

/// Rebuild turn index from elements.
pub fn rebuild_turns(elements: &[Element]) -> Vec<Turn> {
    let mut turns = Vec::new();
    let mut current_start: Option<usize> = None;

    for (i, elem) in elements.iter().enumerate() {
        if matches!(elem, Element::UserMessage { .. }) {
            // Close previous turn
            if let Some(start) = current_start {
                turns.push(Turn { prompt_index: start, end_index: i, active: false });
            }
            current_start = Some(i);
        }
    }

    // Close the last turn
    if let Some(start) = current_start {
        turns.push(Turn {
            prompt_index: start,
            end_index: elements.len(),
            active: true, // Last turn may still be active
        });
    }

    turns
}

/// Find which turn contains a given entry index.
pub fn turn_containing(turns: &[Turn], index: usize) -> Option<usize> {
    turns
        .iter()
        .position(|t| t.prompt_index <= index && index < t.end_index)
}

/// Get the visible entry range for a view mode and current turn.
pub fn visible_range(turns: &[Turn], view_mode: ViewMode, current_turn: Option<usize>) -> Range<usize> {
    match view_mode {
        ViewMode::AllTurns => 0..usize::MAX, // Will be clamped by actual elements.len()
        ViewMode::SingleTurn => {
            if let Some(idx) = current_turn {
                if let Some(turn) = turns.get(idx) {
                    return turn.range();
                }
            }
            // Fallback: first turn or all
            if let Some(first) = turns.first() {
                first.range()
            } else {
                0..usize::MAX
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Element;

    #[test]
    fn test_rebuild_turns_empty() {
        let elements: Vec<Element> = vec![];
        let turns = rebuild_turns(&elements);
        assert!(turns.is_empty());
    }

    #[test]
    fn test_rebuild_turns_single_turn() {
        let elements = vec![
            Element::user("hello").at(1.0),
            Element::agent("hi there").at(1.0),
            Element::tool_running("ls", ".", std::time::Instant::now()).at(1.0),
        ];
        let turns = rebuild_turns(&elements);
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].prompt_index, 0);
        assert_eq!(turns[0].end_index, 3);
        assert!(turns[0].active);
    }

    #[test]
    fn test_rebuild_turns_multiple() {
        let elements = vec![
            Element::user("first").at(1.0),
            Element::agent("response1").at(1.0),
            Element::user("second").at(2.0),
            Element::agent("response2").at(2.0),
            Element::agent("response3").at(2.0),
        ];
        let turns = rebuild_turns(&elements);
        assert_eq!(turns.len(), 2);

        assert_eq!(turns[0].prompt_index, 0);
        assert_eq!(turns[0].end_index, 2);
        assert!(!turns[0].active);

        assert_eq!(turns[1].prompt_index, 2);
        assert_eq!(turns[1].end_index, 5);
        assert!(turns[1].active);
    }

    #[test]
    fn test_turn_containing() {
        let turns = vec![
            Turn { prompt_index: 0, end_index: 2, active: false },
            Turn { prompt_index: 2, end_index: 4, active: true },
        ];

        assert_eq!(turn_containing(&turns, 0), Some(0));
        assert_eq!(turn_containing(&turns, 1), Some(0));
        assert_eq!(turn_containing(&turns, 2), Some(1));
        assert_eq!(turn_containing(&turns, 3), Some(1));
        assert_eq!(turn_containing(&turns, 10), None);
    }

    #[test]
    fn test_visible_range_all_turns() {
        let turns = vec![
            Turn { prompt_index: 0, end_index: 2, active: false },
            Turn { prompt_index: 2, end_index: 4, active: true },
        ];

        let range = visible_range(&turns, ViewMode::AllTurns, Some(0));
        assert_eq!(range, 0..usize::MAX);
    }

    #[test]
    fn test_visible_range_single_turn() {
        let turns = vec![
            Turn { prompt_index: 0, end_index: 2, active: false },
            Turn { prompt_index: 2, end_index: 4, active: true },
        ];

        let range = visible_range(&turns, ViewMode::SingleTurn, Some(1));
        assert_eq!(range, 2..4);
    }

    #[test]
    fn test_visible_range_single_turn_fallback() {
        let turns: Vec<Turn> = vec![];
        let range = visible_range(&turns, ViewMode::SingleTurn, None);
        assert_eq!(range, 0..usize::MAX);
    }
}
