use super::PluginPackage;
use std::path::{Path, PathBuf};

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
}
