use runie_core::tools::{
    question_history_page, question_history_rows, UserQuestionHistoryRow, UserQuestionTrace,
};

#[test]
fn question_history_queries_newest_filtered_traces() {
    let traces = vec![
        UserQuestionTrace {
            id: "1".into(),
            question: "Deploy now?".into(),
            outcome: "cancelled".into(),
            attempted_answer: None,
            error: None,
        },
        UserQuestionTrace {
            id: "2".into(),
            question: "Continue deploy?".into(),
            outcome: "answered".into(),
            attempted_answer: None,
            error: None,
        },
    ];
    let rows = question_history_rows(&traces, "deploy", Some("answered"), 8);
    assert_eq!(
        rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
        ["2"]
    );
    assert_eq!(rows[0].question, "Continue deploy?");
    let page = question_history_page(&traces, "deploy", None, 0, 1);
    assert_eq!(page.rows.len(), 1);
    assert!(page.has_more);
}

#[test]
fn question_history_row_owns_terminal_projection() {
    let row = UserQuestionHistoryRow {
        id: "q-1".into(),
        question: "Deploy now?".into(),
        outcome: "rejected".into(),
        detail: Some("invalid option".into()),
    };
    assert_eq!(
        row.terminal_line(),
        "q-1 · rejected · Deploy now? · invalid option"
    );
}
