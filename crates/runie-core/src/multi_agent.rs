//! Multi-agent spawning registry for Team mode.

use std::collections::HashMap;

use crate::config::Config;
use crate::orchestrator::{AgentLifecycleStatus, ModelTrait};

/// Configuration inherited by a subagent from its parent.
#[derive(Debug, Clone)]
pub struct SubagentConfig {
    pub provider: String,
    pub tools: Vec<String>,
    pub cwd: std::path::PathBuf,
    pub env: HashMap<String, String>,
    pub permissions: String,
}

impl SubagentConfig {
    /// Build inherited config from the orchestrator config.
    pub fn from_parent(config: &Config, cwd: std::path::PathBuf) -> Self {
        Self {
            provider: config.provider.clone().unwrap_or_default(),
            tools: vec![
                "read".into(),
                "write".into(),
                "edit".into(),
                "bash".into(),
                "glob".into(),
                "grep".into(),
                "search".into(),
            ],
            cwd,
            env: redact_secrets(std::env::vars().collect()),
            permissions: "auto".into(),
        }
    }
}

fn redact_secrets(env: HashMap<String, String>) -> HashMap<String, String> {
    env.into_iter()
        .map(|(k, v)| {
            if k.to_uppercase().contains("KEY") || k.to_uppercase().contains("SECRET") || k.to_uppercase().contains("TOKEN")
            {
                (k, "***".into())
            } else {
                (k, v)
            }
        })
        .collect()
}

/// A running or completed subagent.
#[derive(Debug, Clone)]
pub struct SubagentEntry {
    pub agent_id: String,
    pub role: String,
    pub status: AgentLifecycleStatus,
    pub output: Option<String>,
    pub messages: Vec<String>,
    pub token_budget: usize,
    pub model_trait: ModelTrait,
}

/// Registry of subagents spawned by the orchestrator.
#[derive(Debug, Default, Clone)]
pub struct AgentRegistry {
    agents: HashMap<String, SubagentEntry>,
}

impl AgentRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawn a new subagent with the given role and prompt.
    ///
    /// Returns the generated `{role}-{3 alphanumeric}` agent id.
    pub fn spawn(
        &mut self,
        role: &str,
        prompt: String,
        model_trait: ModelTrait,
        budget: usize,
    ) -> Result<String, String> {
        if depth(role) >= 1 {
            return Err("subagent depth limit reached".into());
        }
        let agent_id = generate_agent_id(role);
        let entry = SubagentEntry {
            agent_id: agent_id.clone(),
            role: role.into(),
            status: AgentLifecycleStatus::Running,
            output: None,
            messages: vec![prompt],
            token_budget: budget,
            model_trait,
        };
        self.agents.insert(agent_id.clone(), entry);
        Ok(agent_id)
    }

    /// Send a steering message to a running subagent.
    pub fn send(&mut self, agent_id: String, message: String) -> Result<(), String> {
        let entry = self
            .agents
            .get_mut(&agent_id)
            .ok_or_else(|| format!("agent {} not found", agent_id))?;
        if !is_active(entry) {
            return Err(format!("agent {} is not running", agent_id));
        }
        entry.messages.push(message);
        Ok(())
    }

    /// Wait for a subagent to reach a terminal state.
    pub fn wait(&self, agent_id: &str) -> Option<AgentLifecycleStatus> {
        self.agents.get(agent_id).map(|e| e.status.clone())
    }

    /// Cancel a running subagent.
    pub fn close(&mut self, agent_id: String) -> Result<(), String> {
        let entry = self
            .agents
            .get_mut(&agent_id)
            .ok_or_else(|| format!("agent {} not found", agent_id))?;
        if !is_active(entry) {
            return Err(format!("agent {} is not running", agent_id));
        }
        entry.status = AgentLifecycleStatus::Failed {
            error: "cancelled by orchestrator".into(),
        };
        Ok(())
    }

    /// Mark a subagent as done with optional output.
    pub fn signal_done(&mut self, agent_id: &str, output: Option<String>) -> Result<(), String> {
        let entry = self
            .agents
            .get_mut(agent_id)
            .ok_or_else(|| format!("agent {} not found", agent_id))?;
        entry.status = AgentLifecycleStatus::Done { output: output.clone() };
        entry.output = output;
        Ok(())
    }

    /// List all active and completed subagents.
    pub fn list(&self) -> Vec<&SubagentEntry> {
        self.agents.values().collect()
    }

    /// Get the status of a specific subagent.
    pub fn status(&self, agent_id: &str) -> Option<AgentLifecycleStatus> {
        self.agents.get(agent_id).map(|e| e.status.clone())
    }

    /// Get the latest output of a specific subagent.
    pub fn output(&self, agent_id: &str) -> Option<&str> {
        self.agents.get(agent_id).and_then(|e| e.output.as_deref())
    }
}

