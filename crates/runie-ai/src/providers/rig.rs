use rig_core::providers::{
    openai, anthropic, gemini, cohere, deepseek, groq, openrouter,
    huggingface, xai, azure, moonshot, perplexity, ollama, hyperbolic,
    mistral, together, voyageai, zai, minimax, mira, galadriel, llamafile,
};
use rig_core::client::CompletionClient;
use rig_core::completion::message::{
    AssistantContent, Message as RigMessage, Text, ToolCall,
    ToolResult, ToolResultContent, UserContent,
};
use rig_core::completion::request::ToolDefinition;
use rig_core::completion::{CompletionModel, CompletionRequest};
use rig_core::OneOrMany;
use runie_core::{Event, Message, ToolSchema};
use crate::provider::ProviderError;
use futures::stream::BoxStream;
use futures::StreamExt;

/// Provider enum dispatching to rig's provider clients.
/// Each variant holds (client, model_name).
/// The enum exists because rig's CompletionClient trait returns `impl CompletionModel`
/// with lifetimes tied to self - we cannot use trait objects here.
pub enum RigProvider {
    OpenAi(openai::Client, String),
    Anthropic(anthropic::Client, String),
    Gemini(gemini::Client, String),
    Cohere(cohere::Client, String),
    Mistral(mistral::Client, String),
    DeepSeek(deepseek::Client, String),
    Groq(groq::Client, String),
    OpenRouter(openrouter::Client, String),
    HuggingFace(huggingface::Client, String),
    Xai(xai::Client, String),
    Azure(azure::Client, String),
    Moonshot(moonshot::Client, String),
    Perplexity(perplexity::Client, String),
    Ollama(ollama::Client, String),
    Hyperbolic(hyperbolic::Client, String),
    Together(together::Client, String),
    VoyageAi(voyageai::Client, String),
    Zai(zai::Client, String),
    Minimax(minimax::Client, String),
    Mira(mira::Client, String),
    Galadriel(galadriel::Client, String),
    Llamafile(llamafile::Client, String),
}

/// Macro to define provider name and model accessor methods concisely.
/// Reduces boilerplate for the enum's 22 variants.
macro_rules! define_provider_accessors {
    ($($variant:ident => $name:literal),* $(,)?) => {
        impl RigProvider {
            pub fn name(&self) -> &str {
                match self {
                    $(Self::$variant(_, _) => $name),*
                }
            }

            pub fn model(&self) -> &str {
                match self {
                    $(Self::$variant(_, m) => m.as_str()),*
                }
            }
        }
    };
}

define_provider_accessors!(
    OpenAi => "openai",
    Anthropic => "anthropic",
    Gemini => "gemini",
    Cohere => "cohere",
    Mistral => "mistral",
    DeepSeek => "deepseek",
    Groq => "groq",
    OpenRouter => "openrouter",
    HuggingFace => "huggingface",
    Xai => "xai",
    Azure => "azure",
    Moonshot => "moonshot",
    Perplexity => "perplexity",
    Ollama => "ollama",
    Hyperbolic => "hyperbolic",
    Together => "together",
    VoyageAi => "voyageai",
    Zai => "zai",
    Minimax => "minimax",
    Mira => "mira",
    Galadriel => "galadriel",
    Llamafile => "llamafile",
);

