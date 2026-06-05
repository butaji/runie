use crate::provider::Provider;
use crate::types::{AgentEvent, Message, ToolOutput, ToolError};
use async_trait::async_trait;
use futures::stream::BoxStream;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, args: serde_json::Value) -> Result<ToolOutput, ToolError>;
}

pub struct AgentLoop {
    provider: Arc<dyn Provider>,
    tools: Vec<Arc<dyn Tool>>,
    max_turns: usize,
}

impl AgentLoop {
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        Self {
            provider,
            tools: vec![],
            max_turns: 10,
        }
    }

    pub fn with_tools(mut self, tools: Vec<Arc<dyn Tool>>) -> Self {
        self.tools = tools;
        self
    }

    pub fn run(&self, messages: Vec<Message>) -> BoxStream<'static, AgentEvent> {
        let provider = self.provider.clone();
        let tools = self.tools.clone();
        let max_turns = self.max_turns;

        let (tx, rx) = mpsc::channel::<AgentEvent>(128);

        tokio::spawn(async move {
            let mut msgs = messages;
            for _turn in 0..max_turns {
                match provider.chat(msgs.clone()).await {
                    Ok((content, tool_calls)) => {
                        let _ = tx.send(AgentEvent::MessageStart {
                            role: "assistant".into(),
                        }).await;

                        for chunk in content.chars().collect::<Vec<_>>().chunks(8) {
                            let s: String = chunk.iter().collect();
                            let _ = tx
                                .send(AgentEvent::MessageDelta { content: s })
                                .await;
                            tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
                        }

                        let _ = tx.send(AgentEvent::MessageEnd).await;

                        if tool_calls.is_empty() {
                            msgs.push(Message::Assistant {
                                content,
                                tool_calls: vec![],
                            });
                            break;
                        }

                        msgs.push(Message::Assistant {
                            content: content.clone(),
                            tool_calls: tool_calls.clone(),
                        });

                        for tc in &tool_calls {
                            let _ = tx.send(AgentEvent::ToolCallStart {
                                id: tc.id.clone(),
                                name: tc.name.clone(),
                            }).await;

                            let result = if let Some(tool) = tools.iter().find(|t| t.name() == tc.name) {
                                match tool.execute(tc.arguments.clone()).await {
                                    Ok(out) => out.content,
                                    Err(e) => format!("Error: {}", e),
                                }
                            } else {
                                format!("Tool '{}' not found", tc.name)
                            };

                            let _ = tx.send(AgentEvent::ToolCallEnd {
                                id: tc.id.clone(),
                                result: result.clone(),
                            }).await;

                            msgs.push(Message::ToolResult {
                                tool_call_id: tc.id.clone(),
                                content: result,
                                is_error: false,
                            });
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(AgentEvent::Error { message: e.to_string() })
                            .await;
                        break;
                    }
                }
            }
        });

        Box::pin(ReceiverStream::new(rx))
    }
}
