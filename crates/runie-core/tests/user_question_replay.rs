use runie_core::tools::{AskUserQuestionTool, UserQuestionRequest};
use runie_core::types::AgentTool;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct QuestionTrace {
    question: UserQuestionRequest,
    answer: serde_json::Value,
}

#[test]
fn yaml_question_trace_replays_request_and_answer_as_data() {
    let trace: QuestionTrace = serde_yaml::from_str(include_str!("fixtures/user-question.yaml"))
        .expect("question fixture is valid YAML");
    let tool = AskUserQuestionTool;
    let args = serde_json::to_value(&trace.question).expect("request serializes");
    tool.validate_arguments(&args).expect("request validates");
    assert_eq!(trace.answer["answers"][0], "Unit tests");
    assert_eq!(trace.answer["answers"][1], "Replay tests");
}