/// Macro for dispatching to concrete RigProvider variants.
/// Generates match arms for each provider variant, avoiding generic type erasure issues.
/// Note: VoyageAI is handled separately before calling this macro (it doesn't support chat).
#[macro_export]
macro_rules! with_rig_provider {
    ($provider:expr, $client:ident, $model:ident, $body:expr) => {
        match $provider {
            RigProvider::OpenAi($client, $model) => $body,
            RigProvider::Anthropic($client, $model) => $body,
            RigProvider::Gemini($client, $model) => $body,
            RigProvider::Cohere($client, $model) => $body,
            RigProvider::Mistral($client, $model) => $body,
            RigProvider::DeepSeek($client, $model) => $body,
            RigProvider::Groq($client, $model) => $body,
            RigProvider::OpenRouter($client, $model) => $body,
            RigProvider::HuggingFace($client, $model) => $body,
            RigProvider::Xai($client, $model) => $body,
            RigProvider::Azure($client, $model) => $body,
            RigProvider::Moonshot($client, $model) => $body,
            RigProvider::Perplexity($client, $model) => $body,
            RigProvider::Ollama($client, $model) => $body,
            RigProvider::Hyperbolic($client, $model) => $body,
            RigProvider::Together($client, $model) => $body,
            RigProvider::Zai($client, $model) => $body,
            RigProvider::Minimax($client, $model) => $body,
            RigProvider::Mira($client, $model) => $body,
            RigProvider::Galadriel($client, $model) => $body,
            RigProvider::Llamafile($client, $model) => $body,
            RigProvider::VoyageAi(_, _) => {
                unreachable!("VoyageAI is handled before macro dispatch")
            }
        }
    };
}

impl RigProvider {
    pub fn new(provider: &str, api_key: &str, model: &str) -> Result<Self, String> {
        let model = model.to_string();
        match provider.to_lowercase().as_str() {
            "openai" => Ok(Self::OpenAi(openai::Client::new(api_key).map_err(|e| e.to_string())?, model)),
            "anthropic" => Ok(Self::Anthropic(anthropic::Client::new(api_key).map_err(|e| e.to_string())?, model)),
            "gemini" => Ok(Self::Gemini(gemini::Client::new(api_key).map_err(|e| e.to_string())?, model)),
            "cohere" => Ok(Self::Cohere(cohere::Client::new(api_key).map_err(|e| e.to_string())?, model)),
            "mistral" => Ok(Self::Mistral(mistral::Client::new(api_key).map_err(|e| e.to_string())?, model)),
            "deepseek" => Ok(Self::DeepSeek(deepseek::Client::new(api_key).map_err(|e| e.to_string())?, model)),
            "groq" => Ok(Self::Groq(groq::Client::new(api_key).map_err(|e| e.to_string())?, model)),
            "openrouter" => Ok(Self::OpenRouter(openrouter::Client::new(api_key).map_err(|e| e.to_string())?, model)),
            "huggingface" => Ok(Self::HuggingFace(huggingface::Client::new(api_key).map_err(|e| e.to_string())?, model)),
            "xai" => Ok(Self::Xai(xai::Client::new(api_key).map_err(|e| e.to_string())?, model)),
            "azure" => Ok(Self::Azure(azure::Client::new(api_key).map_err(|e| e.to_string())?, model)),
            "moonshot" => Ok(Self::Moonshot(moonshot::Client::new(api_key).map_err(|e| e.to_string())?, model)),
            "perplexity" => Ok(Self::Perplexity(perplexity::Client::new(api_key).map_err(|e| e.to_string())?, model)),
            "ollama" => Ok(Self::Ollama(ollama::Client::new(api_key).map_err(|e| e.to_string())?, model)),
            "hyperbolic" => Ok(Self::Hyperbolic(hyperbolic::Client::new(api_key).map_err(|e| e.to_string())?, model)),
            "together" => Ok(Self::Together(together::Client::new(api_key).map_err(|e| e.to_string())?, model)),
            "voyageai" => Ok(Self::VoyageAi(voyageai::Client::new(api_key).map_err(|e| e.to_string())?, model)),
            "zai" => Ok(Self::Zai(zai::Client::new(api_key).map_err(|e| e.to_string())?, model)),
            "minimax" => Ok(Self::Minimax(minimax::Client::new(api_key).map_err(|e| e.to_string())?, model)),
            "mira" => Ok(Self::Mira(mira::Client::new(api_key).map_err(|e| e.to_string())?, model)),
            "galadriel" => Ok(Self::Galadriel(galadriel::Client::new(api_key).map_err(|e| e.to_string())?, model)),
            "llamafile" => Ok(Self::Llamafile(llamafile::Client::from_url("http://localhost:8080").map_err(|e| e.to_string())?, model)),
            _ => Err(format!("Unknown provider: {}", provider)),
        }
    }

