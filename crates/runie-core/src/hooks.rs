//! Event hooks registry for extensibility.
//!
//! Hooks receive a JSON payload and return an `Allow`, `Deny`, or `Modify`
//! decision. The registry calls all handlers registered for an event; the first
//! deny wins, otherwise the last modification wins.
//!
//! ## Async Hook System
//!
//! The async hook system provides hooks for LLM API calls with message
//! transformation capabilities. Handlers can inspect and modify the request
//! payload (model, messages, kwargs) before sending to the API.

use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use strum::EnumString;

use crate::proto::message::ChatMessage;
use crate::scoped_model::ScopedModel;

/// Lifecycle events that can be hooked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString)]
pub enum HookEvent {
    #[strum(serialize = "pretooluse", serialize = "pre_tool_use")]
    PreToolUse,
    #[strum(serialize = "posttooluse", serialize = "post_tool_use")]
    PostToolUse,
    #[strum(serialize = "permissionrequest", serialize = "permission_request")]
    PermissionRequest,
    #[strum(serialize = "precompact", serialize = "pre_compact")]
    PreCompact,
    #[strum(serialize = "postcompact", serialize = "post_compact")]
    PostCompact,
    #[strum(serialize = "sessionstart", serialize = "session_start")]
    SessionStart,
    #[strum(serialize = "userpromptsubmit", serialize = "user_prompt_submit")]
    UserPromptSubmit,
    #[strum(serialize = "subagentstart", serialize = "subagent_start")]
    SubagentStart,
    #[strum(serialize = "subagentstop", serialize = "subagent_stop")]
    SubagentStop,
    #[strum(serialize = "stop")]
    Stop,
    /// Fires before an API call is made. Supports async message transformation.
    #[strum(serialize = "preapicall", serialize = "pre_api_call")]
    PreApiCall,
    /// Fires after an API call completes. Supports async processing.
    #[strum(serialize = "postapicall", serialize = "post_api_call")]
    PostApiCall,
    /// Fires for each streaming event from the API. Supports async processing.
    #[strum(serialize = "streamevent", serialize = "stream_event")]
    StreamEvent,
}

/// Decision returned by a hook handler.
#[derive(Debug, Clone, PartialEq)]
pub enum HookDecision {
    /// Allow the action to proceed unchanged.
    Allow,
    /// Deny the action with an optional reason.
    Deny { reason: String },
    /// Allow the action but replace the payload.
    Modify { payload: Value },
}

impl HookDecision {
    /// Short string label for logging.
    pub fn label(&self) -> &'static str {
        match self {
            HookDecision::Allow => "allow",
            HookDecision::Deny { .. } => "deny",
            HookDecision::Modify { .. } => "modify",
        }
    }
}

/// A handler that participates in a hook event.
pub trait HookHandler: Send + Sync {
    /// Process the payload and return a decision.
    fn handle(&self, payload: &Value) -> HookDecision;
}

impl<F> HookHandler for F
where
    F: Fn(&Value) -> HookDecision + Send + Sync,
{
    fn handle(&self, payload: &Value) -> HookDecision {
        (self)(payload)
    }
}

/// A handler that participates in async hook events.
#[async_trait::async_trait]
pub trait AsyncHookHandler: Send + Sync {
    /// Transform messages before an API call is made.
    ///
    /// Return `Some(transformed_messages)` to replace the messages,
    /// or `None` to leave them unchanged.
    async fn async_pre_request_hook(
        &self,
        model: &str,
        messages: &Messages,
    ) -> Option<Messages>;

    /// Process the payload after an API call completes.
    async fn async_post_request_hook(&self, model: &str, payload: &Value) -> Value {
        payload.clone()
    }

    /// Process a streaming event.
    async fn async_stream_event_hook(&self, model: &str, event: &Value) -> Option<Value> {
        let _ = (model, event);
        None
    }
}

#[async_trait::async_trait]
impl<F, Fut> AsyncHookHandler for F
where
    F: Fn(&str, &Messages) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Option<Messages>> + Send,
{
    async fn async_pre_request_hook(
        &self,
        model: &str,
        messages: &Messages,
    ) -> Option<Messages> {
        (self)(model, messages).await
    }
}

