#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub max_turns: usize,
    pub tool_execution_mode: ToolExecutionMode,
    pub enable_compaction: bool,
    pub compaction_threshold: usize,
    pub temperature: f32,
    pub max_tokens: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_turns: 50,
            tool_execution_mode: ToolExecutionMode::Sequential,
            enable_compaction: true,
            compaction_threshold: 6000,
            temperature: 0.7,
            max_tokens: 8192,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ToolExecutionMode {
    Parallel,
    Sequential,
}
