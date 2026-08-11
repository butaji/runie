use super::UserQuestionRequest;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingUserQuestion {
    pub id: String,
    pub request: UserQuestionRequest,
}

include!("question_pending.inc");
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UserQuestionTrace {
    pub id: String,
    pub question: String,
    pub outcome: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempted_answer: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UserQuestionHistoryRow {
    pub id: String,
    pub question: String,
    pub outcome: String,
    pub detail: Option<String>,
}

impl UserQuestionHistoryRow {
    pub fn terminal_line(&self) -> String {
        format!(
            "{} · {} · {}{}",
            self.id,
            self.outcome,
            self.question,
            self.detail
                .as_deref()
                .map(|detail| format!(" · {detail}"))
                .unwrap_or_default()
        )
    }
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UserQuestionHistoryPage {
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
    pub rows: Vec<UserQuestionHistoryRow>,
}
impl From<UserQuestionTrace> for UserQuestionHistoryRow {
    fn from(trace: UserQuestionTrace) -> Self {
        Self {
            id: trace.id,
            question: trace.question,
            outcome: trace.outcome,
            detail: trace.error,
        }
    }
}
pub fn question_history_rows(
    traces: &[UserQuestionTrace],
    text: &str,
    outcome: Option<&str>,
    limit: usize,
) -> Vec<UserQuestionHistoryRow> {
    query_question_history_page(traces, text, outcome, 0, limit)
        .into_iter()
        .map(Into::into)
        .collect()
}
pub fn question_history_rows_page(
    traces: &[UserQuestionTrace],
    text: &str,
    outcome: Option<&str>,
    offset: usize,
    limit: usize,
) -> Vec<UserQuestionHistoryRow> {
    query_question_history_page(traces, text, outcome, offset, limit)
        .into_iter()
        .map(Into::into)
        .collect()
}
pub fn question_history_page(
    traces: &[UserQuestionTrace],
    text: &str,
    outcome: Option<&str>,
    offset: usize,
    limit: usize,
) -> UserQuestionHistoryPage {
    let mut rows =
        question_history_rows_page(traces, text, outcome, offset, limit.saturating_add(1));
    let has_more = rows.len() > limit;
    rows.truncate(limit);
    UserQuestionHistoryPage {
        offset,
        limit,
        has_more,
        rows,
    }
}
pub fn encode_question_traces(traces: &[UserQuestionTrace]) -> Result<String, serde_json::Error> {
    traces
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()
        .map(|lines| format!("{}\n", lines.join("\n")))
}
pub fn decode_question_traces(input: &str) -> Result<Vec<UserQuestionTrace>, serde_json::Error> {
    input
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str)
        .collect()
}
const MAX_QUESTION_TRACES: usize = 128;
pub fn query_question_history(
    traces: &[UserQuestionTrace],
    text: &str,
    outcome: Option<&str>,
    limit: usize,
) -> Vec<UserQuestionTrace> {
    query_question_history_page(traces, text, outcome, 0, limit)
}
pub fn query_question_history_page(
    traces: &[UserQuestionTrace],
    text: &str,
    outcome: Option<&str>,
    offset: usize,
    limit: usize,
) -> Vec<UserQuestionTrace> {
    let text = text.trim().to_ascii_lowercase();
    traces
        .iter()
        .rev()
        .filter(|trace| outcome.is_none_or(|value| trace.outcome == value))
        .filter(|trace| text.is_empty() || trace.question.to_ascii_lowercase().contains(&text))
        .skip(offset.min(MAX_QUESTION_TRACES))
        .take(limit.min(MAX_QUESTION_TRACES))
        .cloned()
        .collect()
}
#[derive(Clone)]
pub struct UserQuestionBroker {
    tx: mpsc::UnboundedSender<PendingUserQuestion>,
    rx: Arc<Mutex<mpsc::UnboundedReceiver<PendingUserQuestion>>>,
    answers: Arc<Mutex<HashMap<String, oneshot::Sender<serde_json::Value>>>>,
    requests: Arc<Mutex<HashMap<String, UserQuestionRequest>>>,
    traces: Arc<Mutex<Vec<UserQuestionTrace>>>,
    next_id: Arc<std::sync::atomic::AtomicU64>,
}
impl Default for UserQuestionBroker {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            tx,
            rx: Arc::new(Mutex::new(rx)),
            answers: Arc::new(Mutex::new(HashMap::new())),
            requests: Arc::new(Mutex::new(HashMap::new())),
            traces: Arc::new(Mutex::new(Vec::new())),
            next_id: Default::default(),
        }
    }
}
impl UserQuestionBroker {
    pub async fn ask(&self, request: UserQuestionRequest) -> Result<serde_json::Value, String> {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            .to_string();
        let (answer_tx, answer_rx) = oneshot::channel();
        self.answers
            .lock()
            .expect("question answers lock")
            .insert(id.clone(), answer_tx);
        self.requests
            .lock()
            .expect("question requests lock")
            .insert(id.clone(), request.clone());
        if self
            .tx
            .send(PendingUserQuestion {
                id: id.clone(),
                request,
            })
            .is_err()
        {
            self.answers
                .lock()
                .expect("question answers lock")
                .remove(&id);
            self.requests
                .lock()
                .expect("question requests lock")
                .remove(&id);
            return Err("question UI is closed".to_owned());
        }
        answer_rx
            .await
            .map_err(|_| "question was cancelled".to_owned())
    }
    pub fn try_next(&self) -> Option<PendingUserQuestion> {
        self.rx.lock().expect("question queue lock").try_recv().ok()
    }
    pub fn traces(&self) -> Vec<UserQuestionTrace> {
        self.traces.lock().expect("question traces lock").clone()
    }
    pub fn clear_traces(&self) {
        self.traces.lock().expect("question traces lock").clear();
    }
    pub fn export_traces_jsonl(&self) -> Result<String, serde_json::Error> {
        encode_question_traces(&self.traces())
    }
    pub fn restore_traces_jsonl(&self, input: &str) -> Result<(), serde_json::Error> {
        let traces = decode_question_traces(input)?;
        let mut owned = self.traces.lock().expect("question traces lock");
        owned.extend(traces);
        if owned.len() > MAX_QUESTION_TRACES {
            let keep_from = owned.len() - MAX_QUESTION_TRACES;
            owned.drain(..keep_from);
        }
        Ok(())
    }
    pub fn answer(&self, id: &str, value: serde_json::Value) -> Result<(), String> {
        let request = self
            .requests
            .lock()
            .expect("question requests lock")
            .get(id)
            .cloned()
            .ok_or_else(|| "question is no longer pending".to_owned())?;
        if let Err(error) = validate_answer(&request, &value) {
            self.record_trace(
                id,
                &request.question,
                "rejected",
                Some(value),
                Some(error.clone()),
            );
            return Err(error);
        }
        self.record_trace(id, &request.question, "answered", None, None);
        self.requests
            .lock()
            .expect("question requests lock")
            .remove(id);
        self.answers
            .lock()
            .expect("question answers lock")
            .remove(id)
            .ok_or_else(|| "question is no longer pending".to_owned())?
            .send(value)
            .map_err(|_| "question waiter is closed".to_owned())
    }
    pub fn cancel(&self, id: &str) -> Result<(), String> {
        let request = self
            .requests
            .lock()
            .expect("question requests lock")
            .remove(id)
            .ok_or_else(|| "question is no longer pending".to_owned())?;
        self.record_trace(id, &request.question, "cancelled", None, None);
        self.answers
            .lock()
            .expect("question answers lock")
            .remove(id)
            .ok_or_else(|| "question is no longer pending".to_owned())?
            .send(serde_json::json!({"cancelled": true}))
            .map_err(|_| "question waiter is closed".to_owned())
    }
    fn record_trace(
        &self,
        id: &str,
        question: &str,
        outcome: &str,
        attempted_answer: Option<serde_json::Value>,
        error: Option<String>,
    ) {
        let mut traces = self.traces.lock().expect("question traces lock");
        traces.push(UserQuestionTrace {
            id: id.to_owned(),
            question: question.to_owned(),
            outcome: outcome.to_owned(),
            attempted_answer,
            error,
        });
        if traces.len() > MAX_QUESTION_TRACES {
            traces.remove(0);
        }
    }
}
fn validate_answer(request: &UserQuestionRequest, value: &serde_json::Value) -> Result<(), String> {
    if value.get("cancelled").and_then(serde_json::Value::as_bool) == Some(true) {
        return Err("cancel answers must use cancel()".into());
    }
    let labels = answer_labels(request, value)?;
    if labels.is_empty() {
        return Err("answer must select at least one option".into());
    }
    if !labels
        .iter()
        .all(|label| request.options.iter().any(|option| option.label == *label))
    {
        return Err(format!(
            "answer contains an option not offered by the question: {labels:?}"
        ));
    }
    if !request.allow_multiple && labels.len() != 1 {
        return Err("single-select questions accept exactly one option".into());
    }
    Ok(())
}
fn answer_labels<'a>(
    request: &UserQuestionRequest,
    value: &'a serde_json::Value,
) -> Result<Vec<&'a str>, String> {
    if request.allow_multiple {
        return value
            .get("answers")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| "answer must contain an `answers` array".to_owned())?
            .iter()
            .map(|item| {
                item.as_str()
                    .ok_or_else(|| "answers must contain strings".to_owned())
            })
            .collect();
    }
    Ok(vec![value
        .get("answer")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            "answer must contain an `answer` string".to_owned()
        })?])
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::UserQuestionOption;
    #[tokio::test]
    async fn broker_round_trips_an_answer() {
        let broker = UserQuestionBroker::default();
        let waiter = {
            let broker = broker.clone();
            // OWNER: broker_round_trips_an_answer test task is awaited below.
            tokio::spawn(async move {
                broker
                    .ask(UserQuestionRequest {
                        question: "Continue?".into(),
                        header: None,
                        body: None,
                        options: vec![
                            UserQuestionOption {
                                id: None,
                                label: "yes".into(),
                                description: String::new(),
                            },
                            UserQuestionOption {
                                id: None,
                                label: "no".into(),
                                description: String::new(),
                            },
                        ],
                        allow_multiple: false,
                    })
                    .await
            })
        };
        let pending = loop {
            if let Some(value) = broker.try_next() {
                break value;
            }
            tokio::task::yield_now().await;
        };
        assert_pending_line(&broker);
        broker
            .answer(&pending.id, serde_json::json!({"answer": "yes"}))
            .unwrap();
        assert_eq!(waiter.await.unwrap().unwrap()["answer"], "yes");
    }

    fn assert_pending_line(broker: &UserQuestionBroker) {
        assert_eq!(
            broker.pending()[0].terminal_line(),
            "0 · pending · Continue?"
        );
    }
    #[tokio::test]
    async fn broker_cancellation_is_a_typed_terminal_answer() {
        let broker = UserQuestionBroker::default();
        let waiter = {
            let broker = broker.clone();
            // OWNER: broker_cancellation_is_a_typed_terminal_answer test task is awaited below.
            tokio::spawn(async move {
                broker
                    .ask(UserQuestionRequest {
                        question: "Continue?".into(),
                        header: None,
                        body: None,
                        options: vec![],
                        allow_multiple: false,
                    })
                    .await
            })
        };
        let pending = loop {
            if let Some(value) = broker.try_next() {
                break value;
            }
            tokio::task::yield_now().await;
        };
        broker.cancel(&pending.id).unwrap();
        assert_eq!(waiter.await.unwrap().unwrap()["cancelled"], true);
    }
    #[tokio::test]
    async fn broker_rejects_answers_not_in_the_question_options() {
        let broker = UserQuestionBroker::default();
        let waiter = {
            let broker = broker.clone();
            // OWNER: broker_rejects_answers_not_in_question_options test task is awaited below.
            tokio::spawn(async move {
                broker
                    .ask(UserQuestionRequest {
                        question: "Continue?".into(),
                        header: None,
                        body: None,
                        options: vec![UserQuestionOption {
                            id: None,
                            label: "yes".into(),
                            description: String::new(),
                        }],
                        allow_multiple: false,
                    })
                    .await
            })
        };
        let pending = loop {
            if let Some(value) = broker.try_next() {
                break value;
            }
            tokio::task::yield_now().await;
        };
        assert!(
            broker
                .answer(&pending.id, serde_json::json!({"answer": "no"}))
                .unwrap_err()
                .contains("not offered")
                && broker.traces()[0].outcome == "rejected"
        );
        broker
            .answer(&pending.id, serde_json::json!({"answer": "yes"}))
            .unwrap();
        assert_eq!(waiter.await.unwrap().unwrap()["answer"], "yes");
        assert_eq!(broker.traces()[1].outcome, "answered");
    }
    #[test]
    fn question_trace_deserializes_legacy_error_only_records() {
        let trace: UserQuestionTrace = serde_json::from_value(serde_json::json!({
            "id": "7",
            "question": "Continue?",
            "outcome": "rejected",
            "error": "invalid answer"
        }))
        .unwrap();
        assert!(trace.attempted_answer.is_none());
        assert_eq!(trace.error.as_deref(), Some("invalid answer"));
    }
    #[test]
    fn question_traces_round_trip_as_session_jsonl() {
        let traces = vec![UserQuestionTrace {
            id: "7".into(),
            question: "Continue?".into(),
            outcome: "answered".into(),
            attempted_answer: None,
            error: None,
        }];
        let jsonl = encode_question_traces(&traces).unwrap();
        assert_eq!(decode_question_traces(&jsonl).unwrap(), traces);
    }
    #[test]
    fn broker_restores_only_the_bounded_trace_tail() {
        let traces = (0..=MAX_QUESTION_TRACES)
            .map(|index| UserQuestionTrace {
                id: index.to_string(),
                question: "Continue?".into(),
                outcome: "cancelled".into(),
                attempted_answer: None,
                error: None,
            })
            .collect::<Vec<_>>();
        let broker = UserQuestionBroker::default();
        broker
            .restore_traces_jsonl(&encode_question_traces(&traces).unwrap())
            .unwrap();
        let restored = broker.traces();
        assert_eq!(restored.len(), MAX_QUESTION_TRACES);
        assert_eq!(restored[0].id, "1");
    }
}
