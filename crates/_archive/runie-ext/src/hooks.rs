//! Hook system - pre/post action interceptors
//!
//! Hooks are lightweight callbacks that fire before or after events.
//! Unlike plugins, hooks cannot emit arbitrary actions - they can only
//! either allow, modify, or block the original operation.
//!
//! ## Hook Types
//!
//! - `pre_*`: Fire before an action (can modify args or block)
//! - `post_*`: Fire after an action (can inspect results)
//! - `on_*`: Fire on events (no blocking capability)

use crate::{PluginEvent, PluginAction, Plugin};
use std::sync::Arc;
use std::collections::HashMap;

/// Hook priority - higher = earlier execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HookPriority(pub u8);

impl HookPriority {
    pub const LOWEST: HookPriority = HookPriority(0);
    pub const LOW: HookPriority = HookPriority(50);
    pub const NORMAL: HookPriority = HookPriority(100);
    pub const HIGH: HookPriority = HookPriority(150);
    pub const HIGHEST: HookPriority = HookPriority(200);
}

impl Default for HookPriority {
    fn default() -> Self {
        HookPriority::NORMAL
    }
}

/// A single hook function
pub struct Hook {
    pub name: String,
    pub priority: HookPriority,
    pub handler: HookHandler,
}

/// Handler for a hook
pub enum HookHandler {
    /// Sync pre-hook - can modify args or block
    Pre(fn(PreHookContext) -> PreHookResult),
    /// Sync post-hook - can inspect results
    Post(fn(PostHookContext) -> PostHookResult),
    /// Async event hook
    Async(fn(HookEvent) -> Box<dyn std::future::Future<Output = HookResult> + Send>),
}

impl Hook {
    pub fn pre(name: impl Into<String>, priority: HookPriority, handler: fn(PreHookContext) -> PreHookResult) -> Self {
        Self {
            name: name.into(),
            priority,
            handler: HookHandler::Pre(handler),
        }
    }

    pub fn post(name: impl Into<String>, priority: HookPriority, handler: fn(PostHookContext) -> PostHookResult) -> Self {
        Self {
            name: name.into(),
            priority,
            handler: HookHandler::Post(handler),
        }
    }

    pub fn async_(name: impl Into<String>, priority: HookPriority, handler: fn(HookEvent) -> Box<dyn std::future::Future<Output = HookResult> + Send>) -> Self {
        Self {
            name: name.into(),
            priority,
            handler: HookHandler::Async(handler),
        }
    }
}

/// Pre-hook context
pub struct PreHookContext {
    pub event: PluginEvent,
    pub args: HashMap<String, serde_json::Value>,
}

impl PreHookContext {
    pub fn new(event: PluginEvent) -> Self {
        Self {
            event,
            args: HashMap::new(),
        }
    }
}

/// Pre-hook result
pub enum PreHookResult {
    /// Allow the operation to proceed with original args
    Allow,
    /// Allow with modified args
    Modify(HashMap<String, serde_json::Value>),
    /// Block the operation with an error message
    Block(String),
}

/// Post-hook context
pub struct PostHookContext {
    pub event: PluginEvent,
    pub result: HookResult,
}

impl PostHookContext {
    pub fn new(event: PluginEvent, result: HookResult) -> Self {
        Self { event, result }
    }
}

/// Post-hook result
pub enum PostHookResult {
    /// Continue processing
    Continue,
    /// Override the result
    Override(HookResult),
}

/// Generic hook event
pub struct HookEvent {
    pub hook_name: String,
    pub event: PluginEvent,
}

/// Hook result for post-processing
#[derive(Debug, Clone)]
pub enum HookResult {
    /// Operation succeeded
    Success,
    /// Operation failed
    Error(String),
    /// Operation was skipped
    Skipped,
}

impl From<Result<(), String>> for HookResult {
    fn from(r: Result<(), String>) -> Self {
        match r {
            Ok(()) => HookResult::Success,
            Err(e) => HookResult::Error(e),
        }
    }
}

