use async_stream::stream;
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

    #[must_use]
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

        let stream = stream! {
            yield Event::AgentStart {
                session_id: model.clone(),
                timestamp: chrono::Utc::now(),
            };

            let events = yield_events_from_chat(&client, &model, chat_req).await;
            for event in events {
                yield event;
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

async fn yield_events_from_chat(
    client: &Client,
    model: &str,
    chat_req: ChatRequest,
) -> Vec<Event> {
    let mut events = Vec::new();
    match client.exec_chat_stream(model, chat_req, None).await {
        Ok(chat_res) => {
            let mut stream = chat_res.stream;
            let mut tool_call_index = 0usize;
            while let Some(event_result) = stream.next().await {
                match event_result {
                    Ok(genai::chat::ChatStreamEvent::Chunk(chunk)) => {
                        events.push(Event::MessageDelta { content: chunk.content });
                    }
                    Ok(genai::chat::ChatStreamEvent::ToolCallChunk(tool_call)) => {
                        let id = format!("call_{}", tool_call_index);
                        tool_call_index += 1;
                        events.push(Event::ToolCallDelta {
                            id,
                            name: tool_call.tool_call.fn_name,
                            arguments: tool_call.tool_call.fn_arguments.to_string(),
                        });
                    }
                    Err(e) => {
                        events.push(Event::Error { message: e.to_string() });
                        break;
                    }
                    _ => {}
                }
            }
        }
        Err(e) => {
            events.push(Event::Error { message: e.to_string() });
        }
    }
    events
}
