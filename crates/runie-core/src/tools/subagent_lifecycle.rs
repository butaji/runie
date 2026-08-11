use super::{
    reduce_subagent_turn, SubagentCapability, SubagentResourceUsage, SubagentRole,
    SubagentTurnEvent, SubagentUsageState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubagentLifecycleStatus {
    Idle,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SubagentEvent {
    Started {
        role: SubagentRole,
        capabilities: Vec<SubagentCapability>,
    },
    Turn(SubagentTurnEvent),
    Completed {
        output: serde_json::Value,
    },
    Failed {
        error: String,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SubagentLifecycleState {
    pub role: Option<SubagentRole>,
    pub capabilities: Vec<SubagentCapability>,
    pub status: SubagentLifecycleStatus,
    pub usage: SubagentResourceUsage,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
}

impl Default for SubagentLifecycleState {
    fn default() -> Self {
        Self {
            role: None,
            capabilities: Vec::new(),
            status: SubagentLifecycleStatus::Idle,
            usage: SubagentResourceUsage::default(),
            output: None,
            error: None,
        }
    }
}

pub fn reduce_subagent_event(
    state: &mut SubagentLifecycleState,
    event: SubagentEvent,
) -> Result<(), String> {
    match event {
        SubagentEvent::Started { role, capabilities } => start(state, role, capabilities),
        SubagentEvent::Turn(turn) => turn_step(state, turn),
        SubagentEvent::Completed { output } => complete(state, output),
        SubagentEvent::Failed { error } => fail(state, error),
    }
}

fn start(
    state: &mut SubagentLifecycleState,
    role: SubagentRole,
    capabilities: Vec<SubagentCapability>,
) -> Result<(), String> {
    if !matches!(state.status, SubagentLifecycleStatus::Idle) {
        return Err("subagent can only start from idle".into());
    }
    role.clone().authorize(&capabilities)?;
    state.role = Some(role);
    state.capabilities = capabilities;
    state.status = SubagentLifecycleStatus::Running;
    state.usage = SubagentResourceUsage::default();
    state.output = None;
    state.error = None;
    Ok(())
}

fn turn_step(state: &mut SubagentLifecycleState, turn: SubagentTurnEvent) -> Result<(), String> {
    let role = state
        .role
        .clone()
        .ok_or_else(|| "subagent turn requires start".to_owned())?;
    if !matches!(state.status, SubagentLifecycleStatus::Running) {
        return Err("subagent turn requires running state".into());
    }
    let mut usage = SubagentUsageState { usage: state.usage };
    reduce_subagent_turn(role, &mut usage, turn)?;
    state.usage = usage.usage;
    Ok(())
}

fn complete(state: &mut SubagentLifecycleState, output: serde_json::Value) -> Result<(), String> {
    let role = state
        .role
        .clone()
        .ok_or_else(|| "subagent completion requires start".to_owned())?;
    if !matches!(state.status, SubagentLifecycleStatus::Running) {
        return Err("subagent completion requires running state".into());
    }
    role.validate_output(&output)?;
    state.output = Some(output);
    state.status = SubagentLifecycleStatus::Completed;
    Ok(())
}

fn fail(state: &mut SubagentLifecycleState, error: String) -> Result<(), String> {
    if !matches!(state.status, SubagentLifecycleStatus::Running) {
        return Err("subagent failure requires running state".into());
    }
    state.error = Some(error);
    state.status = SubagentLifecycleStatus::Failed;
    Ok(())
}
