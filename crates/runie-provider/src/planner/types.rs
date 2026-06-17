use runie_core::orchestrator::OrchestratorContext;
use runie_core::trait_resolver::ModelTrait;
use serde::{Deserialize, Serialize};

/// A tool available to the Orchestrator for planning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDescription {
    pub name: String,
    pub description: String,
}

/// Project context passed to the planner.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectContext {
    /// Short workspace description (e.g. "Rust CLI tool with TUI").
    pub description: String,
    /// Top-level directory names.
    pub directories: Vec<String>,
    /// Key file names (e.g. Cargo.toml, package.json).
    pub key_files: Vec<String>,
}

impl ProjectContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_directories(mut self, dirs: Vec<String>) -> Self {
        self.directories = dirs;
        self
    }

    pub fn with_key_files(mut self, files: Vec<String>) -> Self {
        self.key_files = files;
        self
    }
}

/// Everything the planner needs to generate a plan.
#[derive(Debug, Clone)]
pub struct PlanInput<'a> {
    /// The user's request to break into subagent tasks.
    pub user_request: &'a str,
    /// Project context to include in the prompt.
    pub project: &'a ProjectContext,
    /// Orchestrator working memory (Ask-User Q&A).
    pub orchestrator_context: &'a OrchestratorContext,
    /// Available tools with descriptions.
    pub tools: &'a [ToolDescription],
    /// Available model traits (for the prompt).
    pub available_traits: &'a [ModelTrait],
}

impl<'a> PlanInput<'a> {
    pub fn new(
        user_request: &'a str,
        project: &'a ProjectContext,
        orchestrator_context: &'a OrchestratorContext,
    ) -> Self {
        Self {
            user_request,
            project,
            orchestrator_context,
            tools: &[],
            available_traits: &[ModelTrait::General],
        }
    }

    pub fn with_tools(mut self, tools: &'a [ToolDescription]) -> Self {
        self.tools = tools;
        self
    }

    pub fn with_traits(mut self, traits: &'a [ModelTrait]) -> Self {
        self.available_traits = traits;
        self
    }
}
