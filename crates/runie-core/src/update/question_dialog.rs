//! Hosted question dialog builder — builds a non-closable PanelStack
//! for the AskUserQuestion flow.

use crate::commands::{DialogKind, DialogState};
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::model::QuestionState;
use crate::Event;

/// Build a hosted form panel for the current question in the dialog.
/// Shows the question text and numbered options (1, 2, 3, S to skip).
pub fn build_question_dialog(state: &QuestionState) -> PanelStack {
    let request_id = state.request_id.clone();
    let Some(question) = state.current_question() else {
        // All questions answered — shouldn't happen in practice
        return build_complete_dialog(&request_id);
    };

    let progress = format!(
        "Question {}/{}",
        state.current_index + 1,
        state.questions.len()
    );

    let panel = build_question_panel(
        question.question.as_str(),
        &progress,
        &state.questions,
        state.current_index,
        &request_id,
    );

    PanelStack::new(panel)
}

/// Build a panel for a single question.
fn build_question_panel(
    question_text: &str,
    progress: &str,
    questions: &[crate::model::Question],
    current_index: usize,
    request_id: &str,
) -> Panel {
    let mut panel = Panel::new("question", format!(" {} ", progress))
        .form()
        .non_closable()
        .header(question_text);

    // Map options: 1→_1, 2→_2, 3→_3, S→_S
    for opt in questions[current_index].options.iter() {
        let shortcut = match opt.id.as_str() {
            "_1" => "1",
            "_2" => "2",
            "_3" => "3",
            "_S" => "S",
            _ => "",
        };
        let label = format!("_{}. {}", shortcut, opt.label);

        panel = panel.item(
            label,
            ItemAction::Emit(Event::QuestionAnswer { request_id: request_id.to_string(), option_id: opt.id.clone() }),
        );
    }

    // Add skip hint and submit (only shown when all questions are done)
    if questions[current_index]
        .options
        .iter()
        .any(|o| o.id == "_S")
    {
        panel = panel.item(
            "  Press S to skip",
            ItemAction::Emit(Event::QuestionSkip { request_id: request_id.to_string() }),
        );
    }

    panel
}

/// Build the completion panel when all questions are answered.
fn build_complete_dialog(request_id: &str) -> PanelStack {
    let panel = Panel::new("question_complete", " Questions Complete ")
        .form()
        .non_closable()
        .header("All questions answered!")
        .item(
            "Enter Submit answers",
            ItemAction::Emit(Event::QuestionSubmit { request_id: request_id.to_string() }),
        );

    PanelStack::new(panel)
}

/// Build and wrap a hosted question dialog as an open `DialogState`.
pub fn open_question_dialog(state: &QuestionState) -> DialogState {
    DialogState::Active { kind: DialogKind::Generic, panels: build_question_dialog(state) }
}