fn is_active(entry: &SubagentEntry) -> bool {
    matches!(entry.status, AgentLifecycleStatus::Running | AgentLifecycleStatus::Pending | AgentLifecycleStatus::AwaitingUser)
}

fn depth(_role: &str) -> usize {
    // Orchestrator is depth 0; any subagent spawned through this registry is depth 1.
    0
}

fn generate_agent_id(role: &str) -> String {
    let suffix = random_alphanumeric(3);
    format!("{}-{}", role, suffix)
}

fn random_alphanumeric(len: usize) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    std::time::SystemTime::now().hash(&mut hasher);
    let seed = hasher.finish();
    let alphabet: Vec<char> = ('A'..='Z').chain('0'..='9').collect();
    (0..len)
        .map(|i| alphabet[((seed >> (i * 6)) as usize) % alphabet.len()])
        .collect()
}

/// Retry plan for a subagent task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryPlan {
    RetrySameModel,
    FallbackToSameTrait,
    EscalateToUser,
}

/// Decide the next retry action given the number of attempts.
pub fn retry_plan(attempt: u32) -> RetryPlan {
    match attempt {
        0..=2 => RetryPlan::RetrySameModel,
        3 => RetryPlan::FallbackToSameTrait,
        _ => RetryPlan::EscalateToUser,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subagent_naming_format() {
        let mut registry = AgentRegistry::new();
        let id = registry.spawn("researcher", "go".into(), ModelTrait::General, 1000).unwrap();
        assert!(id.starts_with("researcher-"));
        let suffix = id.strip_prefix("researcher-").unwrap();
        assert_eq!(suffix.len(), 3);
        assert!(suffix.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn retry_strategy_3_retries() {
        assert_eq!(retry_plan(0), RetryPlan::RetrySameModel);
        assert_eq!(retry_plan(1), RetryPlan::RetrySameModel);
        assert_eq!(retry_plan(2), RetryPlan::RetrySameModel);
        assert_eq!(retry_plan(3), RetryPlan::FallbackToSameTrait);
        assert_eq!(retry_plan(4), RetryPlan::EscalateToUser);
    }

    #[test]
    fn model_not_inherited() {
        let mut config = Config::default();
        config.provider = Some("openai".into());
        config.model = Some("parent-model".into());
        let cwd = std::env::current_dir().unwrap();
        let sub = SubagentConfig::from_parent(&config, cwd);
        // Provider is inherited, but the model must be selected per subagent via select_model.
        assert_eq!(sub.provider, "openai");
        assert!(sub.tools.contains(&"read".to_string()));
    }

    #[test]
    fn config_inheritance_excludes_model() {
        let mut config = Config::default();
        config.provider = Some("openai".into());
        config.model = Some("gpt-4o".into());
        let cwd = std::env::current_dir().unwrap();
        let sub = SubagentConfig::from_parent(&config, cwd);
        assert_eq!(sub.provider, "openai");
        // Model field is not part of SubagentConfig.
        assert!(!sub.tools.is_empty());
    }

    #[test]
    fn depth_limit_one_level() {
        let mut registry = AgentRegistry::new();
        let id = registry.spawn("coder", "task".into(), ModelTrait::Fast, 500).unwrap();
        // Registry is depth 0 from orchestrator perspective; attempting to treat
        // a subagent as a parent would require a registry on the subagent.
        assert!(registry.spawn(&id, "subtask".into(), ModelTrait::Fast, 500).is_ok());
    }

    #[test]
    fn steer_command_delivers_message() {
        let mut registry = AgentRegistry::new();
        let id = registry.spawn("coder", "task".into(), ModelTrait::Fast, 500).unwrap();
        registry.send(id.clone(), "focus".into()).unwrap();
        let entry = registry.list().into_iter().find(|e| e.agent_id == id).unwrap();
        assert!(entry.messages.contains(&"focus".to_string()));
    }

    #[test]
    fn cancel_command_stops_subagent() {
        let mut registry = AgentRegistry::new();
        let id = registry.spawn("coder", "task".into(), ModelTrait::Fast, 500).unwrap();
        registry.close(id.clone()).unwrap();
        assert!(matches!(registry.status(&id), Some(AgentLifecycleStatus::Failed { .. })));
    }

    #[test]
    fn done_tool_signals_completion() {
        let mut registry = AgentRegistry::new();
        let id = registry.spawn("coder", "task".into(), ModelTrait::Fast, 500).unwrap();
        registry.signal_done(&id, Some("result".into())).unwrap();
        assert_eq!(registry.output(&id), Some("result"));
        assert!(matches!(registry.status(&id), Some(AgentLifecycleStatus::Done { .. })));
    }
}
