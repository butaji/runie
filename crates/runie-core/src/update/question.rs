//! AskUserQuestion dialog event handling.
//!
//! Projects AskUserQuestion events to AppState, mirroring the permission
//! dialog pattern exactly.

use crate::dialog::PanelItem;
use crate::model::{AppState, InputReceiver, QuestionState, Role};
use crate::update::question_dialog::open_question_dialog;
use crate::Event;

/// Clear the question dialog UI if it matches the given request id.
fn clear_matching_question(state: &mut AppState, request_id: &str) {
    if state
        .question_state()
        .map(|r| r.request_id == request_id)
        .unwrap_or(false)
    {
        *state.question_state_mut() = None;
        close_question_dialog(state);
        state.view_mut().dirty = true;
    }
}

/// Close the question dialog if it is currently open, returning focus to
/// the chat input.
fn close_question_dialog(state: &mut AppState) {
    let is_question = state
        .open_dialog()
        .and_then(|d| d.panel_stack())
        .and_then(|s| s.current())
        .map(|p| p.id.starts_with("question"))
        .unwrap_or(false);
    if is_question {
        *state.open_dialog_mut() = None;
        state.view_mut().input_receiver = InputReceiver::ChatInput;
        state.view_mut().dirty = true;
    }
}

/// Project question events to AppState.
pub(crate) fn question_event(state: &mut AppState, event: Event) {
    match event {
        Event::AskUserQuestion {
            request_id,
            questions,
        } => {
            if questions.is_empty() {
                // Nothing to ask — silently ignore
                return;
            }
            let qs = QuestionState::new(request_id, questions);
            *state.question_state_mut() = Some(qs.clone());
            *state.open_dialog_mut() = Some(open_question_dialog(&qs));
            state.view_mut().input_receiver = InputReceiver::Dialog;
            // Pause streaming while waiting for answers
            state.agent_state_mut().streaming = false;
            state.view_mut().dirty = true;
        }
        Event::QuestionAnswer {
            request_id,
            option_id,
        } => {
            // Answer the current question and advance
            if let Some(ref mut qs) = state.question_state_mut() {
                if qs.request_id == request_id {
                    qs.answer(option_id);
                    if qs.is_complete() {
                        // All answered — rebuild to show completion panel
                        let qs = state.question_state().cloned().unwrap();
                        *state.open_dialog_mut() = Some(open_question_dialog(&qs));
                    } else {
                        // Move to next question — rebuild dialog
                        let qs = state.question_state().cloned().unwrap();
                        *state.open_dialog_mut() = Some(open_question_dialog(&qs));
                    }
                    state.view_mut().dirty = true;
                }
            }
        }
        Event::QuestionSkip { request_id } => {
            if let Some(ref mut qs) = state.question_state_mut() {
                if qs.request_id == request_id {
                    qs.skip();
                    if qs.is_complete() {
                        let qs = state.question_state().cloned().unwrap();
                        *state.open_dialog_mut() = Some(open_question_dialog(&qs));
                    } else {
                        let qs = state.question_state().cloned().unwrap();
                        *state.open_dialog_mut() = Some(open_question_dialog(&qs));
                    }
                    state.view_mut().dirty = true;
                }
            }
        }
        Event::QuestionSubmit { request_id } => {
            if state
                .question_state()
                .map(|qs| qs.request_id == request_id)
                .unwrap_or(false)
            {
                // Emit a system message summarizing answers
                let answers = state
                    .question_state()
                    .map(|qs| {
                        qs.answers
                            .iter()
                            .map(|a| {
                                let question = qs
                                    .questions
                                    .iter()
                                    .find(|q| q.id == a.question_id)
                                    .map(|q| q.question.as_str())
                                    .unwrap_or("?");
                                let option = qs
                                    .questions
                                    .iter()
                                    .find(|q| q.id == a.question_id)
                                    .and_then(|q| q.options.iter().find(|o| o.id == a.option_id))
                                    .map(|o| o.label.as_str())
                                    .unwrap_or("skipped");
                                format!("{}: {}", question, option)
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default();

                state.add_system_msg(format!(
                    "Answers collected:\n{}",
                    if answers.is_empty() { "(all skipped)".to_string() } else { answers }
                ));
                clear_matching_question(state, &request_id);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{DialogKind, DialogState};
    use crate::dialog::{Panel, PanelStack};
    use crate::model::{Question, QuestionOption, QuestionState};

    fn open_sample_question_dialog(state: &mut AppState) {
        let questions = vec![
            Question {
                id: "type".into(),
                question: "Project Type".into(),
                options: vec![
                    QuestionOption {
                        id: "_1".into(),
                        label: "Feature".into(),
                    },
                    QuestionOption {
                        id: "_2".into(),
                        label: "Bug fix".into(),
                    },
                    QuestionOption {
                        id: "_S".into(),
                        label: "Skip".into(),
                    },
                ],
            },
            Question {
                id: "priority".into(),
                question: "Priority".into(),
                options: vec![
                    QuestionOption {
                        id: "_1".into(),
                        label: "High".into(),
                    },
                    QuestionOption {
                        id: "_2".into(),
                        label: "Low".into(),
                    },
                    QuestionOption {
                        id: "_S".into(),
                        label: "Skip".into(),
                    },
                ],
            },
        ];
        state.update(Event::AskUserQuestion {
            request_id: "req-1".into(),
            questions,
        });
    }

    #[test]
    fn ask_user_question_opens_dialog() {
        let mut state = AppState::default();
        open_sample_question_dialog(&mut state);
        assert!(state.question_state().is_some());
        let dialog = state.open_dialog().expect("dialog should be open");
        let stack = dialog.panel_stack().expect("panel stack");
        let panel = stack.current().expect("panel");
        assert!(panel.title.contains("Question 1/2"));
    }

    #[test]
    fn ask_user_question_shows_first_question() {
        let mut state = AppState::default();
        open_sample_question_dialog(&mut state);
        let dialog = state.open_dialog().expect("dialog should be open");
        let stack = dialog.panel_stack().expect("panel stack");
        let panel = stack.current().expect("panel");
        assert!(panel.items.iter().any(|i| matches!(i, PanelItem::Header(h) if h.contains("Project Type"))));
    }

    #[test]
    fn question_answer_advances_to_next_question() {
        let mut state = AppState::default();
        open_sample_question_dialog(&mut state);

        // Answer first question
        state.update(Event::QuestionAnswer {
            request_id: "req-1".into(),
            option_id: "_1".into(),
        });

        let dialog = state.open_dialog().expect("dialog should still be open");
        let stack = dialog.panel_stack().expect("panel stack");
        let panel = stack.current().expect("panel");
        assert!(panel.title.contains("Question 2/2"));
    }

    #[test]
    fn question_answer_all_answered_shows_complete() {
        let mut state = AppState::default();
        open_sample_question_dialog(&mut state);

        // Answer both questions
        state.update(Event::QuestionAnswer {
            request_id: "req-1".into(),
            option_id: "_1".into(),
        });
        state.update(Event::QuestionAnswer {
            request_id: "req-1".into(),
            option_id: "_1".into(),
        });

        let dialog = state.open_dialog().expect("dialog should still be open");
        let stack = dialog.panel_stack().expect("panel stack");
        let panel = stack.current().expect("panel");
        assert!(panel.title.contains("complete") || panel.items.iter().any(|i| matches!(i, PanelItem::Header(h) if h.contains("answered"))));
    }

    #[test]
    fn question_skip_advances_without_answer() {
        let mut state = AppState::default();
        open_sample_question_dialog(&mut state);

        state.update(Event::QuestionSkip {
            request_id: "req-1".into(),
        });

        let dialog = state.open_dialog().expect("dialog should still be open");
        let stack = dialog.panel_stack().expect("panel stack");
        let panel = stack.current().expect("panel");
        assert!(panel.title.contains("Question 2/2"));
    }

    #[test]
    fn question_submit_clears_dialog() {
        let mut state = AppState::default();
        open_sample_question_dialog(&mut state);

        state.update(Event::QuestionSubmit {
            request_id: "req-1".into(),
        });

        assert!(state.open_dialog().is_none());
        assert!(state.question_state().is_none());
    }

    #[test]
    fn question_submit_shows_summary_message() {
        let mut state = AppState::default();
        open_sample_question_dialog(&mut state);

        state.update(Event::QuestionAnswer {
            request_id: "req-1".into(),
            option_id: "_1".into(),
        });

        state.update(Event::QuestionSubmit {
            request_id: "req-1".into(),
        });

        // System message should be added
        assert!(state.session().messages.iter().any(|m| {
            m.role == Role::System && m.content().contains("Answers collected")
        }));
    }

    #[test]
    fn question_pauses_streaming() {
        let mut state = AppState::default();
        state.agent_state_mut().streaming = true;
        open_sample_question_dialog(&mut state);
        assert!(
            !state.agent_state().streaming,
            "streaming should pause while waiting for answers"
        );
    }

    #[test]
    fn question_dialog_is_non_closable() {
        let mut state = AppState::default();
        open_sample_question_dialog(&mut state);
        let dialog = state.open_dialog().expect("dialog should be open");
        let stack = dialog.panel_stack().expect("panel stack");
        let panel = stack.current().expect("panel");
        assert!(panel.is_form(), "question panel should be a form");
    }
}
