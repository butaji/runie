use super::PluginPackage;
use std::path::{Path, PathBuf};

pub const DEFAULT_PLUGIN_TIMEOUT_MS: u64 = 30_000;
pub const MAX_PLUGIN_ARGUMENTS: usize = 64;
pub const MAX_PLUGIN_OUTPUT_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PluginExecutionRequest {
    pub entrypoint: PathBuf,
    #[serde(default)]
    pub arguments: Vec<String>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PluginExecutionResult {
    pub status: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub truncated: bool,
}

fn default_timeout() -> u64 {
    DEFAULT_PLUGIN_TIMEOUT_MS
}

pub fn validate_execution_request(request: &PluginExecutionRequest) -> Result<(), String> {
    if request.arguments.len() > MAX_PLUGIN_ARGUMENTS {
        return Err(format!(
            "plugin has too many arguments (max {MAX_PLUGIN_ARGUMENTS})"
        ));
    }
    if request.timeout_ms == 0 {
        return Err("plugin timeout must be greater than zero".into());
    }
    Ok(())
}

/// Execute one already-admitted plugin entrypoint in its package root.
/// Process ownership remains with this async host boundary; reducers consume
/// the returned data rather than a child process handle.
pub async fn execute_plugin(
    package: &PluginPackage,
    request: PluginExecutionRequest,
) -> Result<PluginExecutionResult, String> {
    validate_execution_request(&request)?;
    let entrypoint = resolve_plugin_entrypoint(package, &request.entrypoint)?;
    let output = tokio::time::timeout(
        std::time::Duration::from_millis(request.timeout_ms),
        tokio::process::Command::new(entrypoint)
            .args(&request.arguments)
            .current_dir(&package.root)
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|_| "plugin execution timed out".to_owned())?
    .map_err(|error| format!("execute plugin: {error}"))?;
    let total = output.stdout.len().saturating_add(output.stderr.len());
    let truncated = total > MAX_PLUGIN_OUTPUT_BYTES;
    let stdout =
        String::from_utf8_lossy(&output.stdout[..output.stdout.len().min(MAX_PLUGIN_OUTPUT_BYTES)])
            .into_owned();
    let stderr =
        String::from_utf8_lossy(&output.stderr[..output.stderr.len().min(MAX_PLUGIN_OUTPUT_BYTES)])
            .into_owned();
    Ok(PluginExecutionResult {
        status: output.status.code(),
        stdout,
        stderr,
        truncated,
    })
}

/// Resolve an executable declared by a plugin without allowing it to escape
/// the installed package root. The host executor owns process spawning.
pub fn resolve_plugin_entrypoint(
    package: &PluginPackage,
    entrypoint: impl AsRef<Path>,
) -> Result<PathBuf, String> {
    let entrypoint = entrypoint.as_ref();
    if entrypoint.as_os_str().is_empty() || entrypoint.is_absolute() {
        return Err("plugin entrypoint must be a non-empty relative path".into());
    }
    if entrypoint
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err("plugin entrypoint must stay within the package root".into());
    }
    let path = package.root.join(entrypoint);
    if !path.is_file() {
        return Err(format!(
            "plugin entrypoint does not exist: {}",
            path.display()
        ));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entrypoint_resolution_is_package_bounded() {
        let root = std::env::temp_dir().join(format!("runie-plugin-entry-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("run.sh"), "#!/bin/sh\n").unwrap();
        let package = PluginPackage {
            root: root.clone(),
            manifest: crate::plugins::PluginManifest {
                name: "sample-plugin".into(),
                version: "1".into(),
                commands: vec![],
                tools: vec![],
                hooks: vec![],
            },
        };
        assert_eq!(
            resolve_plugin_entrypoint(&package, "run.sh").unwrap(),
            root.join("run.sh")
        );
        assert!(resolve_plugin_entrypoint(&package, "../run.sh").is_err());
        assert!(resolve_plugin_entrypoint(&package, "/bin/sh").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn execution_requests_are_bounded_data() {
        let request = PluginExecutionRequest {
            entrypoint: PathBuf::from("run.sh"),
            arguments: vec!["ok".into()],
            timeout_ms: DEFAULT_PLUGIN_TIMEOUT_MS,
        };
        assert!(validate_execution_request(&request).is_ok());
        assert!(validate_execution_request(&PluginExecutionRequest {
            timeout_ms: 0,
            ..request.clone()
        })
        .is_err());
        assert!(validate_execution_request(&PluginExecutionRequest {
            arguments: vec!["x".into(); MAX_PLUGIN_ARGUMENTS + 1],
            ..request
        })
        .is_err());
    }
}