/// Registry of hook handlers keyed by event.
#[derive(Default)]
pub struct HookRegistry {
    handlers: HashMap<HookEvent, Vec<Box<dyn HookHandler>>>,
    async_handlers: HashMap<HookEvent, Vec<Box<dyn AsyncHookHandler>>>,
}

impl HookRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a sync handler for an event.
    pub fn register(&mut self, event: HookEvent, handler: Box<dyn HookHandler>) {
        self.handlers.entry(event).or_default().push(handler);
    }

    /// Register an async handler for an event.
    pub fn register_async(&mut self, event: HookEvent, handler: Box<dyn AsyncHookHandler>) {
        self.async_handlers.entry(event).or_default().push(handler);
    }

    /// Emit an event to all registered handlers.
    ///
    /// Returns the folded decision: first `Deny` wins, otherwise the most recent
    /// `Modify` wins, otherwise `Allow`.
    pub fn emit(&self, event: HookEvent, payload: &Value) -> HookDecision {
        let handlers = match self.handlers.get(&event) {
            Some(h) => h,
            None => return HookDecision::Allow,
        };

        let mut decision = HookDecision::Allow;
        for handler in handlers {
            match handler.handle(payload) {
                HookDecision::Deny { reason } => return HookDecision::Deny { reason },
                modify @ HookDecision::Modify { .. } => decision = modify,
                HookDecision::Allow => {}
            }
        }
        decision
    }

    /// Emit an async event to all registered async handlers.
    ///
    /// For `PreApiCall`, calls `async_pre_request_hook` on each handler and
    /// returns the last modification if any handler modified the messages.
    pub async fn async_emit(&self, event: HookEvent, model: &str, messages: &Messages) -> Option<Messages> {
        let handlers = match self.async_handlers.get(&event) {
            Some(h) => h,
            None => return None,
        };

        let mut result: Option<Messages> = None;
        for handler in handlers {
            if let Some(modified) = handler.async_pre_request_hook(model, messages).await {
                result = Some(modified);
            }
        }
        result
    }

    /// Emit an async event with a value payload.
    ///
    /// Calls `async_post_request_hook` or `async_stream_event_hook` on handlers.
    pub async fn async_emit_value(&self, event: HookEvent, model: &str, payload: &Value) -> Option<Value> {
        let handlers = match self.async_handlers.get(&event) {
            Some(h) => h,
            None => return None,
        };

        let mut result: Option<Value> = None;
        for handler in handlers {
            match event {
                HookEvent::PostApiCall => {
                    let modified = handler.async_post_request_hook(model, payload).await;
                    if !modified.is_null() {
                        result = Some(modified);
                    }
                }
                HookEvent::StreamEvent => {
                    if let Some(modified) = handler.async_stream_event_hook(model, payload).await {
                        result = Some(modified);
                    }
                }
                _ => {}
            }
        }
        result
    }

    /// Load hooks declared in config.
    ///
    /// Each configured command receives the JSON payload on stdin and must print
    /// `allow`, `deny`, or a JSON object to replace the payload.
    pub fn load_from_config(config: &crate::config::Config) -> Self {
        let mut registry = Self::new();
        for (event_name, commands) in &config.hooks.commands {
            if let Some(event) = parse_event_name(event_name) {
                for cmd in commands {
                    registry.register(event, Box::new(ShellHook::new(cmd.clone())));
                }
            }
        }
        registry
    }
}

#[async_trait::async_trait]
impl AsyncHookHandler for HookRegistry {
    async fn async_pre_request_hook(
        &self,
        model: &str,
        messages: &Messages,
    ) -> Option<Messages> {
        self.async_emit(HookEvent::PreApiCall, model, messages).await
    }

    async fn async_post_request_hook(&self, model: &str, payload: &Value) -> Value {
        self.async_emit_value(HookEvent::PostApiCall, model, payload)
            .await
            .unwrap_or_else(|| payload.clone())
    }

