//! Rig provider implementation using the rig library.

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
use runie_core::{Event, Message, ProviderError, ToolSchema};
use futures::stream::BoxStream;
use futures::StreamExt;

/// Provider enum dispatching to rig's provider clients.
/// Each variant holds (client, model_name).
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
    XAi(xai::Client, String),
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

// =============================================================================
// Macros to reduce boilerplate
// =============================================================================

/// Macro to define provider name and model accessor methods concisely.
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

/// Macro to define constructor for a provider.
macro_rules! define_constructor {
    ($fn_name:ident, $variant:ident, $client_type:ty) => {
        fn $fn_name(api_key: &str, model: String) -> Result<RigProvider, String> {
            let client = <$client_type>::new(api_key).map_err(|e| e.to_string())?;
            Ok(RigProvider::$variant(client, model))
        }
    };
}

/// Macro for dispatching to concrete RigProvider variants.
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
            RigProvider::XAi($client, $model) => $body,
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
                panic!("internal error: VoyageAI must be handled before with_rig_provider macro")
            }
        }
    };
}

// Define accessors
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
    XAi => "xai",
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

// Define constructors using macro
define_constructor!(new_openai_client, OpenAi, openai::Client);
define_constructor!(new_anthropic_client, Anthropic, anthropic::Client);
define_constructor!(new_gemini_client, Gemini, gemini::Client);
define_constructor!(new_cohere_client, Cohere, cohere::Client);
define_constructor!(new_mistral_client, Mistral, mistral::Client);
define_constructor!(new_deepseek_client, DeepSeek, deepseek::Client);
define_constructor!(new_groq_client, Groq, groq::Client);
define_constructor!(new_openrouter_client, OpenRouter, openrouter::Client);
define_constructor!(new_huggingface_client, HuggingFace, huggingface::Client);
define_constructor!(new_xai_client, XAi, xai::Client);
define_constructor!(new_azure_client, Azure, azure::Client);
define_constructor!(new_moonshot_client, Moonshot, moonshot::Client);
define_constructor!(new_perplexity_client, Perplexity, perplexity::Client);
define_constructor!(new_ollama_client, Ollama, ollama::Client);
define_constructor!(new_hyperbolic_client, Hyperbolic, hyperbolic::Client);
define_constructor!(new_together_client, Together, together::Client);
define_constructor!(new_voyageai_client, VoyageAi, voyageai::Client);
define_constructor!(new_zai_client, Zai, zai::Client);
define_constructor!(new_minimax_client, Minimax, minimax::Client);
define_constructor!(new_mira_client, Mira, mira::Client);
define_constructor!(new_galadriel_client, Galadriel, galadriel::Client);

fn new_llamafile_client(model: String) -> Result<RigProvider, String> {
    Ok(RigProvider::Llamafile(
        llamafile::Client::from_url("http://localhost:8080").map_err(|e| e.to_string())?,
        model,
    ))
}

// Provider lookup table
type ProviderCtor = fn(&str, String) -> Result<RigProvider, String>;

const PROVIDER_TABLE: &[(&str, ProviderCtor)] = &[
    ("openai", new_openai_client),
    ("anthropic", new_anthropic_client),
    ("gemini", new_gemini_client),
    ("cohere", new_cohere_client),
    ("mistral", new_mistral_client),
    ("deepseek", new_deepseek_client),
    ("groq", new_groq_client),
    ("openrouter", new_openrouter_client),
    ("huggingface", new_huggingface_client),
    ("xai", new_xai_client),
    ("azure", new_azure_client),
    ("moonshot", new_moonshot_client),
    ("perplexity", new_perplexity_client),
    ("ollama", new_ollama_client),
    ("hyperbolic", new_hyperbolic_client),
    ("together", new_together_client),
    ("voyageai", new_voyageai_client),
    ("zai", new_zai_client),
    ("minimax", new_minimax_client),
    ("mira", new_mira_client),
    ("galadriel", new_galadriel_client),
];

impl RigProvider {
    pub fn new(provider: &str, api_key: &str, model: &str) -> Result<Self, String> {
        let model = model.to_string();
        let provider_lower = provider.to_lowercase();

        if provider_lower == "llamafile" {
            return new_llamafile_client(model);
        }

        PROVIDER_TABLE
            .iter()
            .find(|(name, _)| *name == provider_lower)
            .map(|(_, ctor)| ctor(api_key, model))
            .unwrap_or_else(|| Err(format!("Unknown provider: {}", provider)))
    }

    pub fn supports_tools(&self) -> bool {
        true
    }

    pub fn supports_vision(&self) -> bool {
        let model = self.model().to_lowercase();
        model.contains("4o") || model.contains("claude-3") || model.contains("gemini-1.5") || model.contains("vision")
    }
}

// =============================================================================
// Context token calculation
// =============================================================================

fn openai_tokens(model: &str) -> usize {
    let m = model.to_lowercase();
    if m.contains("4o") || m.contains("4-turbo") {
        128_000
    } else if m.contains("4") {
        8_192
    } else {
        4_096
    }
}

fn anthropic_tokens(model: &str) -> usize {
    let m = model.to_lowercase();
    if m.contains("claude-3-5") || m.contains("claude-3-opus") {
        200_000
    } else if m.contains("claude-3") {
        180_000
    } else {
        100_000
    }
}

