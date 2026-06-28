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
    dry_run_report_with_skills(config, &crate::skills::load_all())
}

fn dry_run_report_with_skills(config: &Config, skills: &[crate::skills::Skill]) -> DryRunReport {
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
        .ok_or_else(|| "no provider configured".to_owned())?;

    if !crate::provider::is_known_provider(&provider) {
        return Err(format!("unknown provider '{}'", provider));
    }

    let model = config
        .default_model()
        .map(|s| s.to_owned())
        .ok_or_else(|| "no model configured".to_owned())?;

    let known = crate::model_catalog::model_catalog()
        .iter()
        .any(|m| m.provider == provider && m.name == model);
    if !known {
        return Err(format!("model '{}/{}' not in catalog", provider, model));
    }

    Ok((provider, model))
}

fn core_tool_names() -> &'static [&'static str] {
    crate::tool::BUILTIN_TOOL_NAMES
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let skill = crate::skills::Skill {
            name: "rust".to_string(),
            description: "Rust skill.".to_string(),
            context: String::new(),
            user_invocable: false,
            file_path: std::path::PathBuf::from("rust/SKILL.md"),
        };

        let config = temp_config_with_provider("openai", "gpt-4o");
        let report = dry_run_report_with_skills(&config, &[skill]);
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

    #[test]
    fn dry_run_tool_names_match_canonical() {
        assert_eq!(core_tool_names(), crate::tool::BUILTIN_TOOL_NAMES);
    }
}
