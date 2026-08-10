use super::UserQuestionRequest;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingUserQuestion {
    pub id: String,
    pub request: UserQuestionRequest,
}

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

const MAX_QUESTION_TRACES: usize = 128;

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
                        options: vec![
                            UserQuestionOption {
                                label: "yes".into(),
                                description: String::new(),
                            },
                            UserQuestionOption {
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
        broker
            .answer(&pending.id, serde_json::json!({"answer": "yes"}))
            .unwrap();
        assert_eq!(waiter.await.unwrap().unwrap()["answer"], "yes");
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
                        options: vec![UserQuestionOption {
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
        let error = broker
            .answer(&pending.id, serde_json::json!({"answer": "no"}))
            .unwrap_err();
        assert!(error.contains("not offered") && broker.traces()[0].outcome == "rejected");
        assert!(broker.traces()[0].error.is_some());
        assert_eq!(
            broker.traces()[0].attempted_answer,
            Some(serde_json::json!({"answer": "no"}))
        );
        broker
            .answer(&pending.id, serde_json::json!({"answer": "yes"}))
            .unwrap();
        assert_eq!(waiter.await.unwrap().unwrap()["answer"], "yes");
        assert_eq!(broker.traces()[1].outcome, "answered");
        assert!(broker.traces()[1].attempted_answer.is_none());
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
}
