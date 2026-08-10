use super::UserQuestionRequest;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingUserQuestion {
    pub id: String,
    pub request: UserQuestionRequest,
}

#[derive(Clone)]
pub struct UserQuestionBroker {
    tx: mpsc::UnboundedSender<PendingUserQuestion>,
    rx: Arc<Mutex<mpsc::UnboundedReceiver<PendingUserQuestion>>>,
    answers: Arc<Mutex<HashMap<String, oneshot::Sender<serde_json::Value>>>>,
    next_id: Arc<std::sync::atomic::AtomicU64>,
}

impl Default for UserQuestionBroker {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            tx,
            rx: Arc::new(Mutex::new(rx)),
            answers: Arc::new(Mutex::new(HashMap::new())),
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
            return Err("question UI is closed".to_owned());
        }
        answer_rx
            .await
            .map_err(|_| "question was cancelled".to_owned())
    }

    pub fn try_next(&self) -> Option<PendingUserQuestion> {
        self.rx.lock().expect("question queue lock").try_recv().ok()
    }

    pub fn answer(&self, id: &str, value: serde_json::Value) -> Result<(), String> {
        self.answers
            .lock()
            .expect("question answers lock")
            .remove(id)
            .ok_or_else(|| "question is no longer pending".to_owned())?
            .send(value)
            .map_err(|_| "question waiter is closed".to_owned())
    }

    pub fn cancel(&self, id: &str) -> Result<(), String> {
        self.answer(id, serde_json::json!({"cancelled": true}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