/// Hook registry - manages all registered hooks
pub struct HookRegistry {
    pre_hooks: Vec<(HookPriority, String, fn(PreHookContext) -> PreHookResult)>,
    post_hooks: Vec<(HookPriority, String, fn(PostHookContext) -> PostHookResult)>,
    async_hooks: Vec<(HookPriority, String, fn(HookEvent) -> Box<dyn std::future::Future<Output = HookResult> + Send>)>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            pre_hooks: Vec::new(),
            post_hooks: Vec::new(),
            async_hooks: Vec::new(),
        }
    }

    /// Register a hook from a plugin
    pub fn register_hook(&self, plugin: &Arc<dyn Plugin>) {
        // Hooks are defined via the plugin trait's hooks() method
        // For now, we use a convention: plugin provides hooks via on_event
        // In the future, this could be extended with a separate Hooks trait
        tracing::debug!("Registering hooks from plugin: {}", plugin.name());
    }

    /// Register a pre-hook
    pub fn register_pre(&mut self, name: String, priority: HookPriority, handler: fn(PreHookContext) -> PreHookResult) {
        self.pre_hooks.push((priority, name, handler));
        self.pre_hooks.sort_by(|a, b| b.0.cmp(&a.0)); // Higher priority first
    }

    /// Register a post-hook
    pub fn register_post(&mut self, name: String, priority: HookPriority, handler: fn(PostHookContext) -> PostHookResult) {
        self.post_hooks.push((priority, name, handler));
        self.post_hooks.sort_by(|a, b| b.0.cmp(&a.0));
    }

    /// Register an async hook
    pub fn register_async(&mut self, name: String, priority: HookPriority, handler: fn(HookEvent) -> Box<dyn std::future::Future<Output = HookResult> + Send>) {
        self.async_hooks.push((priority, name, handler));
        self.async_hooks.sort_by(|a, b| b.0.cmp(&a.0));
    }

    /// Dispatch event to all hooks and collect actions
    pub fn dispatch(&self, event: PluginEvent) -> Vec<PluginAction> {
        let mut actions = Vec::new();

        // Run pre-hooks
        for (_, name, handler) in &self.pre_hooks {
            let ctx = PreHookContext::new(event.clone());
            match handler(ctx) {
                PreHookResult::Allow => {}
                PreHookResult::Modify(args) => {
                    // Could emit UpdateUI action to show modified args
                    tracing::debug!("Hook {} modified args: {:?}", name, args);
                }
                PreHookResult::Block(reason) => {
                    actions.push(PluginAction::ShowNotification {
                        title: "Hook Blocked".to_string(),
                        body: reason,
                        urgency: crate::NotificationUrgency::Normal,
                    });
                    return actions;
                }
            }
        }

        // Run post-hooks (would need result tracking)

        actions
    }

    /// Fire a pre-hook and get result
    pub fn fire_pre(&self, event: &PluginEvent) -> PreHookResult {
        for (_, _, handler) in &self.pre_hooks {
            let ctx = PreHookContext::new(event.clone());
            let result = handler(ctx);
            if !matches!(result, PreHookResult::Allow) {
                return result;
            }
        }
        PreHookResult::Allow
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────
// Built-in hooks
// ─────────────────────────────────────────────────────────────────

/// Register built-in system hooks
pub fn register_builtin_hooks(registry: &mut HookRegistry) {
    // File change hook
    registry.register_pre(
        "file-change".to_string(),
        HookPriority::NORMAL,
        |ctx| {
            if matches!(ctx.event, PluginEvent::FileEdited { .. }) {
                tracing::debug!("File change detected: {:?}", ctx.event);
            }
            PreHookResult::Allow
        },
    );

    // Tool call hook
    registry.register_pre(
        "tool-call".to_string(),
        HookPriority::HIGH,
        |ctx| {
            if let PluginEvent::ToolCalled { ref tool_name, .. } = ctx.event {
                tracing::debug!("Tool called: {}", tool_name);
            }
            PreHookResult::Allow
        },
    );
}
