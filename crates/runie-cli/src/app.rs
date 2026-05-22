use runie_agent::{Agent, CodingAgent, AgentConfig, AgentLoop, AgentState};
use runie_ai::providers::OpenAiProvider;
use runie_tools::{Workspace, create_default_toolkit};
use runie_router::Router;
use runie_orchestrator::Orchestrator;
use runie_core::Session;
use std::sync::Arc;
use std::path::PathBuf;

use crate::session_manager::SessionManager;
use crate::settings::{Settings, sessions_dir};

pub struct App {
    pub agent: Option<Box<dyn Agent>>,
    pub session_manager: SessionManager,
    pub router: Option<Box<dyn Router>>,
    pub orchestrator: Option<Box<dyn Orchestrator>>,
    pub workspace: Workspace,
    pub settings: Settings,
}

impl App {
    pub fn new(workspace_path: PathBuf, settings: Settings) -> Self {
        let workspace = Workspace::new(workspace_path);
        let sessions_path = sessions_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".tidy/sessions"));
        let session_manager = SessionManager::new(sessions_path);

        Self {
            agent: None,
            session_manager,
            router: None,
            orchestrator: None,
            workspace,
            settings,
        }
    }

    pub async fn initialize(&mut self) -> Result<(), AppError> {
        // Create default tool registry
        let registry = Arc::new(create_default_toolkit(self.workspace.clone()));

        // Create provider using settings
        let api_key = self.settings.api_key.clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok());
        let provider = Arc::new(OpenAiProvider::new(
            api_key.unwrap_or_default(),
            self.settings.model.clone(),
        ));

        // Create agent
        let session = Session::new(uuid::Uuid::new_v4().to_string());
        let state = AgentState::new(session);
        let mut config = AgentConfig::default();
        config.max_turns = self.settings.max_turns;
        config.temperature = self.settings.temperature;
        config.compaction_threshold = self.settings.compact_threshold;
        config.tool_execution_mode = match self.settings.tool_mode.as_str() {
            "sequential" => runie_agent::config::ToolExecutionMode::Sequential,
            _ => runie_agent::config::ToolExecutionMode::Parallel,
        };
        let loop_inner = AgentLoop::new(provider, registry, vec![], state, config);
        self.agent = Some(Box::new(CodingAgent::new(loop_inner)));

        Ok(())
    }

    pub async fn run_interactive(&mut self) -> Result<(), AppError> {
        println!("Tidy Coding Harness");
        println!("Type your request or 'quit' to exit.");
        
        loop {
            // In real impl, use crossterm for input
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let input = input.trim();
            
            if input == "quit" || input == "exit" {
                break;
            }
            
            if let Some(agent) = &mut self.agent {
                let events = agent.run(input.to_string()).await
                    .map_err(|e| AppError::AgentError(e.to_string()))?;
                
                for event in events {
                    match event {
                        runie_core::Event::MessageDelta { content } => print!("{}", content),
                        runie_core::Event::ToolExecutionStart { tool_name, .. } => {
                            println!("\n[Executing: {}]", tool_name);
                        }
                        runie_core::Event::Error { message } => {
                            eprintln!("\n[Error: {}]", message);
                        }
                        _ => {}
                    }
                }
                println!();
            }
        }
        
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("app error: {0}")]
    Failed(String),
    #[error("agent error: {0}")]
    AgentError(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
