//! Tool adapter for rig-core integration.
//!
//! This module provides the bridge between runie's Tool trait and rig-core's
//! ToolDyn trait. Rig-core expects tools to implement `ToolDyn`, but runie
//! defines its own `Tool` trait with different semantics. This adapter
//! translates between the two systems.
//!
//! ## Architecture
//!
//! [`RunieToolAdapter`] wraps a `Box<dyn Tool + Send + Sync>` and implements
//! rig-core's `ToolDyn` trait. The adapter handles:
//!
//! - **Name translation**: Directly exposes the tool's name
//! - **Schema conversion**: Converts runie's `ToolSchema` to rig's `ToolDefinition`
//! - **Argument parsing**: Parses JSON arguments from rig into `serde_json::Value`
//! - **Execution**: Delegates to runie's `Tool::execute()` method
//! - **Result serialization**: Converts tool output back to JSON string
//!
//! ## Usage
//!
//! ```ignore
//! use runie_tools::rig_adapter::RunieToolAdapter;
//! use runie_core::ToolRegistry;
//!
//! let registry: ToolRegistry = /* ... */;
//! let tools: Vec<Box<dyn ToolDyn>> = registry
//!     .tools()
//!     .map(|tool| Box::new(RunieToolAdapter::new(tool)) as Box<dyn ToolDyn>)
//!     .collect();
//! ```
//!
//! ## Error Handling
//!
//! The adapter maps runie tool errors to rig `ToolError` variants:
//! - `ToolError::JsonError` for serialization/deserialization failures
//! - `ToolError::ToolCallError` for tool execution failures (wrapped)

use rig_core::completion::ToolDefinition;
use rig_core::tool::{ToolDyn, ToolError};
use rig_core::wasm_compat::WasmBoxedFuture;
use runie_core::Tool;

/// Adapts a runie Tool to rig's ToolDyn trait
pub struct RunieToolAdapter {
    inner: Box<dyn Tool + Send + Sync>,
}

impl RunieToolAdapter {
    /// Creates a new adapter wrapping the given tool.
    pub fn new(tool: Box<dyn Tool + Send + Sync>) -> Self {
        Self { inner: tool }
    }
}

impl ToolDyn for RunieToolAdapter {
    fn name(&self) -> String {
        self.inner.name().to_string()
    }

    fn definition<'a>(&'a self, _prompt: String) -> WasmBoxedFuture<'a, ToolDefinition> {
        let schema = self.inner.schema();
        let def = ToolDefinition {
            name: schema.name,
            description: schema.description,
            parameters: schema.parameters,
        };
        Box::pin(async move { def })
    }

    fn call<'a>(&'a self, args: String) -> WasmBoxedFuture<'a, Result<String, ToolError>> {
        let inner = &self.inner;
        Box::pin(async move {
            let json_args: serde_json::Value = serde_json::from_str(&args)
                .map_err(|e| ToolError::JsonError(e))?;

            let output = inner.execute(json_args).await
                .map_err(|e| ToolError::ToolCallError(Box::new(e)))?;

            // Convert ToolOutput to String (JSON)
            serde_json::to_string(&output)
                .map_err(|e| ToolError::JsonError(e))
        })
    }
}
