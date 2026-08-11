use runie_core::tools::UserQuestionBroker;

#[test]
fn clearing_history_removes_owned_traces() {
    let broker = UserQuestionBroker::default();
    broker
        .restore_traces_jsonl(r#"{"id":"1","question":"Deploy now?","outcome":"answered"}"#)
        .unwrap();
    assert_eq!(broker.traces().len(), 1);
    broker.clear_traces();
    assert!(broker.traces().is_empty());
}
