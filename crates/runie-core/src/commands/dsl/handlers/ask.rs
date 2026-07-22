//! `/ask` command handler.
//!
//! Opens an interactive multi-question dialog using the AskUserQuestion system.

use crate::commands::CommandResult;
use crate::model::{Question, QuestionOption};
use crate::Event;

/// Handle `/ask` — opens an interactive question dialog.
///
/// The questions are defined here for now. In the future, this could be
/// extended to accept questions as arguments or load them from a prompt template.
#[allow(clippy::too_many_lines)]
pub fn handle_ask(_state: &mut crate::model::AppState, _args: &str) -> CommandResult {
    let questions = build_default_questions();
    let request_id = format!("ask-{}", uuid::Uuid::new_v4());

    CommandResult::Event(Event::AskUserQuestion { request_id, questions })
}

/// Build the default question set.
///
/// These match Kimi's /ask dialog: Project Type, Priority, Extras.
#[allow(clippy::too_many_lines)]
fn build_default_questions() -> Vec<Question> {
    vec![
        Question {
            id: "project_type".into(),
            question: "Project Type".into(),
            options: vec![
                QuestionOption { id: "_1".into(), label: "New feature".into() },
                QuestionOption { id: "_2".into(), label: "Bug fix".into() },
                QuestionOption { id: "_3".into(), label: "Refactor".into() },
                QuestionOption { id: "_S".into(), label: "Skip".into() },
            ],
        },
        Question {
            id: "priority".into(),
            question: "Priority".into(),
            options: vec![
                QuestionOption { id: "_1".into(), label: "High".into() },
                QuestionOption { id: "_2".into(), label: "Medium".into() },
                QuestionOption { id: "_3".into(), label: "Low".into() },
                QuestionOption { id: "_S".into(), label: "Skip".into() },
            ],
        },
        Question {
            id: "extras".into(),
            question: "Extras".into(),
            options: vec![
                QuestionOption { id: "_1".into(), label: "Add tests".into() },
                QuestionOption { id: "_2".into(), label: "Update docs".into() },
                QuestionOption { id: "_3".into(), label: "Breaking change".into() },
                QuestionOption { id: "_S".into(), label: "Skip".into() },
            ],
        },
    ]
}
