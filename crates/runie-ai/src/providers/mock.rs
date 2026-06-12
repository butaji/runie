use async_trait::async_trait;
use runie_core::{Message, ToolSchema, Event, ProviderError, ToolOutput};
use futures::stream::BoxStream;
use crate::Provider;
use async_stream::stream;
use chrono::Utc;
use std::time::Duration;

/// A mock provider for development/testing that simulates LLM responses.
pub struct MockProvider {
    model: String,
    response_delay_ms: u64,
    simulate_errors: bool,
    error_rate: f32,
    simulate_rate_limit: bool,
}

impl MockProvider {

    #[must_use]
    pub fn new() -> Self {
        Self {
            model: "mock-gpt-4".to_string(),
            response_delay_ms: 100,
            simulate_errors: false,
            error_rate: 0.0,
            simulate_rate_limit: false,
        }
    }

    pub fn with_delay(mut self, ms: u64) -> Self {
        self.response_delay_ms = ms;
        self
    }

    pub fn with_errors(mut self, rate: f32) -> Self {
        self.simulate_errors = true;
        self.error_rate = rate.clamp(0.0, 1.0);
        self
    }

    pub fn with_rate_limit_simulation(mut self) -> Self {
        self.simulate_rate_limit = true;
        self
    }

    fn generate_response(&self, messages: &[Message], tools: &[ToolSchema]) -> Vec<Event> {
        let user_message = Self::extract_user_message(messages);
        let response_content = Self::build_content(messages);

        // Only use tools on the user's literal first message — never on tool
        // outputs (would loop forever because tool result text contains "read",
        // "list", etc., which the heuristic below would re-fire on).
        let last_msg_is_tool = messages.last().map_or(false, |m| matches!(m, Message::ToolResult { .. }));
        let can_use_tools = !last_msg_is_tool && !tools.is_empty();

        if can_use_tools {
            if let Some((tool_name, tool_args)) = Self::should_use_tools(&user_message, tools) {
                return Self::build_tool_response(&tool_name, &tool_args);
            }
        }

        Self::build_text_response(&response_content)
    }

    fn extract_user_message(messages: &[Message]) -> String {
        let last = messages.iter().rev().find(|m| matches!(m, Message::User { .. }));
        match last {
            Some(Message::User { content, .. }) => content.clone(),
            _ => String::new(),
        }
    }

    fn build_tool_response(name: &str, args: &str) -> Vec<Event> {
        let duration_ms = 2000 + (rand::random::<u64>() % 1000); // 2-3 seconds
        vec![
            Event::AgentStart { session_id: "mock-session".to_string(), timestamp: Utc::now() },
            Event::MessageStart { role: "assistant".to_string(), timestamp: Utc::now() },
            Event::MessageDelta { content: format!("Let me {} for you...\n\n", name.to_lowercase()) },
            Event::ToolCallDelta {
                id: "mock-1".to_string(),
                name: name.to_string(),
                arguments: args.to_string(),
            },
            Event::ToolExecutionStart {
                tool_call_id: "mock-1".to_string(),
                tool_name: name.to_string(),
                args: serde_json::Value::String(args.to_string()),
                timestamp: Utc::now(),
            },
            // ToolExecutionEnd will be streamed separately with delay in chat()
            Event::ToolExecutionEnd {
                tool_call_id: "mock-1".to_string(),
                result: ToolOutput {
                    content: format!("{} completed successfully", name),
                    metadata: serde_json::json!({ "duration_ms": duration_ms }),
                    terminate: false,
                },
                timestamp: Utc::now(),
            },
            Event::MessageEnd,
            Event::AgentEnd { timestamp: Utc::now() },
        ]
    }

    fn build_text_response(content: &str) -> Vec<Event> {
        let thinking = Self::thinking_for_text(content);

        let mut events = vec![
            Event::AgentStart { session_id: "mock-session".to_string(), timestamp: Utc::now() },
            Event::MessageStart { role: "assistant".to_string(), timestamp: Utc::now() },
        ];

        // Emit thinking with proper tags and as ThinkingDelta
        events.push(Event::ThinkingDelta { content: "<think>\n".to_string() });
        for line in thinking.lines() {
            events.push(Event::ThinkingDelta { content: format!("{}\n", line) });
        }
        events.push(Event::ThinkingDelta { content: "</think>\n".to_string() });

        // Split response into word-level chunks for realistic streaming
        let words: Vec<&str> = content.split_whitespace().collect();
        for word in words {
            events.push(Event::MessageDelta { content: format!("{} ", word) });
        }

        events.push(Event::MessageEnd);
        events.push(Event::AgentEnd { timestamp: Utc::now() });
        events
    }

    fn thinking_for_text(text: &str) -> String {
        let lower = text.to_lowercase();
        if lower.contains("hello") || lower.contains("hi") {
            return "Looking at the conversation context...\n\
The user is greeting me with \"hello\" or \"hi\".\n\
This indicates they want a friendly, welcoming response.\n\
I should respond warmly and offer assistance.\n\n"
            .to_string();
        }
        if lower.contains("list") {
            return "The user mentioned \"list\" which suggests file system enumeration.\n\
I need to check what directory they want listed.\n\
Let me prepare to show them available files and folders.\n\
This will help them navigate their project structure.\n\n"
            .to_string();
        }
        if lower.contains("read") {
            return "The user wants to \"read\" something from the filesystem.\n\
I should locate the file they mentioned and retrieve its contents.\n\
Reading files is a common operation for understanding code.\n\
I'll make sure to handle any encoding or size issues.\n\n"
            .to_string();
        }
        if lower.contains("edit") || lower.contains("fix") {
            return "The user wants to \"edit\" or \"fix\" something in their code.\n\
I need to first understand the current state of the file.\n\
Then I can determine what changes are needed.\n\
I'll be careful to preserve existing functionality.\n\n"
            .to_string();
        }
        let preview: String = text.chars().take(30).collect();
        format!(
            "Analyzing the user's request: \"{}\"\n\
This appears to be a general query or command.\n\
I need to determine the appropriate action to take.\n\
Let me formulate a helpful response.\n\n",
            preview
        )
    }

