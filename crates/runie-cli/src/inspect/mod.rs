//! `runie inspect` — Print runtime configuration for the current directory.
//!
//! This command loads and displays all declarative configuration discovered
//! for the current directory: skills, commands, subagent types, MCP servers,
//! permission rules, and config sources.

use runie_core::config::Config;
use runie_core::proto::ProviderConfig;
use runie_core::skills::{load_all, Skill};
use runie_core::subagents::{PermissionMode, PromptMode, SubagentRegistry};
use secrecy::ExposeSecret;

use std::collections::HashSet;

/// Capacity of the EventBus channel used for config actor initialization in inspect.
const EVENT_BUS_CHANNEL_CAPACITY: usize = 16;

// Type alias for the config handle reply type
type ConfigHandle = runie_core::actors::RactorConfigHandle;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Report sections for the inspect command.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct InspectReport {
    pub config_sources: Vec<ConfigSource>,
    pub skill_items: Vec<SkillInfo>,
    pub commands: Vec<CommandInfo>,
    pub subagents: Vec<SubagentInfo>,
    pub permissions: Vec<PermissionInfo>,
    pub providers: Vec<ProviderInfo>,
    pub model_catalog: Vec<ModelInfoEntry>,
    /// Validation errors found in config.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validation_errors: Vec<String>,
    /// Hints for setting up providers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub setup_hints: Vec<String>,
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
    /// Build a full inspect report asynchronously using ConfigActor.
    pub async fn build_with_config_actor(config_handle: &ConfigHandle) -> Self {
        let config = config_handle.load_layers().await.unwrap_or_default();
        let skills = tokio::task::spawn_blocking(load_all)
            .await
            .unwrap_or_default();
        let subagent_registry = SubagentRegistry::from_builtins();

        let validation_errors = Self::validate_config(&config);
        let setup_hints = Self::generate_setup_hints(&config);

        Self {
            config_sources: Self::discover_config_sources(),
            skill_items: Self::format_skills(skills),
            commands: Self::list_commands(),
            subagents: Self::list_subagents(&subagent_registry),
            permissions: Self::list_permissions(&config),
            providers: Self::list_providers(&config),
            model_catalog: Self::list_model_catalog(),
            validation_errors,
            setup_hints,
        }
    }

    /// Validate config and return error messages.
    fn validate_config(config: &Config) -> Vec<String> {
        let mut errors = Vec::new();

        // Check if provider is set but not configured
        if let Some(provider) = &config.provider {
            if !provider.is_empty()
                && !config.model_providers.contains_key(provider) {
                    errors.push(format!(
                        "provider '{}' set as default but not configured in model_providers",
                        provider
                    ));
                }
        }

        // Check if any configured providers have their API key
        for name in config.model_providers.keys() {
            let api_key = config
                .resolve_api_key(name)
                .map(|s| s.expose_secret().clone())
                .unwrap_or_default();
            if api_key.is_empty() {
                let env_var = runie_core::provider::find_provider(name)
                    .map(|p| p.env_var.clone())
                    .unwrap_or_else(|| format!("{}_API_KEY", name.to_uppercase()));
                errors.push(format!(
                    "provider '{}': API key not found in keyring or {} environment variable",
                    name, env_var
                ));
            }
        }

        // Check if default model exists for provider
        if let Some(provider) = &config.provider {
            if !provider.is_empty() {
                if let Some(model) = config.default_model() {
                    if !model.is_empty() {
                        let model_str: &str = model;
                        if !runie_core::model_catalog::model_catalog()
                            .iter()
                            .any(|m| m.provider == *provider && m.name == model_str) {
                            errors.push(format!(
                                "model '{}' not found in model catalog for provider '{}'",
                                model, provider
                            ));
                        }
                    }
                }
            }
        }

        errors
    }

    /// Generate actionable hints for setting up providers.
    fn generate_setup_hints(config: &Config) -> Vec<String> {
        let mut hints = Vec::new();

        // Check if no providers are configured
        if config.model_providers.is_empty() {
            hints.push("No providers configured. Run `runie login` to set up a provider.".to_string());
            hints.push("Or set a provider in ~/.runie/config.toml:".to_string());
            hints.push("  provider = \"openai\"".to_string());
            hints.push("  [model_providers.openai]".to_string());
            hints.push("  base_url = \"https://api.openai.com/v1\"".to_string());
            hints.push("  models = [\"gpt-4o\"]".to_string());
            hints.push("".to_string());
            hints.push("Then store your API key with: runie login --provider openai".to_string());
        }

        // Check for missing API keys
        for name in config.model_providers.keys() {
            let api_key = config
                .resolve_api_key(name)
                .map(|s| s.expose_secret().clone())
                .unwrap_or_default();
            if api_key.is_empty() {
                let env_var = runie_core::provider::find_provider(name)
                    .map(|p| p.env_var.clone())
                    .unwrap_or_else(|| format!("{}_API_KEY", name.to_uppercase()));
                hints.push(format!(
                    "Missing API key for '{}'. Set {} or run `runie login --provider {}`",
                    name, env_var, name
                ));
            }
        }

        // Check for environment variable hints
        let providers = runie_core::provider::known_providers();
        for provider in &providers {
            if std::env::var(&provider.env_var).is_ok() {
                hints.push(format!(
                    "{} detected in environment. Add '{}' to model_providers in config to use it.",
                    provider.env_var, provider.key
                ));
            }
        }

        hints
    }

    fn discover_config_sources() -> Vec<ConfigSource> {
        let mut sources = Vec::new();
        let mut seen = HashSet::new();

        // Global config
        if let Some(global) = Self::global_config_path() {
            if !seen.contains(&global) {
                seen.insert(global.clone());
                let loaded = std::path::Path::new(&global).exists();
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
                let loaded = std::path::Path::new(&local).exists();
                sources.push(ConfigSource {
                    path: local,
                    loaded,
                });
            }
        }

        sources
    }

    fn global_config_path() -> Option<String> {
        dirs::home_dir().map(|p| {
            p.join(".runie")
                .join("config.toml")
                .to_string_lossy()
                .to_string()
        })
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
                path: s.file_path.to_string(),
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
        self.print_config_sources();
        self.print_skills();
        self.print_commands();
        self.print_subagents();
        self.print_permissions();
        self.print_providers();
        self.print_model_catalog();
        self.print_diagnostics();
    }

    /// Print validation errors and setup hints.
    fn print_diagnostics(&self) {
        if !self.validation_errors.is_empty() {
            println!("## Configuration Errors");
            for error in &self.validation_errors {
                println!("  ✗ {}", error);
            }
            println!();
        }

        if !self.setup_hints.is_empty() {
            println!("## Setup Hints");
            for hint in &self.setup_hints {
                if hint.is_empty() {
                    println!();
                } else {
                    println!("  → {}", hint);
                }
            }
            println!();
        }
    }

    fn print_config_sources(&self) {
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
    }

    fn print_skills(&self) {
        println!("## Skills ({} loaded)", self.skill_items.len());
        if self.skill_items.is_empty() {
            println!("  (none)");
        } else {
            for skill in &self.skill_items {
                let invocable = if skill.invocable { " [invocable]" } else { "" };
                println!("  • {} — {}{}", skill.name, skill.description, invocable);
                println!("    from: {}", skill.path);
            }
        }
        println!();
    }

    fn print_commands(&self) {
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
                println!(
                    "  /{} [{}] — {}{}",
                    cmd.name, cmd.category, cmd.description, aliases
                );
            }
        }
        println!();
    }

    fn print_subagents(&self) {
        println!("## Subagent Types ({} defined)", self.subagents.len());
        if self.subagents.is_empty() {
            println!("  (none)");
        } else {
            for agent in &self.subagents {
                let agents_md = if agent.agents_md { " (AGENTS.md)" } else { "" };
                println!("  • {} — {}{}", agent.name, agent.description, agents_md);
                println!(
                    "    mode: {}, perms: {}",
                    agent.prompt_mode, agent.permission_mode
                );
            }
        }
        println!();
    }

    fn print_permissions(&self) {
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
    }

    fn print_providers(&self) {
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
    }

    fn print_model_catalog(&self) {
        println!("## Model Catalog ({} models)", self.model_catalog.len());
        if self.model_catalog.is_empty() {
            println!("  (empty)");
        } else {
            let mut by_provider: std::collections::BTreeMap<&str, Vec<&ModelInfoEntry>> =
                std::collections::BTreeMap::new();
            for model in &self.model_catalog {
                by_provider.entry(&model.provider).or_default().push(model);
            }
            for (provider, models) in by_provider {
                println!("  {}:", provider);
                for model in models {
                    Self::print_model_entry(model);
                }
            }
        }
    }

    fn print_model_entry(model: &ModelInfoEntry) {
        let context = model.context_window.map(|c| format!("{}k", c / 1000));
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
        let suffix = format!(
            "{}{}",
            flags_str,
            context.map(|c| format!(" ({})", c)).unwrap_or_default()
        );
        println!("    • {}{}", model.name, suffix);
    }

    /// Print JSON report.
    pub fn print_json(&self) {
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Report<'a> {
            config_sources: &'a [ConfigSource],
            skill_items: &'a [SkillInfo],
            commands: &'a [CommandInfo],
            subagents: &'a [SubagentInfo],
            permissions: &'a [PermissionInfo],
            providers: &'a [ProviderInfo],
            model_catalog: &'a [ModelInfoEntry],
            validation_errors: &'a [String],
            setup_hints: &'a [String],
        }

        let report = Report {
            config_sources: &self.config_sources,
            skill_items: &self.skill_items,
            commands: &self.commands,
            subagents: &self.subagents,
            permissions: &self.permissions,
            providers: &self.providers,
            model_catalog: &self.model_catalog,
            validation_errors: &self.validation_errors,
            setup_hints: &self.setup_hints,
        };

        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        );
    }
}

// ---------------------------------------------------------------------------
// CLI Entry Point
// ---------------------------------------------------------------------------

/// Run the inspect command.
pub async fn run(json: bool) -> anyhow::Result<()> {
    let (config_handle, _cell, _join) = runie_core::actors::RactorConfigActor::spawn_default(
        runie_core::bus::EventBus::new(EVENT_BUS_CHANNEL_CAPACITY),
    )
    .await
    .unwrap();

    let report = InspectReport::build_with_config_actor(&config_handle).await;
    if json {
        report.print_json();
    } else {
        report.print_human();
    }
    Ok(())
}
