//! Thought extraction from assistant content, including `<think>` reasoning blocks.

use crate::labels::thought_with_time;
use crate::update::{content_has_tool_markers, strip_tool_markers};

#[derive(Debug)]
pub(crate) struct ThoughtPlan {
    pub thought_content: String,
    pub visible_content: Option<String>,
    pub remove_assistant: bool,
}

impl ThoughtPlan {
    pub fn plain(duration: f64) -> Self {
        Self {
            thought_content: thought_with_time(duration),
            visible_content: None,
            remove_assistant: false,
        }
    }
}

pub(crate) fn plan_thought(content: &str, duration: f64) -> ThoughtPlan {
    let has_tools = content_has_tool_markers(content);
    let stripped = strip_tool_markers(content);
    let (visible, reasoning) = split_think_blocks(&stripped);
    if has_tools && !stripped.trim().is_empty() {
        return ThoughtPlan {
            thought_content: format!("{}\n{}", thought_with_time(duration), stripped),
            visible_content: None,
            remove_assistant: true,
        };
    }
    if let Some(reasoning) = reasoning {
        let thought_content = format!("{}\n{}", thought_with_time(duration), reasoning);
        if visible.trim().is_empty() {
            return ThoughtPlan {
                thought_content,
                visible_content: None,
                remove_assistant: true,
            };
        }
        return ThoughtPlan {
            thought_content,
            visible_content: Some(visible),
            remove_assistant: false,
        };
    }
    ThoughtPlan {
        thought_content: thought_with_time(duration),
        visible_content: None,
        remove_assistant: false,
    }
}

/// Split `<think>...</think>` reasoning blocks out of model text.
/// Returns `(visible_text, optional_reasoning)`. Unclosed `<think>` tags
/// are treated as reasoning that continues to the end of the string.
pub(crate) fn split_think_blocks(content: &str) -> (String, Option<String>) {
    let mut visible = String::new();
    let mut reasoning = String::new();
    let mut in_reasoning = false;
    let mut rest = content;
    loop {
        let marker = if in_reasoning { "</think>" } else { "<think>" };
        match rest.find(marker) {
            Some(idx) => {
                if in_reasoning {
                    reasoning.push_str(&rest[..idx]);
                } else {
                    visible.push_str(&rest[..idx]);
                }
                rest = &rest[idx + marker.len()..];
                in_reasoning = !in_reasoning;
            }
            None => break,
        }
    }
    if in_reasoning {
        reasoning.push_str(rest);
    } else {
        visible.push_str(rest);
    }
    if reasoning.is_empty() {
        (visible, None)
    } else {
        (visible, Some(reasoning))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AppState, Role};

    #[test]
    fn split_think_blocks_extracts_reasoning() {
        let (visible, reasoning) = split_think_blocks("<think>reason</think>answer");
        assert_eq!(visible, "answer");
        assert_eq!(reasoning, Some("reason".to_string()));
    }

    #[test]
    fn split_think_blocks_handles_unclosed_tag() {
        let (visible, reasoning) = split_think_blocks("visible<think>still reasoning");
        assert_eq!(visible, "visible");
        assert_eq!(reasoning, Some("still reasoning".to_string()));
    }

    #[test]
    fn split_think_blocks_preserves_text_without_tags() {
        let (visible, reasoning) = split_think_blocks("just an answer");
        assert_eq!(visible, "just an answer");
        assert_eq!(reasoning, None);
    }

    #[test]
    fn plan_thought_keeps_visible_answer_and_reasoning() {
        let plan = plan_thought("<think>reason</think>answer", 1.5);
        assert!(plan.thought_content.contains("reason"));
        assert_eq!(plan.visible_content, Some("answer".to_string()));
        assert!(!plan.remove_assistant);
    }

    #[test]
    fn plan_thought_removes_assistant_when_only_reasoning() {
        let plan = plan_thought("<think>reason</think>", 1.0);
        assert!(plan.remove_assistant);
        assert!(plan.thought_content.contains("reason"));
    }

    #[test]
    fn think_tags_are_split_into_thought_and_answer() {
        let mut state = AppState::default();
        state.set_thinking("req.0".into());
        state.append_response(
            "req.0".into(),
            "<think>\nreasoning\n</think>\nanswer".into(),
        );
        state.add_thought("req.0".into());
        state.finish_turn("req.0".into());

        let thoughts: Vec<_> = state
            .session
            .messages
            .iter()
            .filter(|m| m.role == Role::Thought)
            .collect();
        assert_eq!(thoughts.len(), 1);
        assert!(thoughts[0].content().contains("reasoning"));
        assert!(!thoughts[0].content().contains("<think>"));
        assert!(!thoughts[0].content().contains("</think>"));

        let assistants: Vec<_> = state
            .session
            .messages
            .iter()
            .filter(|m| m.role == Role::Assistant)
            .collect();
        assert_eq!(assistants.len(), 1);
        assert_eq!(assistants[0].content(), "answer");
    }
}
