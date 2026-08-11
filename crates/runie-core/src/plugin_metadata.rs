use super::PluginManifest;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub commands: usize,
    pub tools: usize,
    pub hooks: usize,
    pub capabilities: Vec<String>,
}

impl PluginMetadata {
    pub fn from_manifest(manifest: &PluginManifest) -> Self {
        Self {
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            commands: manifest.commands.len(),
            tools: manifest.tools.len(),
            hooks: manifest.hooks.len(),
            capabilities: manifest
                .commands
                .iter()
                .map(|name| format!("command:{name}"))
                .chain(manifest.tools.iter().map(|name| format!("tool:{name}")))
                .chain(manifest.hooks.iter().map(|name| format!("hook:{name}")))
                .collect(),
        }
    }

    pub fn terminal_line(&self) -> String {
        let capabilities = self.capabilities.join(",");
        let summary = format!(
            "Plugin {} v{} · commands={} tools={} hooks={}",
            self.name, self.version, self.commands, self.tools, self.hooks
        );
        if capabilities.is_empty() {
            summary
        } else {
            format!("{summary} capabilities={capabilities}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_projects_manifest_capabilities_as_stable_data() {
        let manifest = PluginManifest {
            name: "sample-plugin".into(),
            version: "1.0.0".into(),
            commands: vec!["format".into()],
            tools: vec!["inspect".into()],
            hooks: vec!["after_turn".into()],
        };
        let metadata = PluginMetadata::from_manifest(&manifest);
        assert_eq!(metadata.commands, 1);
        assert_eq!(metadata.tools, 1);
        assert_eq!(metadata.hooks, 1);
        assert_eq!(
            metadata.capabilities,
            ["command:format", "tool:inspect", "hook:after_turn"]
        );
        assert_eq!(
            metadata.terminal_line(),
            "Plugin sample-plugin v1.0.0 · commands=1 tools=1 hooks=1 capabilities=command:format,tool:inspect,hook:after_turn"
        );
    }
}
