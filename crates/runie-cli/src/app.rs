use runie_agent::{Agent, CodingAgent, AgentConfig, AgentLoop, AgentState};
use runie_ai::providers::OpenAiProvider;
use runie_tools::{Workspace, create_default_toolkit};
use runie_router::Router;
use runie_orchestrator::Orchestrator;
use runie_core::Session;
use std::sync::Arc;
use std::path::PathBuf;

use crate::session_manager::SessionManager;

pub struct App {
    pub agent: Option<Box<dyn Agent>>,
    pub session_manager: SessionManager,
    pub router: Option<Box<dyn Router>>,
    pub orchestrator: Option<Box<dyn Orchestrator>>,
    pub workspace: Workspace,
}

impl App {
    pub fn new(workspace_path: PathBuf) -> Self {
        let workspace = Workspace::new(workspace_path);
        let session_manager = SessionManager::new(
            dirs::home_dir().unwrap_or_default().join(".tidy/sessions")
        );
        
        Self {
            agent: None,
            session_manager,
            router: None,
            orchestrator: None,
            workspace,
        }
    }

    pub async fn initialize(&mut self) -> Result<(), AppError> {
        // Create default tool registry
        let registry = Arc::new(create_default_toolkit(self.workspace.clone()));
        
        // Create default provider
        let provider = Arc::new(OpenAiProvider::new(
            std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            "gpt-4".to_string(),
        ));
        
        // Create agent
        let session = Session::new(uuid::Uuid::new_v4().to_string());
        let state = AgentState::new(session);
        let config = AgentConfig::default();
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
