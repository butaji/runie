use async_trait::async_trait;
use runie_core::{Message, ToolSchema, Event, ProviderError};
use futures::stream::BoxStream;
use futures::StreamExt;
use crate::Provider;
use genai::chat::{ChatMessage, ChatRequest};
use genai::Client;

pub struct GenAiProvider {
    client: Client,
    model: String,
}

impl GenAiProvider {
    pub fn new(model: String) -> Self {
        Self {
            client: Client::default(),
            model,
        }
    }

    fn convert_messages(messages: Vec<Message>) -> Vec<ChatMessage> {
        messages.into_iter().map(|msg| {
            match msg {
                Message::System { content } => ChatMessage::system(content),
                Message::User { content, .. } => ChatMessage::user(content),
                Message::Assistant { content, .. } => ChatMessage::assistant(content),
                Message::ToolResult { content, .. } => ChatMessage::user(content),
            }
        }).collect()
    }
}

#[async_trait]
impl Provider for GenAiProvider {
    fn name(&self) -> &str {
        "genai"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn chat(
        &self,
        messages: Vec<Message>,
        _tools: Vec<ToolSchema>,
    ) -> Result<BoxStream<'static, Event>, ProviderError> {
        let chat_req = ChatRequest::new(Self::convert_messages(messages));
        let model = self.model.clone();
        let client = self.client.clone();

        let stream = async_stream::stream! {
            yield Event::AgentStart {
                session_id: model.clone(),
                timestamp: chrono::Utc::now(),
            };

            match client.exec_chat_stream(&model, chat_req, None).await {
                Ok(chat_res) => {
                    let mut stream = chat_res.stream;
                    while let Some(event_result) = stream.next().await {
                        match event_result {
                            Ok(genai::chat::ChatStreamEvent::Chunk(chunk)) => {
                                yield Event::MessageDelta { content: chunk.content };
                            }
                            Ok(genai::chat::ChatStreamEvent::ToolCallChunk(tool_call)) => {
                                yield Event::ToolCallDelta {
                                    name: tool_call.tool_call.fn_name,
                                    arguments: tool_call.tool_call.fn_arguments.to_string(),
                                };
                            }
                            Err(e) => {
                                yield Event::Error { message: e.to_string() };
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    yield Event::Error { message: e.to_string() };
                }
            }

            yield Event::AgentEnd { timestamp: chrono::Utc::now() };
        };

        Ok(Box::pin(stream))
    }

    async fn chat_simple(
        &self,
        messages: Vec<Message>,
    ) -> Result<String, ProviderError> {
        let chat_req = ChatRequest::new(Self::convert_messages(messages));

        match self.client.exec_chat(&self.model, chat_req, None).await {
            Ok(res) => {
                let texts = res.texts();
                let text = texts.first().cloned().unwrap_or_default();
                Ok(text.to_string())
            }
            Err(e) => Err(ProviderError::ApiError(e.to_string())),
        }
    }
}