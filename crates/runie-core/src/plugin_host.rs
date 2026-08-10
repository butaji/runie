use crate::plugins::{
    execute_plugin, reduce_plugin_runtime, PluginExecutionRequest, PluginExecutionResult,
    PluginPackage, PluginRegistry, PluginRuntimeEvent, PluginRuntimeState,
};
use crate::ReducerActor;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapabilityKind {
    Command,
    Tool,
    Hook,
}

pub fn capability_entrypoint(kind: PluginCapabilityKind, name: &str) -> std::path::PathBuf {
    let directory = match kind {
        PluginCapabilityKind::Command => "commands",
        PluginCapabilityKind::Tool => "tools",
        PluginCapabilityKind::Hook => "hooks",
    };
    std::path::PathBuf::from(directory).join(name)
}

/// Actor-owned bridge between installed plugin packages and replayable runtime
/// state. Process handles never cross the snapshot boundary.
#[derive(Clone)]
pub struct PluginHost {
    registry: PluginRegistry,
    packages: BTreeMap<String, PluginPackage>,
    runtime: ReducerActor<PluginRuntimeState, PluginRuntimeEvent>,
}

impl PluginHost {
    pub fn new(registry: PluginRegistry, packages: Vec<PluginPackage>) -> Self {
        let package_map = packages
            .into_iter()
            .map(|package| (package.manifest.name.clone(), package))
            .collect::<BTreeMap<_, _>>();
        let reducer_registry = registry.clone();
        let runtime = ReducerActor::new(64, PluginRuntimeState::default(), move |state, event| {
            let _ = reduce_plugin_runtime(&reducer_registry, state, event);
        });
        Self {
            registry,
            packages: package_map,
            runtime,
        }
    }

    pub fn snapshot(&self) -> PluginRuntimeState {
        self.runtime.snapshot()
    }

    pub fn shared_snapshot(&self) -> crate::SharedSnapshot<PluginRuntimeState> {
        self.runtime.shared_snapshot()
    }

    pub fn shared_subscribe(
        &self,
    ) -> tokio::sync::watch::Receiver<crate::SharedSnapshot<PluginRuntimeState>> {
        self.runtime.shared_subscribe()
    }

    pub async fn execute(
        &self,
        plugin: &str,
        request: PluginExecutionRequest,
    ) -> Result<PluginExecutionResult, String> {
        let package = self
            .packages
            .get(plugin)
            .ok_or_else(|| format!("plugin package is not installed: {plugin}"))?
            .clone();
        self.apply_checked(PluginRuntimeEvent::ExecuteStarted {
            name: plugin.into(),
        })
        .await?;
        match execute_plugin(&package, request).await {
            Ok(result) => {
                self.apply_checked(PluginRuntimeEvent::ExecuteFinished {
                    name: plugin.into(),
                    status: result.status,
                    stdout: result.stdout.clone(),
                    stderr: result.stderr.clone(),
                    truncated: result.truncated,
                })
                .await?;
                Ok(result)
            }
            Err(error) => {
                let _ = self
                    .apply_checked(PluginRuntimeEvent::ExecuteFailed {
                        name: plugin.into(),
                        error: error.clone(),
                    })
                    .await;
                Err(error)
            }
        }
    }

    pub async fn execute_capability(
        &self,
        plugin: &str,
        kind: PluginCapabilityKind,
        capability: &str,
        arguments: Vec<String>,
        timeout_ms: u64,
    ) -> Result<PluginExecutionResult, String> {
        self.execute(
            plugin,
            PluginExecutionRequest {
                entrypoint: capability_entrypoint(kind, capability),
                arguments,
                timeout_ms,
            },
        )
        .await
    }

    async fn apply_checked(&self, event: PluginRuntimeEvent) -> Result<(), String> {
        let mut candidate = self.snapshot();
        reduce_plugin_runtime(&self.registry, &mut candidate, event.clone())?;
        if self.runtime.apply(event).await {
            Ok(())
        } else {
            Err("plugin runtime actor is closed".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn host_rejects_execution_for_uninstalled_plugin() {
        let host = PluginHost::new(PluginRegistry::default(), Vec::new());
        let error = host
            .execute(
                "missing",
                PluginExecutionRequest {
                    entrypoint: "run.sh".into(),
                    arguments: vec![],
                    timeout_ms: 1,
                },
            )
            .await
            .unwrap_err();
        assert!(error.contains("not installed"));
    }

    #[tokio::test]
    async fn host_exposes_immutable_runtime_projection() {
        let host = PluginHost::new(PluginRegistry::default(), Vec::new());
        assert_eq!(host.shared_snapshot().get(), &PluginRuntimeState::default());
        assert_eq!(host.shared_snapshot().strong_count(), 2);
        assert_eq!(
            host.shared_subscribe().borrow().get(),
            &PluginRuntimeState::default()
        );
    }

    #[test]
    fn capability_entrypoints_are_typed_package_relative_data() {
        assert_eq!(
            capability_entrypoint(PluginCapabilityKind::Tool, "inspect"),
            std::path::PathBuf::from("tools/inspect")
        );
        assert_eq!(
            capability_entrypoint(PluginCapabilityKind::Hook, "after_turn"),
            std::path::PathBuf::from("hooks/after_turn")
        );
    }
}
