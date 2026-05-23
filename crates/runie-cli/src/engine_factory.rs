use runie_agent::{AgentLoopConfig, Role};
use runie_router::{Router, RoutingContext, ProviderMetadata};
use runie_orchestrator::{Orchestrator, SimpleOrchestrator, DefaultHandoff};
use runie_core::{Compactor, SimpleCompactor};
use async_trait::async_trait;
use std::sync::Arc;
use std::collections::HashMap;
use runie_core::Session;
use runie_core::compactor::CompactorError;

/// Agent loop abstraction
#[async_trait]
pub trait AgentLoop: Send + Sync {
    async fn run(&self, config: AgentLoopConfig, role: Role) -> Result<(), String>;
}

/// Standard agent loop implementation
pub struct StandardAgentLoop;

impl StandardAgentLoop {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AgentLoop for StandardAgentLoop {
    async fn run(&self, _config: AgentLoopConfig, _role: Role) -> Result<(), String> {
        // Implementation would go here - delegates to run_agent_loop
        Ok(())
    }
}

/// Mock agent loop for testing
pub struct MockAgentLoop;

impl MockAgentLoop {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AgentLoop for MockAgentLoop {
    async fn run(&self, _config: AgentLoopConfig, _role: Role) -> Result<(), String> {
        Ok(())
    }
}

/// Default router implementation
pub struct DefaultRouter {
    providers: HashMap<String, ProviderMetadata>,
    default_provider: String,
}

impl DefaultRouter {
    pub fn new(providers: HashMap<String, ProviderMetadata>, default_provider: String) -> Self {
        Self { providers, default_provider }
    }
}

#[async_trait]
impl Router for DefaultRouter {
    async fn select_provider(
        &self,
        _context: &RoutingContext,
        _available: &[String],
    ) -> Result<String, runie_router::RouterError> {
        Ok(self.default_provider.clone())
    }

    async fn should_handoff(
        &self,
        current: &str,
        _context: &RoutingContext,
    ) -> Result<Option<String>, runie_router::RouterError> {
        if current != &self.default_provider {
            Ok(Some(self.default_provider.clone()))
        } else {
            Ok(None)
        }
    }
}

/// Mock router for testing
pub struct MockRouter;

impl MockRouter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Router for MockRouter {
    async fn select_provider(
        &self,
        _context: &RoutingContext,
        available: &[String],
    ) -> Result<String, runie_router::RouterError> {
        available.first().cloned().ok_or(runie_router::RouterError::NoSuitableProvider)
    }

    async fn should_handoff(
        &self,
        _current: &str,
        _context: &RoutingContext,
    ) -> Result<Option<String>, runie_router::RouterError> {
        Ok(None)
    }
}

/// Mock orchestrator for testing
pub struct MockOrchestrator;

impl MockOrchestrator {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Orchestrator for MockOrchestrator {
    async fn spawn(
        &self,
        _task: runie_orchestrator::Task,
        _parent_context: &runie_core::Context,
    ) -> Result<runie_orchestrator::SubagentHandle, runie_orchestrator::OrchestratorError> {
        Err(runie_orchestrator::OrchestratorError::MaxSubagentsExceeded)
    }

    async fn handoff(
        &self,
        _from: &str,
        _to: &str,
        _context: &runie_core::Context,
    ) -> Result<(), runie_orchestrator::OrchestratorError> {
        Ok(())
    }

    async fn collect(
        &self,
        _handles: Vec<runie_orchestrator::SubagentHandle>,
    ) -> Result<Vec<runie_orchestrator::SubagentResult>, runie_orchestrator::OrchestratorError> {
        Ok(Vec::new())
    }

    async fn cancel(&self, _handle_id: &str) -> Result<(), runie_orchestrator::OrchestratorError> {
        Ok(())
    }
}

pub struct EngineFactory;

impl EngineFactory {
    pub fn create_agent_loop(config: &str) -> Box<dyn AgentLoop> {
        match config {
            "mock" => Box::new(MockAgentLoop::new()),
            _ => Box::new(StandardAgentLoop::new()),
        }
    }

    pub fn create_router(config: &str) -> Box<dyn Router> {
        match config {
            "mock" => Box::new(MockRouter::new()),
            _ => {
                let providers = HashMap::<String, ProviderMetadata>::new();
                Box::new(DefaultRouter::new(providers, "default".to_string()))
            }
        }
    }

    pub fn create_orchestrator(config: &str) -> Arc<dyn Orchestrator> {
        match config {
            "mock" => Arc::new(MockOrchestrator::new()),
            _ => Arc::new(SimpleOrchestrator::new(Arc::new(DefaultHandoff), 10)),
        }
    }

    pub fn create_compactor(config: &str) -> Box<dyn Compactor> {
        match config {
            "none" => Box::new(NoOpCompactor),
            _ => Box::new(SimpleCompactor),
        }
    }
}

pub struct NoOpCompactor;

#[async_trait]
impl Compactor for NoOpCompactor {
    async fn compact(&self, session: &Session, _target_tokens: usize) -> Result<Session, CompactorError> {
        Ok(session.clone())
    }
}