fn gemini_tokens(model: &str) -> usize {
    let m = model.to_lowercase();
    if m.contains("1.5") || m.contains("2.0") {
        1_000_000
    } else {
        32_000
    }
}

fn azure_tokens(model: &str) -> usize {
    let m = model.to_lowercase();
    if m.contains("4o") || m.contains("4-turbo") {
        128_000
    } else {
        8_192
    }
}

// Context token lookup: (provider_name, fixed_tokens_or_compute_fn)
const CONTEXT_TOKEN_TABLE: &[(&str, Option<usize>)] = &[
    ("openai", None),
    ("anthropic", None),
    ("gemini", None),
    ("azure", None),
    ("cohere", Some(4_096)),
    ("mistral", Some(32_000)),
    ("deepseek", Some(64_000)),
    ("groq", Some(8_192)),
    ("openrouter", Some(128_000)),
    ("huggingface", Some(4_096)),
    ("xai", Some(128_000)),
    ("moonshot", Some(32_000)),
    ("perplexity", Some(128_000)),
    ("ollama", Some(4_096)),
    ("hyperbolic", Some(128_000)),
    ("together", Some(128_000)),
    ("voyageai", Some(128_000)),
    ("zai", Some(128_000)),
    ("minimax", Some(128_000)),
    ("mira", Some(128_000)),
    ("galadriel", Some(128_000)),
    ("llamafile", Some(128_000)),
];

impl RigProvider {
    pub fn max_context_tokens(&self) -> usize {
        let provider_name = self.name();

        if let Some(tokens) = CONTEXT_TOKEN_TABLE
            .iter()
            .find(|(name, _)| *name == provider_name)
            .and_then(|(_, tokens)| *tokens)
        {
            return tokens;
        }

        let model = self.model();
        match provider_name {
            "openai" => openai_tokens(model),
            "anthropic" => anthropic_tokens(model),
            "gemini" => gemini_tokens(model),
            "azure" => azure_tokens(model),
            _ => 128_000,
        }
    }
}

// =============================================================================
// Message conversion
// =============================================================================

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
    mut messages: Vec<RigMessage>,
    tools: Vec<ToolDefinition>,
) -> Result<CompletionRequest, ProviderError> {
    if messages.is_empty() {
        return Err(ProviderError::InvalidResponse("No messages provided".to_string()));
    }

    let prompt = messages.pop().expect("messages checked non-empty above");
    let chat_history = messages;

    let mut builder = model.completion_request(prompt);
    if !chat_history.is_empty() {
        builder = builder.messages(chat_history);
    }
    if !tools.is_empty() {
        builder = builder.tools(tools);
    }

    Ok(builder.build())
}

// =============================================================================
// Streaming
// =============================================================================

async fn convert_stream<R>(
    mut stream: rig_core::streaming::StreamingCompletionResponse<R>
) -> Result<BoxStream<'static, Event>, ProviderError>
where
    R: Clone + Unpin + rig_core::completion::GetTokenUsage + Send + 'static,
{
    use rig_core::streaming::{StreamedAssistantContent, ToolCallDeltaContent};

    let event_stream = async_stream::stream! {
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(StreamedAssistantContent::Text(text)) => yield Event::MessageDelta { content: text.text },
                Ok(StreamedAssistantContent::ToolCall { tool_call, .. }) => {
                    let args = serde_json::to_string(&tool_call.function.arguments).unwrap_or_default();
                    yield Event::ToolCallDelta { id: tool_call.id, name: tool_call.function.name, arguments: args };
                }
                Ok(StreamedAssistantContent::ToolCallDelta { id, content, .. }) => {
                    match content {
                        ToolCallDeltaContent::Name(name) => yield Event::ToolCallDelta { id: id.clone(), name, arguments: String::new() },
                        ToolCallDeltaContent::Delta(delta) => yield Event::ToolCallDelta { id: id.clone(), name: String::new(), arguments: delta },
                    }
                }
                Ok(StreamedAssistantContent::Reasoning(r)) => yield Event::ThinkingDelta { content: r.display_text() },
                Ok(StreamedAssistantContent::ReasoningDelta { reasoning, .. }) => yield Event::ThinkingDelta { content: reasoning },
                Ok(_) => {}
                Err(e) => yield Event::Error { message: e.to_string() },
            }
        }
    };

    Ok(Box::pin(event_stream))
}

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

// =============================================================================
// Provider trait implementation
// =============================================================================

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

        if matches!(self, RigProvider::VoyageAi(_, _)) {
            return Err(ProviderError::InvalidResponse("VoyageAI does not support chat".to_string()));
        }

        with_rig_provider!(self, client, model, {
            stream_from_client(client, model, rig_messages, rig_tools).await
        })
    }

    async fn chat_simple(&self, messages: Vec<Message>) -> Result<String, ProviderError> {
        let stream = self.chat(messages, vec![]).await?;
        let mut result = String::new();
        futures::pin_mut!(stream);
        while let Some(event) = stream.next().await {
            if let Event::MessageDelta { content } = event {
                result.push_str(&content);
            }
        }
        Ok(result)
    }
}
