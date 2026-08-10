//! Replay HTTP integration coverage kept separate from the fixture matrix.

mod common;

use std::{path::Path, sync::Arc};

use runie_core::{
    provider::{HttpActor, ReplayHttpActor, ReplayProvider},
    types::{AgentContext, AgentMessage, UserContent, UserMessage},
};

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/traces")
}

#[tokio::test]
async fn replay_http_actor_feeds_provider_before_core_loop() {
    let path = root().join("openai/opencode_go_deepseek_v4_flash_simple.sse");
    let http: Arc<dyn HttpActor> = Arc::new(ReplayHttpActor::from_sse(path).await.unwrap());
    let provider = Arc::new(ReplayProvider::from_http(http).await.unwrap());
    let test = common::TestLoopBuilder::new(provider).build();
    let output = test
        .actor
        .prompt(
            vec![AgentMessage::User(UserMessage {
                content: vec![UserContent::Text {
                    text: "http replay".into(),
                }],
                timestamp: 1,
            })],
            AgentContext::default(),
        )
        .await
        .unwrap();
    assert!(output
        .iter()
        .any(|message| matches!(message, AgentMessage::Assistant(_))));
}
