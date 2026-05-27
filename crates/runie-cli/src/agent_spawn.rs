use std::sync::Arc;
use runie_agent::AgentTool;

pub fn create_agent_tools(registry: Arc<runie_tools::ToolRegistry>) -> Vec<AgentTool> {
    registry.list().into_iter().map(|tool| {
        let name = tool.name().to_string();
        let description = tool.description().to_string();
        let parameters = tool.schema().parameters;
        AgentTool::new(name, description, parameters)
    }).collect()
}
