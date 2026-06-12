//! Plugin trait and core types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

/// Core plugin trait - implemented by all extension types
///
/// # Example
///
/// ```ignore
/// use runie_ext::{Plugin, PluginEvent, PluginAction, SlashCommand};
///
/// struct MyPlugin;
///
/// #[runie_ext::plugin]
/// impl Plugin for MyPlugin {
///     fn name(&self) -> &str { "my-plugin" }
///     fn version(&self) -> &str { "0.1.0" }
///
///     fn commands(&self) -> Vec<SlashCommand> {
///         vec![SlashCommand::Custom("hello".to_string())]
///     }
///
///     fn on_event(&self, event: PluginEvent) -> Vec<PluginAction> {
///         vec![]
///     }
/// }
/// ```
pub trait Plugin: Send + Sync {
    /// Unique identifier for this plugin
    fn name(&self) -> &str;

    /// Semantic version string
    fn version(&self) -> &str;

    /// Plugin description for marketplace/menus
    fn description(&self) -> Option<&str> { None }

    /// Dynamic commands provided by this plugin
    /// These are merged into the command palette at runtime
    fn commands(&self) -> Vec<PluginCommand> { vec![] }

    /// Process events from runie and optionally emit actions
    fn on_event(&self, event: PluginEvent) -> Vec<PluginAction> { vec![] }

    /// Lifecycle hooks
    fn on_load(&self) -> Result<(), String> { Ok(()) }
    fn on_unload(&self) -> Result<(), String> { Ok(()) }
}

/// Plugin metadata - lightweight info for registry/listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub extension_type: ExtensionType,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtensionType {
    Plugin,
    Hook,
    Skill,
    McpServer,
}

/// Extension-provided command for the command palette
#[derive(Debug, Clone)]
pub struct PluginCommand {
    pub name: String,
    pub description: String,
    pub handler: CommandHandler,
}

impl PluginCommand {
    pub fn new(name: impl Into<String>, description: impl Into<String>, handler: CommandHandler) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            handler,
        }
    }
}

/// Handler for plugin commands - takes parsed args, returns action(s)
pub enum CommandHandler {
    /// Sync handler for simple commands
    Sync(Arc<dyn Fn(&[String]) -> Vec<PluginAction> + Send + Sync>),
    /// Async handler for complex operations
    Async(Arc<dyn Fn(&[String]) -> Box<dyn std::future::Future<Output = Vec<PluginAction>> + Send> + Send + Sync>),
}

impl Debug for CommandHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandHandler::Sync(_) => write!(f, "CommandHandler::Sync(...)"),
            CommandHandler::Async(_) => write!(f, "CommandHandler::Async(...)"),
        }
    }
}

impl Clone for CommandHandler {
    fn clone(&self) -> Self {
        match self {
            CommandHandler::Sync(x) => CommandHandler::Sync(Arc::clone(&x)),
            CommandHandler::Async(x) => CommandHandler::Async(Arc::clone(&x)),
        }
    }
}

// Note: Async handlers must be converted via a helper, not From
impl CommandHandler {
    pub fn from_async<F, Fut>(f: F) -> Self
    where
        F: Fn(&[String]) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Vec<PluginAction>> + Send + 'static,
    {
        CommandHandler::Async(Arc::new(move |args| {
            Box::new(f(args)) as Box<dyn std::future::Future<Output = Vec<PluginAction>> + Send>
        }))
    }
}

impl<F> From<F> for CommandHandler
where
    F: Fn(&[String]) -> Vec<PluginAction> + Send + Sync + 'static,
{
    fn from(f: F) -> Self {
        CommandHandler::Sync(Arc::new(f))
    }
}

/// Events that plugins can intercept
#[derive(Debug, Clone, Serialize)]
pub enum PluginEvent {
    // ─────────────────────────────────────────────────────────────
    // Session Events
    // ─────────────────────────────────────────────────────────────
    SessionStarted {
        session_id: String,
    },
    SessionEnded {
        session_id: String,
    },

    // ─────────────────────────────────────────────────────────────
    // Message Events
    // ─────────────────────────────────────────────────────────────
    MessageReceived {
        role: MessageRole,
        content: String,
    },
    MessageSent {
        content: String,
    },
    MessageDelta {
        content: String,
    },

