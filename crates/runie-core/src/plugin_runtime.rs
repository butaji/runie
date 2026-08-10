use super::PluginRegistry;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginRuntimeStatus {
    Loading,
    Ready,
    Failed,
    Unloading,
    Unloaded,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PluginRuntimeState {
    pub status: BTreeMap<String, PluginRuntimeStatus>,
    pub errors: BTreeMap<String, String>,
}

impl Default for PluginRuntimeState {
    fn default() -> Self {
        Self {
            status: BTreeMap::new(),
            errors: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PluginRuntimeEvent {
    LoadStarted { name: String },
    LoadSucceeded { name: String },
    LoadFailed { name: String, error: String },
    UnloadStarted { name: String },
    Unloaded { name: String },
}

pub fn reduce_plugin_runtime(
    registry: &PluginRegistry,
    state: &mut PluginRuntimeState,
    event: PluginRuntimeEvent,
) -> Result<(), String> {
    let (name, status, error) = match event {
        PluginRuntimeEvent::LoadStarted { name } => (name, PluginRuntimeStatus::Loading, None),
        PluginRuntimeEvent::LoadSucceeded { name } => (name, PluginRuntimeStatus::Ready, None),
        PluginRuntimeEvent::LoadFailed { name, error } => {
            (name, PluginRuntimeStatus::Failed, Some(error))
        }
        PluginRuntimeEvent::UnloadStarted { name } => (name, PluginRuntimeStatus::Unloading, None),
        PluginRuntimeEvent::Unloaded { name } => (name, PluginRuntimeStatus::Unloaded, None),
    };
    if registry.get(&name).is_none() {
        return Err(format!("plugin is not registered: {name}"));
    }
    if let Some(error) = error {
        if error.trim().is_empty() {
            return Err("plugin runtime error must not be empty".into());
        }
        state.errors.insert(name.clone(), error);
    } else if matches!(
        status,
        PluginRuntimeStatus::Ready | PluginRuntimeStatus::Unloaded
    ) {
        state.errors.remove(&name);
    }
    state.status.insert(name, status);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::PluginManifest;

    #[test]
    fn runtime_failure_and_recovery_are_replayable() {
        let mut registry = PluginRegistry::default();
        registry
            .register(PluginManifest {
                name: "sample-plugin".into(),
                version: "1".into(),
                commands: vec![],
                tools: vec![],
                hooks: vec![],
            })
            .unwrap();
        let mut state = PluginRuntimeState::default();
        reduce_plugin_runtime(
            &registry,
            &mut state,
            PluginRuntimeEvent::LoadStarted {
                name: "sample-plugin".into(),
            },
        )
        .unwrap();
        reduce_plugin_runtime(
            &registry,
            &mut state,
            PluginRuntimeEvent::LoadFailed {
                name: "sample-plugin".into(),
                error: "missing host".into(),
            },
        )
        .unwrap();
        assert_eq!(state.status["sample-plugin"], PluginRuntimeStatus::Failed);
        reduce_plugin_runtime(
            &registry,
            &mut state,
            PluginRuntimeEvent::LoadSucceeded {
                name: "sample-plugin".into(),
            },
        )
        .unwrap();
        assert_eq!(state.status["sample-plugin"], PluginRuntimeStatus::Ready);
    }
}