    async fn async_stream_event_hook(&self, model: &str, event: &Value) -> Option<Value> {
        self.async_emit_value(HookEvent::StreamEvent, model, event).await
    }
}

/// Parse a hook event name (case-insensitive) into a `HookEvent`.
fn parse_event_name(name: &str) -> Option<HookEvent> {
    HookEvent::from_str(&name.to_ascii_lowercase()).ok()
}

/// Hook handler that runs an external command.
#[derive(Debug, Clone)]
pub struct ShellHook {
    command: String,
}

impl ShellHook {
    /// Create a new shell hook.
    pub fn new(command: String) -> Self {
        Self { command }
    }
}

impl HookHandler for ShellHook {
    fn handle(&self, payload: &Value) -> HookDecision {
        let input = match serde_json::to_string(payload) {
            Ok(s) => s,
            Err(_) => return HookDecision::Allow,
        };
        tokio::task::block_in_place(|| run_shell_hook(&self.command, &input))
    }
}

fn run_shell_hook(command: &str, input: &str) -> HookDecision {
    let output = match std::process::Command::new("sh")
        .args(["-c", command])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(mut child) => {
            use std::io::Write;
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(input.as_bytes());
            }
            match child.wait_with_output() {
                Ok(o) => o,
                Err(_) => return HookDecision::Allow,
            }
        }
        Err(_) => return HookDecision::Allow,
    };

    if !output.status.success() {
        return HookDecision::Allow;
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    match text.to_ascii_lowercase().as_str() {
        "allow" | "" => HookDecision::Allow,
        "deny" => HookDecision::Deny {
            reason: "hook denied".into(),
        },
        _ => match serde_json::from_str::<Value>(&text) {
            Ok(value) => HookDecision::Modify { payload: value },
            Err(_) => HookDecision::Allow,
        },
    }
}

/// Built-in hook that logs the event and returns `Allow`.
pub struct LoggingHook;

impl HookHandler for LoggingHook {
    fn handle(&self, payload: &Value) -> HookDecision {
        tracing::debug!(payload = %payload, "hook event");
        HookDecision::Allow
    }
}

/// Built-in hook that always allows permission requests.
pub struct PermissionHook;

impl HookHandler for PermissionHook {
    fn handle(&self, _payload: &Value) -> HookDecision {
        HookDecision::Allow
    }
}

/// Built-in hook that allows compaction events unchanged.
pub struct CompactionHook;