    // ─────────────────────────────────────────────────────────────
    // Tool Events
    // ─────────────────────────────────────────────────────────────
    ToolCalled {
        tool_name: String,
        arguments: serde_json::Value,
    },
    ToolResult {
        tool_name: String,
        output: runie_core::ToolOutput,
    },
    ToolError {
        tool_name: String,
        error: String,
    },

    // ─────────────────────────────────────────────────────────────
    // File Events
    // ─────────────────────────────────────────────────────────────
    FileEdited {
        path: String,
        change_type: FileChangeType,
    },

    // ─────────────────────────────────────────────────────────────
    // UI Events
    // ─────────────────────────────────────────────────────────────
    UiReady,
    CommandPaletteOpened,
    CommandPaletteClosed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FileChangeType {
    Created,
    Modified,
    Deleted,
}

/// Actions that plugins can emit in response to events
#[derive(Debug, Clone, Serialize)]
pub enum PluginAction {
    // ─────────────────────────────────────────────────────────────
    // Notifications
    // ─────────────────────────────────────────────────────────────
    ShowNotification {
        title: String,
        body: String,
        urgency: NotificationUrgency,
    },
    ShowToast {
        message: String,
        duration_ms: u32,
    },

    // ─────────────────────────────────────────────────────────────
    // Tool Actions
    // ─────────────────────────────────────────────────────────────
    RunTool {
        name: String,
        arguments: serde_json::Value,
    },
    CancelTool {
        tool_call_id: String,
    },

    // ─────────────────────────────────────────────────────────────
    // UI Actions
    // ─────────────────────────────────────────────────────────────
    UpdateUI {
        component: UiComponent,
        payload: serde_json::Value,
    },
    OpenPanel {
        panel: Panel,
    },
    ClosePanel {
        panel: Panel,
    },

    // ─────────────────────────────────────────────────────────────
    // Command Actions
    // ─────────────────────────────────────────────────────────────
    ExecuteCommand {
        command: String,
        args: HashMap<String, String>,
    },

    // ─────────────────────────────────────────────────────────────
    // Context Actions
    // ─────────────────────────────────────────────────────────────
    AppendToContext {
        content: String,
    },
    PrependToContext {
        content: String,
    },

    // ─────────────────────────────────────────────────────────────
    // Extension Actions
    // ─────────────────────────────────────────────────────────────
    /// Invoke another plugin/skill
    InvokeSkill {
        skill_id: String,
        input: String,
    },
    /// Emit raw event to other plugins
    EmitEvent {
        event: Box<PluginEvent>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum NotificationUrgency {
    Low,
    Normal,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum UiComponent {
    StatusBar,
    SidePanel,
    CommandPalette,
    MessageList,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Panel {
    Hooks,
    Plugins,
    Skills,
    Mcp,
    Settings,
}

/// Slash command representation for plugin commands
///
/// Note: This is distinct from runie_core::SlashCommand to allow
/// plugins to define custom commands without modifying the core enum
#[derive(Debug, Clone)]
pub struct SlashCommand {
    pub name: String,
    pub aliases: Vec<String>,
    pub description: String,
    pub args: Vec<CommandArg>,
}

#[derive(Debug, Clone)]
pub struct CommandArg {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default: Option<String>,
}

// ─────────────────────────────────────────────────────────────────
// Plugin Macro - compile-time registration
// ─────────────────────────────────────────────────────────────────

/// Register a plugin with the global registry at compile time
///
/// ```ignore
/// #[runie_ext::plugin]
/// static MY_PLUGIN: MyPlugin = MyPlugin;
/// ```
pub use runie_ext_macros::plugin;

// ─────────────────────────────────────────────────────────────────
// ExtensionInstance - boxed plugin for storage
// ─────────────────────────────────────────────────────────────────

/// Type-erased plugin container
pub type ExtensionInstance = Arc<dyn Plugin>;

impl dyn Plugin {
    pub fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: self.name().to_string(),
            name: self.name().to_string(),
            version: self.version().to_string(),
            description: self.description().map(String::from),
            author: None,
            extension_type: ExtensionType::Plugin,
            dependencies: vec![],
        }
    }
}
