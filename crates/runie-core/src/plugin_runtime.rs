use super::PluginRegistry;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginRuntimeStatus {
    Loading,
    Ready,
    Executing,
    Failed,
    Unloading,
    Unloaded,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PluginRuntimeState {
    pub status: BTreeMap<String, PluginRuntimeStatus>,
    pub errors: BTreeMap<String, String>,
    #[serde(default)]
    pub executions: BTreeMap<String, PluginExecutionSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PluginExecutionSummary {
    pub status: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub truncated: bool,
}

impl PluginRuntimeState {
    /// Stable renderer-neutral rows for plugin lifecycle inspection.
    pub fn terminal_lines(&self) -> Vec<String> {
        let mut lines =
            Vec::with_capacity(self.status.len() + self.errors.len() + self.executions.len());
        for (name, status) in &self.status {
            lines.push(format!("Plugin {name}: {status:?}"));
            if let Some(error) = self.errors.get(name) {
                lines.push(format!("Plugin {name} error: {error}"));
            }
            if let Some(execution) = self.executions.get(name) {
                lines.push(format!(
                    "Plugin {name} execution: status={:?} stdout={:?} stderr={:?} truncated={}",
                    execution.status, execution.stdout, execution.stderr, execution.truncated
                ));
            }
        }
        lines
    }
}

impl Default for PluginRuntimeState {
    fn default() -> Self {
        Self {
            status: BTreeMap::new(),
            errors: BTreeMap::new(),
            executions: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PluginRuntimeEvent {
    LoadStarted {
        name: String,
    },
    LoadSucceeded {
        name: String,
    },
    LoadFailed {
        name: String,
        error: String,
    },
    UnloadStarted {
        name: String,
    },
    Unloaded {
        name: String,
    },
    ExecuteStarted {
        name: String,
    },
    ExecuteFinished {
        name: String,
        status: Option<i32>,
        stdout: String,
        stderr: String,
        truncated: bool,
    },
    ExecuteFailed {
        name: String,
        error: String,
    },
}

fn event_data(event: PluginRuntimeEvent) -> (String, PluginRuntimeStatus, Option<String>) {
    match event {
        PluginRuntimeEvent::LoadStarted { name } => (name, PluginRuntimeStatus::Loading, None),
        PluginRuntimeEvent::LoadSucceeded { name } => (name, PluginRuntimeStatus::Ready, None),
        PluginRuntimeEvent::LoadFailed { name, error } => {
            (name, PluginRuntimeStatus::Failed, Some(error))
        }
        PluginRuntimeEvent::UnloadStarted { name } => (name, PluginRuntimeStatus::Unloading, None),
        PluginRuntimeEvent::Unloaded { name } => (name, PluginRuntimeStatus::Unloaded, None),
        PluginRuntimeEvent::ExecuteStarted { .. }
        | PluginRuntimeEvent::ExecuteFinished { .. }
        | PluginRuntimeEvent::ExecuteFailed { .. } => {
            unreachable!("execution events are handled by the runtime reducer")
        }
    }
}

fn allows_transition(current: Option<&PluginRuntimeStatus>, next: &PluginRuntimeStatus) -> bool {
    match next {
        PluginRuntimeStatus::Loading => matches!(
            current,
            None | Some(PluginRuntimeStatus::Failed) | Some(PluginRuntimeStatus::Unloaded)
        ),
        PluginRuntimeStatus::Ready | PluginRuntimeStatus::Failed => {
            matches!(current, Some(PluginRuntimeStatus::Loading))
        }
        PluginRuntimeStatus::Executing => matches!(current, Some(PluginRuntimeStatus::Ready)),
        PluginRuntimeStatus::Unloading => matches!(
            current,
            Some(PluginRuntimeStatus::Ready) | Some(PluginRuntimeStatus::Failed)
        ),
        PluginRuntimeStatus::Unloaded => matches!(current, Some(PluginRuntimeStatus::Unloading)),
    }
}

pub fn reduce_plugin_runtime(
    registry: &PluginRegistry,
    state: &mut PluginRuntimeState,
    event: PluginRuntimeEvent,
) -> Result<(), String> {
    if matches!(
        event,
        PluginRuntimeEvent::ExecuteStarted { .. }
            | PluginRuntimeEvent::ExecuteFinished { .. }
            | PluginRuntimeEvent::ExecuteFailed { .. }
    ) {
        return reduce_execution_event(registry, state, event);
    }
    let (name, status, error) = event_data(event);
    ensure_registered(registry, &name)?;
    let current = state.status.get(&name);
    if !allows_transition(current, &status) {
        return Err(format!(
            "invalid plugin runtime transition for {name}: {:?} -> {:?}",
            current, status
        ));
    }
    if let Some(error) = error {
        if error.trim().is_empty() {
            return Err("plugin runtime error must not be empty".into());
        }
        state.errors.insert(name.clone(), error);
    } else if matches!(
        &status,
        PluginRuntimeStatus::Ready | PluginRuntimeStatus::Unloaded
    ) {
        state.errors.remove(&name);
    }
    state.status.insert(name, status);
    Ok(())
}

fn reduce_execution_event(
    registry: &PluginRegistry,
    state: &mut PluginRuntimeState,
    event: PluginRuntimeEvent,
) -> Result<(), String> {
    match event {
        PluginRuntimeEvent::ExecuteStarted { name } => start_execution(registry, state, name)?,
        PluginRuntimeEvent::ExecuteFinished {
            name,
            status,
            stdout,
            stderr,
            truncated,
        } => finish_execution(registry, state, name, status, stdout, stderr, truncated)?,
        PluginRuntimeEvent::ExecuteFailed { name, error } => {
            fail_execution(registry, state, name, error)?
        }
        _ => unreachable!("non-execution event passed to execution reducer"),
    }
    Ok(())
}

fn start_execution(
    registry: &PluginRegistry,
    state: &mut PluginRuntimeState,
    name: String,
) -> Result<(), String> {
    ensure_registered(registry, &name)?;
    if !matches!(state.status.get(&name), Some(PluginRuntimeStatus::Ready)) {
        return Err(format!("plugin is not ready to execute: {name}"));
    }
    state.status.insert(name, PluginRuntimeStatus::Executing);
    Ok(())
}

fn finish_execution(
    registry: &PluginRegistry,
    state: &mut PluginRuntimeState,
    name: String,
    status: Option<i32>,
    stdout: String,
    stderr: String,
    truncated: bool,
) -> Result<(), String> {
    ensure_registered(registry, &name)?;
    require_executing(state, &name)?;
    state
        .status
        .insert(name.clone(), PluginRuntimeStatus::Ready);
    state.executions.insert(
        name,
        PluginExecutionSummary {
            status,
            stdout,
            stderr,
            truncated,
        },
    );
    Ok(())
}

fn fail_execution(
    registry: &PluginRegistry,
    state: &mut PluginRuntimeState,
    name: String,
    error: String,
) -> Result<(), String> {
    ensure_registered(registry, &name)?;
    require_executing(state, &name)?;
    if error.trim().is_empty() {
        return Err("plugin execution error must not be empty".into());
    }
    state
        .status
        .insert(name.clone(), PluginRuntimeStatus::Ready);
    state.errors.insert(name, error);
    Ok(())
}

fn require_executing(state: &PluginRuntimeState, name: &str) -> Result<(), String> {
    if matches!(state.status.get(name), Some(PluginRuntimeStatus::Executing)) {
        Ok(())
    } else {
        Err(format!("plugin is not executing: {name}"))
    }
}

fn ensure_registered(registry: &PluginRegistry, name: &str) -> Result<(), String> {
    registry
        .get(name)
        .map(|_| ())
        .ok_or_else(|| format!("plugin is not registered: {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::PluginManifest;

    fn sample_registry() -> PluginRegistry {
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
        registry
    }

    fn apply(registry: &PluginRegistry, state: &mut PluginRuntimeState, event: PluginRuntimeEvent) {
        reduce_plugin_runtime(registry, state, event).unwrap();
    }

    #[test]
    fn runtime_failure_and_recovery_are_replayable() {
        let registry = sample_registry();
        let mut state = PluginRuntimeState::default();
        apply(
            &registry,
            &mut state,
            PluginRuntimeEvent::LoadStarted {
                name: "sample-plugin".into(),
            },
        );
        apply(
            &registry,
            &mut state,
            PluginRuntimeEvent::LoadFailed {
                name: "sample-plugin".into(),
                error: "missing host".into(),
            },
        );
        assert_eq!(state.status["sample-plugin"], PluginRuntimeStatus::Failed);
        apply(
            &registry,
            &mut state,
            PluginRuntimeEvent::LoadStarted {
                name: "sample-plugin".into(),
            },
        );
        apply(
            &registry,
            &mut state,
            PluginRuntimeEvent::LoadSucceeded {
                name: "sample-plugin".into(),
            },
        );
        assert_eq!(state.status["sample-plugin"], PluginRuntimeStatus::Ready);
    }

    #[test]
    fn execution_events_replay_bounded_result_data() {
        let registry = sample_registry();
        let mut state = PluginRuntimeState::default();
        apply(
            &registry,
            &mut state,
            PluginRuntimeEvent::LoadStarted {
                name: "sample-plugin".into(),
            },
        );
        apply(
            &registry,
            &mut state,
            PluginRuntimeEvent::LoadSucceeded {
                name: "sample-plugin".into(),
            },
        );
        apply(
            &registry,
            &mut state,
            PluginRuntimeEvent::ExecuteStarted {
                name: "sample-plugin".into(),
            },
        );
        apply(
            &registry,
            &mut state,
            PluginRuntimeEvent::ExecuteFinished {
                name: "sample-plugin".into(),
                status: Some(0),
                stdout: "ok".into(),
                stderr: String::new(),
                truncated: false,
            },
        );
        assert_eq!(state.status["sample-plugin"], PluginRuntimeStatus::Ready);
        assert_execution_row(&state);
    }

    fn assert_execution_row(state: &PluginRuntimeState) {
        assert_eq!(state.executions["sample-plugin"].status, Some(0));
        assert!(state
            .terminal_lines()
            .iter()
            .any(|line| line.contains("Plugin sample-plugin execution: status=Some(0)")));
    }

    #[test]
    fn runtime_projection_replays_failure_rows_in_stable_order() {
        let registry = sample_registry();
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
        assert_eq!(
            state.terminal_lines(),
            [
                "Plugin sample-plugin: Failed",
                "Plugin sample-plugin error: missing host",
            ]
        );
    }

    #[test]
    fn runtime_rejects_out_of_order_events() {
        let registry = sample_registry();
        let mut state = PluginRuntimeState::default();
        let error = reduce_plugin_runtime(
            &registry,
            &mut state,
            PluginRuntimeEvent::LoadSucceeded {
                name: "sample-plugin".into(),
            },
        )
        .unwrap_err();
        assert!(error.contains("invalid plugin runtime transition"));
        assert!(state.status.is_empty());
    }
}
