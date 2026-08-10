use crate::plugins::{PluginCapabilityKind, PluginHost};
use crate::types::{AgentTool, AgentToolResult, ToolResultContent};
use serde_json::Value;
use std::sync::Arc;

/// Tool-registry adapter for a manifest-declared plugin capability.
#[derive(Clone)]
pub struct PluginTool {
    host: PluginHost,
    plugin: String,
    capability: String,
    name: String,
    label: String,
    description: String,
    timeout_ms: u64,
}

impl PluginTool {
    pub fn new(
        host: PluginHost,
        plugin: impl Into<String>,
        capability: impl Into<String>,
        timeout_ms: u64,
    ) -> Self {
        let capability = capability.into();
        Self {
            host,
            plugin: plugin.into(),
            name: capability.clone(),
            label: capability.clone(),
            description: format!("Plugin capability: {capability}"),
            capability,
            timeout_ms,
        }
    }

    fn with_registry_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }
}

#[async_trait::async_trait]
impl AgentTool for PluginTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn label(&self) -> &str {
        &self.label
    }
    fn description(&self) -> &str {
        &self.description
    }

    async fn execute(
        &self,
        _tool_call_id: &str,
        args: Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _on_update: Option<Box<dyn Fn(Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        let arguments = match args {
            Value::Null => Vec::new(),
            value => vec![serde_json::to_string(&value).map_err(|error| error.to_string())?],
        };
        let result = self
            .host
            .execute_capability(
                &self.plugin,
                PluginCapabilityKind::Tool,
                &self.capability,
                arguments,
                self.timeout_ms,
            )
            .await?;
        Ok(AgentToolResult {
            content: vec![ToolResultContent::Text {
                text: result.stdout,
            }],
            details: serde_json::json!({"stderr": result.stderr, "truncated": result.truncated}),
            usage: None,
            added_tool_names: Vec::new(),
            terminate: false,
        })
    }
}

pub fn plugin_tool(
    host: PluginHost,
    plugin: &str,
    capability: &str,
    timeout_ms: u64,
) -> Arc<dyn AgentTool> {
    Arc::new(PluginTool::new(host, plugin, capability, timeout_ms))
}

pub fn register_plugin_tool(
    registry: &mut crate::tools::ToolRegistry,
    host: PluginHost,
    plugin: &str,
    capability: &str,
    timeout_ms: u64,
) {
    registry.register(plugin_tool(host, plugin, capability, timeout_ms));
}

pub fn register_plugin_manifest_tools(
    registry: &mut crate::tools::ToolRegistry,
    host: PluginHost,
    manifest: &crate::plugins::PluginManifest,
    timeout_ms: u64,
) -> Result<usize, String> {
    let mut capabilities = manifest.tools.clone();
    capabilities.sort();
    let names: Vec<_> = capabilities
        .iter()
        .map(|capability| format!("plugin__{}__{}", manifest.name, capability))
        .collect();
    if names.iter().any(|name| registry.lookup(name).is_some()) {
        return Err("a plugin tool with that qualified name is already registered".into());
    }
    for (capability, name) in capabilities.iter().zip(names) {
        let tool = PluginTool::new(host.clone(), &manifest.name, capability, timeout_ms)
            .with_registry_name(name);
        registry.register(Arc::new(tool));
    }
    Ok(capabilities.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::PluginRegistry;
    const TEST_PLUGIN_TIMEOUT_MS: u64 = 1_000;

    #[tokio::test]
    async fn plugin_tool_is_registry_data_with_stable_identity() {
        let registry = PluginRegistry::default();
        let host = PluginHost::new(registry, Default::default());
        let tool = plugin_tool(host, "demo", "inspect", TEST_PLUGIN_TIMEOUT_MS);
        assert_eq!(tool.name(), "inspect");
        assert_eq!(tool.label(), "inspect");
        assert!(tool.description().contains("inspect"));
    }

    #[tokio::test]
    async fn manifest_tools_register_sorted_and_qualified() {
        let mut registry = crate::tools::ToolRegistry::new();
        let host = PluginHost::new(PluginRegistry::default(), Vec::new());
        let manifest = crate::plugins::PluginManifest {
            name: "demo".into(),
            version: "1".into(),
            commands: Vec::new(),
            tools: vec!["write".into(), "inspect".into()],
            hooks: Vec::new(),
        };
        assert_eq!(
            register_plugin_manifest_tools(&mut registry, host, &manifest, TEST_PLUGIN_TIMEOUT_MS,)
                .unwrap(),
            2
        );
        assert_eq!(
            registry.names(),
            vec!["plugin__demo__inspect", "plugin__demo__write"]
        );
    }
}