impl HookHandler for CompactionHook {
    fn handle(&self, _payload: &Value) -> HookDecision {
        HookDecision::Allow
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Async Hook System
// ─────────────────────────────────────────────────────────────────────────────

/// Pre-request hook context passed to async handlers.
#[derive(Debug, Clone)]
pub struct PreRequestHookContext {
    /// The model being used.
    pub model: ScopedModel,
    /// Current messages in the conversation.
    pub messages: Vec<ChatMessage>,
    /// Additional provider-specific kwargs.
    pub kwargs: Value,
}

impl PreRequestHookContext {
    /// Create a new pre-request context.
    pub fn new(
        model: ScopedModel,
        messages: Vec<ChatMessage>,
        kwargs: Value,
    ) -> Self {
        Self {
            model,
            messages,
            kwargs,
        }
    }
}

/// Async handler for pre-API-call hooks.
///
/// Return `None` to keep messages unchanged, or `Some(Vec<ChatMessage>)` to
/// replace the messages in the request.
#[async_trait::async_trait]
pub trait AsyncPreRequestHookHandler: Send + Sync {
    /// Handle a pre-request hook, potentially transforming the messages.
    async fn handle_pre_request(
        &self,
        ctx: &PreRequestHookContext,
    ) -> Option<Vec<ChatMessage>>;
}

#[async_trait::async_trait]
impl<F> AsyncPreRequestHookHandler for F
where
    F: Fn(&PreRequestHookContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<Vec<ChatMessage>>> + Send>> + Send + Sync,
{
    async fn handle_pre_request(
        &self,
        ctx: &PreRequestHookContext,
    ) -> Option<Vec<ChatMessage>> {
        (self)(ctx).await
    }
}

/// Pre-response hook context passed to async handlers.
#[derive(Debug, Clone)]
pub struct PreResponseHookContext {
    /// The model that was used.
    pub model: ScopedModel,
    /// Messages that were sent (after any pre-request modifications).
    pub messages: Vec<ChatMessage>,
    /// Additional provider-specific kwargs.
    pub kwargs: Value,
}

impl PreResponseHookContext {
    /// Create a new pre-response context.
    pub fn new(
        model: ScopedModel,
        messages: Vec<ChatMessage>,
        kwargs: Value,
    ) -> Self {
        Self {
            model,
            messages,
            kwargs,
        }
    }
}

/// Async handler for post-API-call hooks.
///
/// Return `None` to keep response unchanged, or `Some(String)` to replace the
/// response text.
#[async_trait::async_trait]
pub trait AsyncPostRequestHookHandler: Send + Sync {
    /// Handle a post-request hook, potentially transforming the response.
    async fn handle_post_request(
        &self,
        ctx: &PreResponseHookContext,
        response: &str,
    ) -> Option<String>;
}

#[async_trait::async_trait]
impl<F> AsyncPostRequestHookHandler for F
where
    F: Fn(&PreResponseHookContext, &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<String>> + Send>> + Send + Sync,
{
    async fn handle_post_request(
        &self,
        ctx: &PreResponseHookContext,
        response: &str,
    ) -> Option<String> {
        (self)(ctx, response).await
    }
}

/// Registry for async API call hooks.
///
/// This provides a simpler interface than the raw event system, specifically
/// designed for LLM API call interception with message transformation.
#[derive(Default)]
pub struct AsyncHookRegistry {
    pre_request_handlers: Vec<Box<dyn AsyncPreRequestHookHandler>>,
    post_request_handlers: Vec<Box<dyn AsyncPostRequestHookHandler>>,
}

impl AsyncHookRegistry {
    /// Create an empty async hook registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a handler for pre-request hooks.
    pub fn register_pre_request(
        &mut self,
        handler: Box<dyn AsyncPreRequestHookHandler>,
    ) {
        self.pre_request_handlers.push(handler);
    }

    /// Register a handler for post-request hooks.
    pub fn register_post_request(
        &mut self,
        handler: Box<dyn AsyncPostRequestHookHandler>,
    ) {
        self.post_request_handlers.push(handler);
    }

    /// Run all pre-request hooks on the given context.
    ///
    /// Returns the final messages: `None` if no hook modified them, or
    /// `Some(Vec<ChatMessage>)` with the last modification applied.
    pub async fn async_pre_request_hook(
        &self,
        model: ScopedModel,
        messages: Vec<ChatMessage>,
        kwargs: Value,
    ) -> Option<Vec<ChatMessage>> {
        if self.pre_request_handlers.is_empty() {
            return None;
        }

        let ctx = PreRequestHookContext::new(model, messages, kwargs);
        let mut result: Option<Vec<ChatMessage>> = None;

        for handler in &self.pre_request_handlers {
            if let Some(msgs) = handler.handle_pre_request(&ctx).await {
                result = Some(msgs);
            }
        }

        result
    }

    /// Run all post-request hooks on the given context and response.
    ///
    /// Returns the final response: `None` if no hook modified it, or
    /// `Some(String)` with the last modification applied.
    pub async fn async_post_request_hook(
        &self,
        model: ScopedModel,
        messages: Vec<ChatMessage>,
        kwargs: Value,
        response: &str,
    ) -> Option<String> {
        if self.post_request_handlers.is_empty() {
            return None;
        }

        let ctx = PreResponseHookContext::new(model, messages, kwargs);
        let mut result: Option<String> = None;

        for handler in &self.post_request_handlers {
            if let Some(resp) = handler.handle_post_request(&ctx, response).await {
                result = Some(resp);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Role;

    #[test]
    fn hook_registry_calls_handler_on_event() {
        let mut registry = HookRegistry::new();
        registry.register(
            HookEvent::PreToolUse,
            Box::new(|_payload: &Value| HookDecision::Deny {
                reason: "blocked".into(),
            }),
        );

        let decision = registry.emit(HookEvent::PreToolUse, &Value::Null);
        assert_eq!(
            decision,
            HookDecision::Deny {
                reason: "blocked".into()
            }
        );
    }

    #[test]
    fn hook_can_modify_input() {
        let mut registry = HookRegistry::new();
        registry.register(
            HookEvent::UserPromptSubmit,
            Box::new(|_payload: &Value| HookDecision::Modify {
                payload: Value::String("transformed".into()),
            }),
        );

        let decision = registry.emit(HookEvent::UserPromptSubmit, &Value::Null);
        assert_eq!(
            decision,
            HookDecision::Modify {
                payload: Value::String("transformed".into())
            }
        );
    }

    #[test]
    fn hook_can_deny_action() {
        let mut registry = HookRegistry::new();
        registry.register(
            HookEvent::PreToolUse,
            Box::new(|_payload: &Value| HookDecision::Deny {
                reason: "no tools".into(),
            }),
        );

        let decision = registry.emit(HookEvent::PreToolUse, &Value::Null);
        assert!(matches!(decision, HookDecision::Deny { .. }));
    }

    #[test]
    fn pre_tool_hook_intercepts_tool_call() {
        let mut registry = HookRegistry::new();
        registry.register(
            HookEvent::PreToolUse,
            Box::new(|payload: &Value| {
                if payload.get("tool").and_then(|v| v.as_str()) == Some("write_file") {
                    HookDecision::Deny {
                        reason: "writes blocked".into(),
                    }
                } else {
                    HookDecision::Allow
                }
            }),
        );

        let payload = serde_json::json!({"tool": "write_file"});
        let decision = registry.emit(HookEvent::PreToolUse, &payload);
        assert_eq!(
            decision,
            HookDecision::Deny {
                reason: "writes blocked".into()
            }
        );

        let payload = serde_json::json!({"tool": "read_file"});
        let decision = registry.emit(HookEvent::PreToolUse, &payload);
        assert_eq!(decision, HookDecision::Allow);
    }

    #[test]
    fn shell_hook_allow() {
        let hook = ShellHook::new("echo allow".into());
        let decision = hook.handle(&serde_json::json!({"x": 1}));
        assert_eq!(decision, HookDecision::Allow);
    }

    #[test]
    fn shell_hook_deny() {
        let hook = ShellHook::new("echo deny".into());
        let decision = hook.handle(&serde_json::json!({"x": 1}));
        assert_eq!(
            decision,
            HookDecision::Deny {
                reason: "hook denied".into()
            }
        );
    }

    #[test]
    fn shell_hook_modify() {
        let hook = ShellHook::new(r#"echo '{"modified": true}'"#.into());
        let decision = hook.handle(&serde_json::json!({"x": 1}));
        assert_eq!(
            decision,
            HookDecision::Modify {
                payload: serde_json::json!({"modified": true})
            }
        );
    }

    #[test]
    fn parse_event_name_handles_snake_case() {
        assert_eq!(
            HookEvent::from_str(&"pre_tool_use".to_lowercase()).ok(),
            Some(HookEvent::PreToolUse)
        );
        assert_eq!(
            HookEvent::from_str(&"stop".to_lowercase()).ok(),
            Some(HookEvent::Stop)
        );
        assert_eq!(HookEvent::from_str("unknown").ok(), None);
    }

    #[tokio::test]
    async fn async_hook_registry_transforms_messages() {
        let mut registry = HookRegistry::new();

        // Register an async handler that adds a system message
        registry.register_async(
            HookEvent::PreApiCall,
            Box::new(|_model: &str, messages: &Messages| {
                let mut modified = messages.clone();
                modified.insert(
                    0,
                    ChatMessage::new(Role::System, "injected system prompt"),
                );
                async { Some(modified) }
            }),
        );

        let messages = vec![ChatMessage::new(Role::User, "hello")];
        let result = registry.async_emit(HookEvent::PreApiCall, "gpt-4", &messages).await;
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].content(), "injected system prompt");
        assert_eq!(result[1].content(), "hello");
    }

    #[tokio::test]
    async fn async_hook_registry_returns_none_when_no_handlers() {
        let registry = HookRegistry::new();
        let messages = vec![ChatMessage::new(Role::User, "hello")];
        let result = registry.async_emit(HookEvent::PreApiCall, "gpt-4", &messages).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn async_hook_registry_last_modification_wins() {
        let mut registry = HookRegistry::new();

        // First handler adds "first"
        registry.register_async(
            HookEvent::PreApiCall,
            Box::new(|_model: &str, messages: &Messages| {
                let mut modified = messages.clone();
                modified.insert(0, ChatMessage::new(Role::System, "first"));
                async { Some(modified) }
            }),
        );

        // Second handler adds "second"
        registry.register_async(
            HookEvent::PreApiCall,
            Box::new(|_model: &str, messages: &Messages| {
                let mut modified = messages.clone();
                modified.insert(0, ChatMessage::new(Role::System, "second"));
                async { Some(modified) }
            }),
        );

        let messages = vec![ChatMessage::new(Role::User, "hello")];
        let result = registry.async_emit(HookEvent::PreApiCall, "gpt-4", &messages).await;
        assert!(result.is_some());
        let result = result.unwrap();
        // Last modification wins
        assert_eq!(result[0].content(), "second");
    }

    #[tokio::test]
    async fn async_hook_registry_composition_with_trait_impl() {
        let mut registry = HookRegistry::new();

        registry.register_async(
            HookEvent::PreApiCall,
            Box::new(|_model: &str, messages: &Messages| {
                let mut modified = messages.clone();
                modified.push(ChatMessage::new(Role::System, "appended"));
                async { Some(modified) }
            }),
        );

        // Use the AsyncHookHandler trait impl
        let messages = vec![ChatMessage::new(Role::User, "test")];
        let result = registry.async_pre_request_hook("claude-3", &messages).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 2);
    }

    /// Test helper that implements async post-request hook processing.
    struct PostApiCallTestHandler;

    #[async_trait::async_trait]
    impl AsyncHookHandler for PostApiCallTestHandler {
        async fn async_pre_request_hook(
            &self,
            _model: &str,
            _messages: &Messages,
        ) -> Option<Messages> {
            None
        }

        async fn async_post_request_hook(&self, _model: &str, payload: &Value) -> Value {
            let mut modified = payload.clone();
            if let Some(obj) = modified.as_object_mut() {
                obj.insert("processed".into(), serde_json::json!(true));
            }
            modified
        }
    }

    #[tokio::test]
    async fn async_hook_registry_post_api_call() {
        let mut registry = HookRegistry::new();

        registry.register_async(
            HookEvent::PostApiCall,
            Box::new(PostApiCallTestHandler),
        );

        let payload = serde_json::json!({"response": "hello"});
        let result = registry.async_emit_value(HookEvent::PostApiCall, "gpt-4", &payload).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap()["processed"], serde_json::json!(true));
    }

    /// Test helper that implements async stream-event hook processing.
    struct StreamEventTestHandler;

    #[async_trait::async_trait]
    impl AsyncHookHandler for StreamEventTestHandler {
        async fn async_pre_request_hook(
            &self,
            _model: &str,
            _messages: &Messages,
        ) -> Option<Messages> {
            None
        }

        async fn async_stream_event_hook(
            &self,
            _model: &str,
            event: &Value,
        ) -> Option<Value> {
            let mut modified = event.clone();
            if let Some(obj) = modified.as_object_mut() {
                obj.insert("logged".into(), serde_json::json!(true));
            }
            Some(modified)
        }
    }

    #[tokio::test]
    async fn async_hook_registry_stream_event() {
        let mut registry = HookRegistry::new();

        registry.register_async(
            HookEvent::StreamEvent,
            Box::new(StreamEventTestHandler),
        );

        let event = serde_json::json!({"type": "chunk", "content": "hello"});
        let result = registry.async_emit_value(HookEvent::StreamEvent, "gpt-4", &event).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap()["logged"], serde_json::json!(true));
    }

    #[test]
    fn hook_event_parses_async_events() {
        assert_eq!(
            HookEvent::from_str("pre_api_call").ok(),
            Some(HookEvent::PreApiCall)
        );
        assert_eq!(
            HookEvent::from_str("post_api_call").ok(),
            Some(HookEvent::PostApiCall)
        );
        assert_eq!(
            HookEvent::from_str("stream_event").ok(),
            Some(HookEvent::StreamEvent)
        );
        // Also test alternative serializations
        assert_eq!(
            HookEvent::from_str("preapicall").ok(),
            Some(HookEvent::PreApiCall)
        );
        assert_eq!(
            HookEvent::from_str("postapicall").ok(),
            Some(HookEvent::PostApiCall)
        );
        assert_eq!(
            HookEvent::from_str("streamevent").ok(),
            Some(HookEvent::StreamEvent)
        );
    }

    #[tokio::test]
    async fn async_hook_handler_trait_closure_impl() {
        // Test the blanket impl for Fn closures
        let handler: Box<dyn AsyncHookHandler> = Box::new(
            |model: &str, messages: &Messages| {
                let model = model.to_string();
                let mut msgs = messages.clone();
                async move {
                    msgs.insert(0, ChatMessage::new(Role::System, format!("model: {}", model)));
                    Some(msgs)
                }
            },
        );

        let messages = vec![ChatMessage::new(Role::User, "test")];
        let result = handler.async_pre_request_hook("gpt-4", &messages).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap()[0].content(), "model: gpt-4");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Async Hook Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod async_hooks_tests {
    use super::*;

    fn make_test_messages() -> Vec<ChatMessage> {
        vec![
            ChatMessage::system("You are helpful.".to_string()),
            ChatMessage::user("hello".to_string()),
        ]
    }

    fn make_test_model() -> ScopedModel {
        ScopedModel {
            name: "test-model".to_string(),
            provider: "test".to_string(),
            enabled: true,
        }
    }

    #[tokio::test]
    async fn async_hook_registry_returns_none_when_empty() {
        let registry = AsyncHookRegistry::new();
        let model = make_test_model();
        let messages = make_test_messages();

        let result = registry
            .async_pre_request_hook(model, messages, Value::Null)
            .await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn async_hook_registry_calls_handler_and_returns_modification() {
        let mut registry = AsyncHookRegistry::new();
        registry.register_pre_request(Box::new(|_ctx: &PreRequestHookContext| {
            Box::pin(async {
                Some(vec![ChatMessage::user("modified".to_string())])
            })
        }));

        let model = make_test_model();
        let messages = make_test_messages();

        let result = registry
            .async_pre_request_hook(model, messages, Value::Null)
            .await;
        assert!(result.is_some());
        let msgs = result.unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content(), "modified");
    }

    #[tokio::test]
    async fn async_hook_registry_uses_last_modification() {
        let mut registry = AsyncHookRegistry::new();

        // First handler returns modification
        registry.register_pre_request(Box::new(|_ctx: &PreRequestHookContext| {
            Box::pin(async {
                Some(vec![ChatMessage::user("first".to_string())])
            })
        }));

        // Second handler returns different modification (last wins)
        registry.register_pre_request(Box::new(|_ctx: &PreRequestHookContext| {
            Box::pin(async {
                Some(vec![ChatMessage::user("second".to_string())])
            })
        }));

        let model = make_test_model();
        let messages = make_test_messages();

        let result = registry
            .async_pre_request_hook(model, messages, Value::Null)
            .await;
        assert!(result.is_some());
        let msgs = result.unwrap();
        assert_eq!(msgs[0].content(), "second");
    }

    #[tokio::test]
    async fn async_hook_registry_passes_context_correctly() {
        let mut registry = AsyncHookRegistry::new();
        let expected_model = make_test_model();
        let expected_messages = make_test_messages();
        let expected_kwargs = serde_json::json!({"temperature": 0.7});

        registry.register_pre_request(Box::new(
            move |ctx: &PreRequestHookContext| {
                let model = expected_model.clone();
                let kwargs = expected_kwargs.clone();
                Box::pin(async move {
                    assert_eq!(ctx.model.name, model.name);
                    assert_eq!(ctx.model.provider, model.provider);
                    assert_eq!(ctx.messages.len(), expected_messages.len());
                    assert_eq!(ctx.kwargs, kwargs);
                    None
                })
            },
        ));

        let result = registry
            .async_pre_request_hook(
                expected_model,
                expected_messages,
                expected_kwargs,
            )
            .await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn async_post_request_hook_returns_none_when_empty() {
        let registry = AsyncHookRegistry::new();
        let model = make_test_model();
        let messages = make_test_messages();

        let result = registry
            .async_post_request_hook(model, messages, Value::Null, "hello")
            .await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn async_post_request_hook_modifies_response() {
        let mut registry = AsyncHookRegistry::new();
        registry.register_post_request(Box::new(
            |_ctx: &PreResponseHookContext, _response: &str| {
                Box::pin(async { Some("modified response".to_string()) })
            },
        ));

        let model = make_test_model();
        let messages = make_test_messages();

        let result = registry
            .async_post_request_hook(model, messages, Value::Null, "original")
            .await;
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "modified response");
    }

    #[tokio::test]
    async fn async_hook_registry_both_hooks_work_together() {
        let mut registry = AsyncHookRegistry::new();

        registry.register_pre_request(Box::new(|_ctx: &PreRequestHookContext| {
            Box::pin(async {
                Some(vec![ChatMessage::user("transformed".to_string())])
            })
        }));

        registry.register_post_request(Box::new(
            |_ctx: &PreResponseHookContext, _response: &str| {
                Box::pin(async { Some("final".to_string()) })
            },
        ));

        let model = make_test_model();
        let messages = make_test_messages();

        let pre_result = registry
            .async_pre_request_hook(model.clone(), messages.clone(), Value::Null)
            .await;
        assert!(pre_result.is_some());
        assert_eq!(pre_result.unwrap()[0].content(), "transformed");

        let post_result = registry
            .async_post_request_hook(model, messages, Value::Null, "original")
            .await;
        assert!(post_result.is_some());
        assert_eq!(post_result.unwrap(), "final");
    }

    #[test]
    fn pre_request_hook_context_creation() {
        let model = make_test_model();
        let messages = make_test_messages();
        let kwargs = serde_json::json!({"max_tokens": 100});

        let ctx = PreRequestHookContext::new(model.clone(), messages.clone(), kwargs.clone());

        assert_eq!(ctx.model.name, "test-model");
        assert_eq!(ctx.model.provider, "test");
        assert_eq!(ctx.messages.len(), 2);
        assert_eq!(ctx.kwargs["max_tokens"], 100);
    }

    #[test]
    fn pre_response_hook_context_creation() {
        let model = make_test_model();
        let messages = make_test_messages();
        let kwargs = serde_json::json!({"stream": true});

        let ctx = PreResponseHookContext::new(model.clone(), messages.clone(), kwargs.clone());

        assert_eq!(ctx.model.name, "test-model");
        assert_eq!(ctx.messages.len(), 2);
        assert!(ctx.kwargs["stream"].as_bool().unwrap());
    }

    #[test]
    fn hook_event_api_call_variants() {
        // Test that PreApiCall and PostApiCall parse correctly
        assert_eq!(
            HookEvent::from_str("pre_api_call").ok(),
            Some(HookEvent::PreApiCall)
        );
        assert_eq!(
            HookEvent::from_str("preapicall").ok(),
            Some(HookEvent::PreApiCall)
        );
        assert_eq!(
            HookEvent::from_str("post_api_call").ok(),
            Some(HookEvent::PostApiCall)
        );
        assert_eq!(
            HookEvent::from_str("postapicall").ok(),
            Some(HookEvent::PostApiCall)
        );
    }
}