    pub fn supports_tools(&self) -> bool {
        true
    }

    pub fn supports_vision(&self) -> bool {
        let model = self.model().to_lowercase();
        model.contains("4o") || model.contains("claude-3") || model.contains("gemini-1.5") || model.contains("vision")
    }

    pub fn max_context_tokens(&self) -> usize {
        let model = self.model().to_lowercase();
        match self.name() {
            "openai" => {
                if model.contains("4o") || model.contains("4-turbo") {
                    128_000
                } else if model.contains("4") {
                    8_192
                } else {
                    4_096
                }
            }
            "anthropic" => {
                if model.contains("claude-3-5") || model.contains("claude-3-opus") {
                    200_000
                } else if model.contains("claude-3") {
                    180_000
                } else {
                    100_000
                }
            }
            "gemini" => {
                if model.contains("1.5") || model.contains("2.0") {
                    1_000_000
                } else {
                    32_000
                }
            }
            "cohere" => 4_096,
            "mistral" => 32_000,
            "deepseek" => 64_000,
            "groq" => 8_192,
            "openrouter" => 128_000,
            "huggingface" => 4_096,
            "xai" => 128_000,
            "azure" => {
                if model.contains("4o") || model.contains("4-turbo") {
                    128_000
                } else {
                    8_192
                }
            }
            "moonshot" => 32_000,
            "perplexity" => 128_000,
            "ollama" => 4_096,
            "hyperbolic" => 128_000,
            "together" => 128_000,
            "voyageai" | "zai" | "minimax" | "mira" | "galadriel" | "llamafile" => 128_000,
            _ => 128_000,
        }
    }

    #[allow(dead_code)]
    pub fn with_base_url(self, _url: String) -> Self {
        self
    }

    #[allow(dead_code)]
    pub fn is_supported(&self) -> bool {
        true
    }
}

fn convert_messages(messages: Vec<Message>) -> Vec<RigMessage> {
    messages.into_iter().map(|m| match m {
        Message::System { content } => RigMessage::System { content },
        Message::User { content, .. } => RigMessage::User {
            content: OneOrMany::one(UserContent::Text(Text { text: content })),
        },
        Message::Assistant { content, tool_calls, .. } => {
            let mut contents = Vec::new();
            if !content.is_empty() {
                contents.push(AssistantContent::Text(Text { text: content }));
            }
            for tc in tool_calls {
                contents.push(AssistantContent::ToolCall(ToolCall {
                    id: tc.id,
                    call_id: None,
                    function: rig_core::completion::message::ToolFunction {
                        name: tc.name,
                        arguments: tc.arguments,
                    },
                    signature: None,
                    additional_params: None,
                }));
            }
            if contents.is_empty() {
                contents.push(AssistantContent::Text(Text { text: String::new() }));
            }
            let content = OneOrMany::many(contents).unwrap_or_else(|_|
                OneOrMany::one(AssistantContent::Text(Text { text: String::new() }))
            );
            RigMessage::Assistant { id: None, content }
        }
        Message::ToolResult { tool_call_id, content, .. } => RigMessage::User {
            content: OneOrMany::one(UserContent::ToolResult(ToolResult {
                id: tool_call_id,
                call_id: None,
                content: OneOrMany::one(ToolResultContent::Text(Text { text: content })),
            }))
        },
    }).collect()
}

fn convert_tools(tools: Vec<ToolSchema>) -> Vec<ToolDefinition> {
    crate::helpers::tool_schemas_to_rig_definitions(&tools)
}

