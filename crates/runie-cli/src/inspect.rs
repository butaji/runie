//! `runie inspect` — Print runtime configuration for the current directory.
//!
//! This command loads and displays all declarative configuration discovered
//! for the current directory: skills, commands, subagent types, MCP servers,
//! permission rules, and config sources.

use runie_core::config::Config;
use runie_core::subagents::{PromptMode, PermissionMode, SubagentRegistry};
use runie_core::skills::{load_all, Skill};

use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Report sections for the inspect command.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct InspectReport {
    pub config_sources: Vec<ConfigSource>,
    pub skills: Vec<SkillInfo>,
    pub commands: Vec<CommandInfo>,
    pub subagents: Vec<SubagentInfo>,
    pub permissions: Vec<PermissionInfo>,
    pub providers: Vec<ProviderInfo>,
    pub model_catalog: Vec<ModelInfoEntry>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ConfigSource {
    pub path: String,
    pub loaded: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub invocable: bool,
    pub path: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CommandInfo {
    pub name: String,
    pub category: String,
    pub description: String,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SubagentInfo {
    pub name: String,
    pub description: String,
    pub prompt_mode: String,
    pub permission_mode: String,
    pub agents_md: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PermissionInfo {
    pub action: String,
    pub tool: String,
    pub pattern: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProviderInfo {
    pub name: String,
    pub base_url: String,
    pub model_count: usize,
    // API key is always redacted
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelInfoEntry {
    pub provider: String,
    pub name: String,
    pub context_window: Option<usize>,
    pub supports_thinking: bool,
    pub supports_vision: bool,
}

// ---------------------------------------------------------------------------
// Report Builder
// ---------------------------------------------------------------------------

impl InspectReport {
    /// Build a full inspect report.
    pub fn build() -> Self {
        let config = Config::load_layers();
        let skills = load_all();
        let subagent_registry = SubagentRegistry::from_builtins();

        Self {
            config_sources: Self::discover_config_sources(),
            skills: Self::format_skills(skills),
            commands: Self::list_commands(),
            subagents: Self::list_subagents(&subagent_registry),
            permissions: Self::list_permissions(&config),
            providers: Self::list_providers(&config),
            model_catalog: Self::list_model_catalog(),
        }
    }

    fn discover_config_sources() -> Vec<ConfigSource> {
        let mut sources = Vec::new();
        let mut seen = HashSet::new();

        // Global config
        if let Some(global) = Self::global_config_path() {
            if !seen.contains(&global) {
                seen.insert(global.clone());
                let path = std::path::Path::new(&global);
                let loaded = path.exists();
                sources.push(ConfigSource {
                    path: global,
                    loaded,
                });
            }
        }

        // Local config
        if let Some(local) = Self::local_config_path() {
            if !seen.contains(&local) {
                seen.insert(local.clone());
                let path = std::path::Path::new(&local);
                let loaded = path.exists();
                sources.push(ConfigSource {
                    path: local,
                    loaded,
                });
            }
        }

        sources
    }

    fn global_config_path() -> Option<String> {
        dirs::home_dir()
            .map(|p| p.join(".runie").join("config.toml").to_string_lossy().to_string())
    }

    fn local_config_path() -> Option<String> {
        std::path::Path::new(".runie")
            .join("config.toml")
            .to_str()
            .map(String::from)
    }

    fn format_skills(skills: Vec<Skill>) -> Vec<SkillInfo> {
        skills
            .into_iter()
            .map(|s| SkillInfo {
                name: s.name,
                description: s.description,
                invocable: s.user_invocable,
                path: s.file_path.to_string_lossy().to_string(),
            })
            .collect()
    }

    fn list_commands() -> Vec<CommandInfo> {
        let registry = runie_core::commands::CommandRegistry::new();
        registry
            .list()
            .into_iter()
            .map(|def| CommandInfo {
                name: def.name.clone(),
                category: format!("{:?}", def.category),
                description: def.desc.clone(),
                aliases: def.aliases.clone(),
            })
            .collect()
    }

    fn list_subagents(registry: &SubagentRegistry) -> Vec<SubagentInfo> {
        registry
            .iter()
            .map(|t| SubagentInfo {
                name: t.name.clone(),
                description: t.description.clone(),
                prompt_mode: match t.prompt_mode {
                    PromptMode::Full => "full",
                    PromptMode::Compact => "compact",
                }
                .to_string(),
                permission_mode: match t.permission_mode {
                    PermissionMode::Default => "default",
                    PermissionMode::AcceptEdits => "acceptEdits",
                    PermissionMode::Auto => "auto",
                    PermissionMode::DontAsk => "dontAsk",
                    PermissionMode::BypassPermissions => "bypass",
                    PermissionMode::Plan => "plan",
                }
                .to_string(),
                agents_md: t.agents_md,
            })
            .collect()
    }

    fn list_permissions(config: &Config) -> Vec<PermissionInfo> {
        config
            .permissions
            .rules
            .iter()
            .map(|rule| PermissionInfo {
                action: format!("{:?}", rule.action),
                tool: rule.tool.clone(),
                pattern: rule.pattern.clone(),
            })
            .collect()
    }

    fn list_providers(config: &Config) -> Vec<ProviderInfo> {
        config
            .model_providers
            .iter()
            .map(|(name, provider)| ProviderInfo {
                name: name.clone(),
                base_url: provider.base_url.clone(),
                model_count: provider.models.len(),
                // API key intentionally omitted — always redacted
            })
            .collect()
    }

    fn list_model_catalog() -> Vec<ModelInfoEntry> {
        runie_core::model_catalog::model_catalog()
            .into_iter()
            .map(|m| ModelInfoEntry {
                provider: m.provider.clone(),
                name: m.name.clone(),
                context_window: m.context_window,
                supports_thinking: m.supports_thinking,
                supports_vision: m.supports_vision,
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Output Formatters
// ---------------------------------------------------------------------------

impl InspectReport {
    /// Print human-readable report.
    pub fn print_human(&self) {
        println!("# Runie Inspect Report\n");

        // Config Sources
        println!("## Config Sources");
        if self.config_sources.is_empty() {
            println!("  (none found)");
        } else {
            for source in &self.config_sources {
                let status = if source.loaded { "✓" } else { " " };
                println!("  {} {}", status, source.path);
            }
        }
        println!();

        // Skills
        println!("## Skills ({} loaded)", self.skills.len());
        if self.skills.is_empty() {
            println!("  (none)");
        } else {
            for skill in &self.skills {
                let invocable = if skill.invocable { " [invocable]" } else { "" };
                println!("  • {} — {}{}", skill.name, skill.description, invocable);
                println!("    from: {}", skill.path);
            }
        }
        println!();

        // Commands
        println!("## Commands ({} registered)", self.commands.len());
        if self.commands.is_empty() {
            println!("  (none)");
        } else {
            for cmd in &self.commands {
                let aliases = if cmd.aliases.is_empty() {
                    String::new()
                } else {
                    format!(" (aliases: {})", cmd.aliases.join(", "))
                };
                println!("  /{} [{}] — {}{}", cmd.name, cmd.category, cmd.description, aliases);
            }
        }
        println!();

        // Subagents
        println!("## Subagent Types ({} defined)", self.subagents.len());
        if self.subagents.is_empty() {
            println!("  (none)");
        } else {
            for agent in &self.subagents {
                let agents_md = if agent.agents_md { " (AGENTS.md)" } else { "" };
                println!(
                    "  • {} — {}{}",
                    agent.name, agent.description, agents_md
                );
                println!(
                    "    mode: {}, perms: {}",
                    agent.prompt_mode, agent.permission_mode
                );
            }
        }
        println!();

        // Permissions
        println!("## Permission Rules ({} defined)", self.permissions.len());
        if self.permissions.is_empty() {
            println!("  (none — using defaults)");
        } else {
            for perm in &self.permissions {
                let pattern = perm.pattern.as_deref().unwrap_or("*");
                println!("  {} {} (pattern: {})", perm.action, perm.tool, pattern);
            }
        }
        println!();

        // Providers
        println!("## Providers ({} configured)", self.providers.len());
        if self.providers.is_empty() {
            println!("  (none — run `runie login` to configure)");
        } else {
            for provider in &self.providers {
                println!(
                    "  • {} — {} ({} models)",
                    provider.name, provider.base_url, provider.model_count
                );
                println!("    [API key redacted]");
            }
        }
        println!();

        // Model Catalog
        println!("## Model Catalog ({} models)", self.model_catalog.len());
        if self.model_catalog.is_empty() {
            println!("  (empty)");
        } else {
            // Group by provider
            let mut by_provider: std::collections::BTreeMap<&str, Vec<&ModelInfoEntry>> =
                std::collections::BTreeMap::new();
            for model in &self.model_catalog {
                by_provider.entry(&model.provider).or_default().push(model);
            }
            for (provider, models) in by_provider {
                println!("  {}:", provider);
                for model in models {
                    let context = model
                        .context_window
                        .map(|c| format!("{}k", c / 1000))
                        .unwrap_or_default();
                    let flags: Vec<&str> = [
                        model.supports_thinking.then_some("thinking"),
                        model.supports_vision.then_some("vision"),
                    ]
                    .into_iter()
                    .flatten()
                    .collect();
                    let flags_str = if flags.is_empty() {
                        String::new()
                    } else {
                        format!(" [{}]", flags.join(", "))
                    };
                    if context.is_empty() {
                        println!("    • {}{}", model.name, flags_str);
                    } else {
                        println!("    • {} ({}k){}", model.name, context, flags_str);
                    }
                }
            }
        }
    }

    /// Print JSON report.
    pub fn print_json(&self) {
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Report<'a> {
            config_sources: &'a [ConfigSource],
            skills: &'a [SkillInfo],
            commands: &'a [CommandInfo],
            subagents: &'a [SubagentInfo],
            permissions: &'a [PermissionInfo],
            providers: &'a [ProviderInfo],
            model_catalog: &'a [ModelInfoEntry],
        }

        let report = Report {
            config_sources: &self.config_sources,
            skills: &self.skills,
            commands: &self.commands,
            subagents: &self.subagents,
            permissions: &self.permissions,
            providers: &self.providers,
            model_catalog: &self.model_catalog,
        };

        println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
    }
}

// ---------------------------------------------------------------------------
// CLI Entry Point
// ---------------------------------------------------------------------------

/// Run the inspect command.
pub fn run(json: bool) -> anyhow::Result<()> {
    let report = InspectReport::build();
    if json {
        report.print_json();
    } else {
        report.print_human();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspect_report_builds_without_panic() {
        let report = InspectReport::build();
        // Commands should be registered
        assert!(!report.commands.is_empty(), "Expected commands to be registered");
        // Model catalog should have entries
        assert!(!report.model_catalog.is_empty(), "Expected model catalog entries");
    }

    #[test]
    fn inspect_report_json_serializes() {
        let report = InspectReport::build();
        let json = serde_json::to_string(&report);
        assert!(json.is_ok(), "JSON serialization should succeed");
    }

    #[test]
    fn inspect_report_human_does_not_panic() {
        let report = InspectReport::build();
        // Should not panic
        report.print_human();
    }

    #[test]
    fn skill_info_contains_path() {
        let report = InspectReport::build();
        for skill in &report.skills {
            assert!(!skill.path.is_empty(), "Skill path should not be empty");
        }
    }

    #[test]
    fn provider_info_has_no_api_key() {
        let report = InspectReport::build();
        for provider in &report.providers {
            // ProviderInfo struct should not have api_key field
            // This is verified by struct definition (no api_key field)
            assert!(provider.name != "" || provider.base_url != "");
        }
    }
}
