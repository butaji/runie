use runie_core::tools::{UserQuestionBroker, UserQuestionOption, UserQuestionRequest};

#[tokio::test]
async fn ask_many_publishes_ordered_questions_without_overlap() {
    let broker = UserQuestionBroker::default();
    let producer = {
        let broker = broker.clone();
        // OWNER: question_batch_test task is awaited below.
        tokio::spawn(async move {
            broker
                .ask_many([request("First?", "one"), request("Second?", "two")])
                .await
        })
    };
    let mut ids = Vec::new();
    while ids.len() < 2 {
        if let Some(pending) = broker.try_next() {
            assert_eq!(pending.request.question, ["First?", "Second?"][ids.len()]);
            let answer = if ids.is_empty() { "one" } else { "two" };
            broker
                .answer(&pending.id, serde_json::json!({"answer": answer}))
                .unwrap();
            ids.push(pending.id);
        } else {
            tokio::task::yield_now().await;
        }
    }
    assert_eq!(producer.await.unwrap().unwrap().len(), 2);
}

fn request(question: &str, label: &str) -> UserQuestionRequest {
    UserQuestionRequest {
        question: question.into(),
        header: None,
        body: None,
        options: vec![UserQuestionOption {
            id: None,
            label: label.into(),
            description: String::new(),
        }],
        allow_multiple: false,
    }
}