fn build_request<M: CompletionModel>(
    model: &M,
    messages: Vec<RigMessage>,
    tools: Vec<ToolDefinition>,
) -> Result<CompletionRequest, ProviderError> {
    if messages.is_empty() {
        return Err(ProviderError::InvalidResponse("No messages provided".to_string()));
    }

    let prompt = messages.last().cloned().unwrap();
    let chat_history = messages.into_iter().rev().skip(1).rev().collect::<Vec<_>>();

    let mut builder = model.completion_request(prompt);
    if !chat_history.is_empty() {
        builder = builder.messages(chat_history);
    }
    if !tools.is_empty() {
        builder = builder.tools(tools);
    }

    Ok(builder.build())
}

/// Converts a rig streaming completion response to our Event stream.
async fn convert_stream<R>(
    mut stream: rig_core::streaming::StreamingCompletionResponse<R>
) -> Result<BoxStream<'static, Event>, ProviderError>
where
    R: Clone + Unpin + rig_core::completion::GetTokenUsage + Send + 'static,
{
    use rig_core::streaming::StreamedAssistantContent;

    let event_stream = async_stream::stream! {
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(StreamedAssistantContent::Text(text)) => {
                    yield Event::MessageDelta { content: text.text };
                }
                Ok(StreamedAssistantContent::ToolCall { tool_call, .. }) => {
                    let args = serde_json::to_string(&tool_call.function.arguments).unwrap_or_default();
                    yield Event::ToolCallDelta { name: tool_call.function.name, arguments: args };
                }
                Ok(StreamedAssistantContent::ToolCallDelta { id: _, content, .. }) => {
                    match content {
                        rig_core::streaming::ToolCallDeltaContent::Name(name) => {
                            yield Event::ToolCallDelta { name, arguments: String::new() };
                        }
                        rig_core::streaming::ToolCallDeltaContent::Delta(delta) => {
                            yield Event::ToolCallDelta { name: String::new(), arguments: delta };
                        }
                    }
                }
                Ok(StreamedAssistantContent::Reasoning(r)) => {
                    yield Event::ThinkingDelta { content: r.display_text() };
                }
                Ok(StreamedAssistantContent::ReasoningDelta { reasoning, .. }) => {
                    yield Event::ThinkingDelta { content: reasoning };
                }
                Ok(_) => {} // FinalResponse, etc.
                Err(e) => {
                    yield Event::Error { message: e.to_string() };
                }
            }
        }
    };

    Ok(Box::pin(event_stream))
}

/// Helper async fn to stream from any CompletionClient-implementing client.
/// This avoids duplicating the same 4-line pattern for each provider.
async fn stream_from_client<C: CompletionClient>(
    client: &C,
    model_name: &str,
    rig_messages: Vec<RigMessage>,
    rig_tools: Vec<ToolDefinition>,
) -> Result<BoxStream<'static, Event>, ProviderError>
where
    <<C as CompletionClient>::CompletionModel as rig_core::completion::CompletionModel>::StreamingResponse: 'static,
{
    let model = client.completion_model(model_name);
    let request = build_request(&model, rig_messages, rig_tools)?;
    let stream = model
        .stream(request)
        .await
        .map_err(|e| ProviderError::ApiError(e.to_string()))?;
    convert_stream(stream).await
}

#[async_trait::async_trait]
impl crate::Provider for RigProvider {
    fn name(&self) -> &str {
        self.name()
    }

    fn model(&self) -> &str {
        self.model()
    }

    fn supports_tools(&self) -> bool {
        self.supports_tools()
    }

    fn supports_vision(&self) -> bool {
        self.supports_vision()
    }

    fn max_context_tokens(&self) -> usize {
        self.max_context_tokens()
    }

    async fn chat(&self, messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<BoxStream<'static, Event>, ProviderError> {
        let rig_messages = convert_messages(messages);
        let rig_tools = convert_tools(tools);

        // VoyageAI is the only provider that doesn't support chat
        if matches!(self, RigProvider::VoyageAi(_, _)) {
            return Err(ProviderError::InvalidResponse("VoyageAI does not support chat".to_string()));
        }

        // All other providers follow the same streaming pattern via helper fn
        with_rig_provider!(self, client, model, {
            stream_from_client(client, model, rig_messages, rig_tools).await
        })
    }
}
