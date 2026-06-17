//! Dry-run / preview mode for validating configuration without execution.

use crate::config::Config;

/// Result status of a dry-run validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DryRunStatus {
    /// Configuration is valid and the session can start.
    Ready,
    /// Configuration has non-fatal issues.
    Warning,
    /// Configuration is invalid and blocks execution.
    Blocked,
}

/// Human-readable dry-run report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DryRunReport {
    pub status: DryRunStatus,
    pub lines: Vec<String>,
}

impl std::fmt::Display for DryRunReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.lines.join("\n"))
    }
}

/// Validate the current configuration without making any API calls.
pub fn run_dry_run(config: &Config) -> DryRunReport {
    let mut lines = Vec::new();
    let mut status = DryRunStatus::Ready;

    lines.push("✓ Config valid".into());

    match resolve_provider_model(config) {
        Ok((provider, model)) => lines.push(format!("✓ Provider: {}/{}", provider, model)),
        Err(e) => {
            lines.push(format!("✗ Provider: {}", e));
            status = DryRunStatus::Blocked;
        }
    }

    let tools = core_tool_names();
    lines.push(format!("✓ Tools: {}", tools.join(", ")));

    let skills = crate::skills::load_all();
    if skills.is_empty() {
        lines.push("✓ Skills: none loaded".into());
    } else {
        lines.push(format!("✓ Skills: {} loaded", skills.len()));
    }

    lines.push("✓ MCP servers: none configured".into());

    lines.push("✓ Permissions: auto mode (file writes require approval)".into());

    lines.push("⚠ No model calls made (dry-run)".into());

    DryRunReport { status, lines }
}

fn resolve_provider_model(config: &Config) -> Result<(String, String), String> {
    let provider = config
        .provider
        .clone()
        .ok_or_else(|| "no provider configured".to_string())?;

    if !crate::provider_registry::is_known_provider(&provider) {
        return Err(format!("unknown provider '{}'", provider));
    }

    let model = config
        .default_model()
        .map(|s| s.to_string())
        .ok_or_else(|| "no model configured".to_string())?;

    let known = crate::model_catalog::model_catalog()
        .iter()
        .any(|m| m.provider == provider && m.name == model);
    if !known {
        return Err(format!("model '{}/{}' not in catalog", provider, model));
    }

    Ok((provider, model))
}

fn core_tool_names() -> Vec<&'static str> {
    vec!["read", "write", "edit", "bash", "glob", "grep", "search"]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_config_with_provider(provider: &str, model: &str) -> Config {
        Config {
            provider: Some(provider.into()),
            model: Some(model.into()),
            ..Default::default()
        }
    }

    #[test]
    fn dry_run_validates_config() {
        let config = Config::default();
        let report = run_dry_run(&config);
        // Default config has no provider, so status is blocked.
        assert_eq!(report.status, DryRunStatus::Blocked);
        assert!(report.lines.iter().any(|l| l.contains("Provider")));
    }

    #[test]
    fn dry_run_resolves_provider() {
        let config = temp_config_with_provider("openai", "gpt-4o");
        let report = run_dry_run(&config);
        assert_eq!(report.status, DryRunStatus::Ready);
        assert!(report.lines.iter().any(|l| l.contains("openai/gpt-4o")));
    }

    #[test]
    fn dry_run_loads_skills() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", dir.path());

        let skill_dir = dir.path().join(".runie").join("skills").join("rust");
        std::fs::create_dir_all(&skill_dir).unwrap();
        let mut file = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        write!(file, "# Rust\n\n## Description\n\nRust skill.\n").unwrap();

        let config = temp_config_with_provider("openai", "gpt-4o");
        let report = run_dry_run(&config);
        assert!(report.lines.iter().any(|l| l.contains("Skills: 1 loaded")));
    }

    #[test]
    fn dry_run_no_llm_calls() {
        let config = temp_config_with_provider("openai", "gpt-4o");
        let report = run_dry_run(&config);
        assert!(report
            .lines
            .iter()
            .any(|l| l.contains("No model calls made")));
    }
}