    fn should_use_tools(content: &str, tools: &[ToolSchema]) -> Option<(String, String)> {
        if tools.is_empty() {
            return None;
        }
        // Only trigger on explicit user commands - check for exact word boundaries
        // to avoid matching AI response text like "I can help with reading files"
        let lower = content.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();
        let has_tool_command = words.iter().any(|&w| {
            w == "read" || w == "list" || w == "edit" || w == "search" || w == "write"
        });
        if !has_tool_command {
            return None;
        }
        let (tool_name, tool_args) = Self::detect_tool_and_args(&lower, tools);
        Some((tool_name, tool_args))
    }

    fn detect_tool_and_args(lower: &str, tools: &[ToolSchema]) -> (String, String) {
        let name = if lower.contains("read") {
            "Read".to_string()
        } else if lower.contains("list") {
            "List".to_string()
        } else {
            tools[0].name.clone()
        };
        let args = if lower.contains("list") { ".".to_string() } else { "{}".to_string() };
        (name, args)
    }

    fn build_content(messages: &[Message]) -> String {
        let last = messages.iter().rev().find(|m| matches!(m, Message::User { .. }));
        let text = match last {
            Some(Message::User { content, .. }) => content.as_str(),
            _ => return Self::default_response(),
        };
        Self::response_for_text(text)
    }

    fn default_response() -> String {
        "I'm ready to help! What would you like to work on?".to_string()
    }

    fn greeting_response() -> String {
        "Hello! 👋 How can I help you today?\n\n\
I'm your AI coding assistant, ready to help with:\n\
• Reading and editing files\n\
• Running commands and tests\n\
• Navigating your codebase\n\
• Answering questions about your code\n\n\
What would you like to work on?".to_string()
    }

    fn list_response() -> String {
        "I can help you list files! 📁\n\n\
The `List` tool will show you all files and directories\n\
in the specified path. You can use it to explore your\n\
project structure and find the files you need.\n\n\
What directory would you like me to list?".to_string()
    }

    fn read_response() -> String {
        "I'll read that file for you! 📖\n\n\
The `Read` tool will fetch the contents of any file\n\
you specify. I can help you understand the code,\n\
find specific functions or patterns, and see how\n\
different parts of your project are organized.\n\n\
Which file would you like me to read?".to_string()
    }

    fn edit_response() -> String {
        "I can help with that edit! ✏️\n\n\
The `Edit` tool lets me make precise changes to your\n\
files. I'll read the current content first, then\n\
apply your requested modifications carefully,\n\
preserving the rest of the file intact.\n\n\
Let me take a look at the current state.".to_string()
    }

    fn test_response() -> String {
        "I'll run those tests for you! 🧪\n\n\
The test runner will execute your test suite and\n\
report back with the results. I'll show you which\n\
tests passed, which failed, and any error messages\n\
that might help debug issues.\n\n\
Let me check your test setup.".to_string()
    }

    fn default_text_response(text: &str) -> String {
        let preview: String = text.chars().take(50).collect();
        format!(
            "I see: \"{}\"\n\n\
That's an interesting request! Let me think about\n\
how I can best help you with this. I'll need to\n\
understand more context to provide a complete answer.\n\
Could you provide more details about what you're\n\
looking for?\n\n🔧 How can I assist you with this?",
            preview
        )
    }

    fn response_for_text(text: &str) -> String {
        let lower = text.to_lowercase();
        if lower.split_whitespace().any(|w| w == "hello" || w == "hi") {
            return Self::greeting_response();
        }
        if lower.contains("list") {
            return Self::list_response();
        }
        if lower.contains("read") {
            return Self::read_response();
        }
        if lower.contains("edit") || lower.contains("fix") {
            return Self::edit_response();
        }
        if lower.contains("test") {
            return Self::test_response();
        }
        Self::default_text_response(text)
    }
}

#[async_trait]
impl Provider for MockProvider {
    fn name(&self) -> &str {
        "mock"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn supports_vision(&self) -> bool {
        true
    }

    fn max_context_tokens(&self) -> usize {
        128_000
    }

    async fn chat(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
    ) -> Result<BoxStream<'static, Event>, ProviderError> {
        if self.simulate_errors && rand::random::<f32>() < self.error_rate {
            return Err(ProviderError::ApiError("Simulated error".to_string()));
        }
        if self.simulate_rate_limit && rand::random::<f32>() < 0.3 {
            return Err(ProviderError::RateLimited);
        }

        let events = self.generate_response(&messages, &tools);

        let delay = self.response_delay_ms;
        let s = stream! {
            for event in events {
                // Special handling for tool execution - add 2-3s delay
                if matches!(event, Event::ToolExecutionStart { .. }) {
                    yield event;
                    let tool_delay = 2000 + (rand::random::<u64>() % 1000);
                    tokio::time::sleep(Duration::from_millis(tool_delay)).await;
                    continue;
                }
                yield event;
                if delay > 0 {
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }
        };

        Ok(Box::pin(s))
    }

    async fn chat_simple(
        &self,
        messages: Vec<Message>,
    ) -> Result<String, ProviderError> {
        let events = self.generate_response(&messages, &[]);
        let mut content = String::new();

        for event in events {
            if let Event::MessageDelta { content: c } = event {
                content.push_str(&c);
            }
        }

        Ok(content)
    }
}
