//! `/workflow` slash command — declarative Team mode orchestration.

use crate::commands::{CommandCategory, CommandRegistry, CommandResult};
use crate::dsl::workflow::{parse_workflow_args, WorkflowTask};
use crate::event::CommandEvent;
use crate::model::AppState;
use crate::orchestrator::{
    ExecutionMode, ModelTrait, OrchestratorPlan, SubagentTask, SynthesisConfig,
};
use crate::orchestrator_actor::OrchestratorState;

use super::dsl::handlers::spec::{CommandKind, CommandSpec};

static WORKFLOW_COMMANDS: &[CommandSpec] = &[CommandSpec {
    name: "workflow",
    desc: "Define a Team mode workflow",
    aliases: &[],
    category: CommandCategory::System,
    sub: false,
    kind: CommandKind::Handler(handle_workflow),
}];

pub fn register(registry: &mut CommandRegistry) {
    super::dsl::spec::register_commands(registry, WORKFLOW_COMMANDS);
}

/// `/workflow <definition>` — parse the DSL, switch to Team mode, and emit a
/// `PlanGenerated` event so the Orchestrator starts executing.
pub fn handle_workflow(state: &mut AppState, args: &str) -> CommandResult {
    let definition = match parse_workflow_args(args) {
        Ok(d) => d,
        Err(e) => return CommandResult::Warning(format!("Invalid workflow: {e}")),
    };

    state.config.execution_mode = ExecutionMode::Team;
    state.orchestrator_state = OrchestratorState::Executing;

    let plan = build_plan(&definition);
    let summary = plan_summary(&plan);
    state.add_system_msg(summary);

    CommandResult::Event(CommandEvent::PlanGenerated {
        plan: Box::new(plan),
    })
}

fn build_plan(definition: &crate::dsl::workflow::WorkflowDefinition) -> OrchestratorPlan {
    let tasks: Vec<SubagentTask> = definition
        .tasks
        .iter()
        .enumerate()
        .map(|(idx, task)| build_subagent_task(task, idx))
        .collect();

    OrchestratorPlan {
        tasks,
        synthesis_trait: ModelTrait::General,
        summary: Some(format!("Workflow with {} task(s)", definition.tasks.len())),
        rationale: None,
        synthesis: definition.synthesis.clone(),
    }
}

fn build_subagent_task(task: &WorkflowTask, idx: usize) -> SubagentTask {
    let role = normalize_role(&task.alias);
    let id = generate_task_id(&role, idx);
    let role_prompt = format!("You are a {}.", role);
    SubagentTask::new(
        id,
        role_prompt,
        task.description.clone(),
        ModelTrait::General,
    )
}

fn normalize_role(alias: &str) -> String {
    if alias.is_empty() {
        return "agent".into();
    }
    alias.to_lowercase()
}

fn generate_task_id(role: &str, index: usize) -> String {
    let suffix = encode_base36(index, 3);
    format!("{}-{}", role, suffix)
}

fn encode_base36(mut n: usize, width: usize) -> String {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut chars = Vec::with_capacity(width);
    for _ in 0..width {
        chars.push(ALPHABET[n % ALPHABET.len()]);
        n /= ALPHABET.len();
    }
    chars.reverse();
    String::from_utf8(chars).unwrap_or_else(|_| "000".into())
}

fn plan_summary(plan: &OrchestratorPlan) -> String {
    let names: Vec<String> = plan
        .tasks
        .iter()
        .map(|t| format!("{}: {}", t.id, t.task_description))
        .collect();
    let suffix = match &plan.synthesis {
        SynthesisConfig::Llm => "LLM synthesis".into(),
        SynthesisConfig::Prompt(p) => format!("custom prompt: {p}"),
        SynthesisConfig::Template(t) => format!("template: {t}"),
    };
    format!("Started workflow with {} task(s). {}.", names.len(), suffix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::CommandResult;
    use crate::orchestrator::ExecutionMode;

    #[test]
    fn workflow_command_parses() {
        let mut state = AppState::default();
        let result = handle_workflow(&mut state, "\"echo test\" as tester");
        assert!(
            matches!(
                result,
                CommandResult::Event(CommandEvent::PlanGenerated { .. })
            ),
            "expected PlanGenerated event, got {:?}",
            result
        );
        assert_eq!(state.config.execution_mode, ExecutionMode::Team);
    }

    #[test]
    fn parallel_workflow_creates_multiple_agents() {
        let mut state = AppState::default();
        let result = handle_workflow(&mut state, "[\"Task A\" as a, \"Task B\" as b]");
        let plan = match result {
            CommandResult::Event(CommandEvent::PlanGenerated { plan }) => plan,
            other => panic!("expected PlanGenerated event, got {:?}", other),
        };
        assert_eq!(plan.tasks.len(), 2);
        assert!(plan.tasks[0].id.starts_with("a-"));
        assert!(plan.tasks[1].id.starts_with("b-"));
    }

    #[test]
    fn synthesis_options_accepted() {
        let mut state = AppState::default();
        let result = handle_workflow(
            &mut state,
            "\"Research\" as r --synthesize \"Combine findings\"",
        );
        let plan = match result {
            CommandResult::Event(CommandEvent::PlanGenerated { plan }) => plan,
            other => panic!("expected PlanGenerated event, got {:?}", other),
        };
        assert!(
            matches!(plan.synthesis, SynthesisConfig::Prompt(ref s) if s == "Combine findings")
        );

        let result = handle_workflow(&mut state, "[\"A\" as a] --template \"Results:\\n{tasks}\"");
        let plan = match result {
            CommandResult::Event(CommandEvent::PlanGenerated { plan }) => plan,
            other => panic!("expected PlanGenerated event, got {:?}", other),
        };
        assert!(
            matches!(plan.synthesis, SynthesisConfig::Template(ref s) if s == "Results:\\n{tasks}")
        );
    }

    #[test]
    fn workflow_command_starts_orchestrator() {
        let mut state = AppState::default();
        let result = handle_workflow(&mut state, "\"echo test\" as tester");
        assert!(
            matches!(
                result,
                CommandResult::Event(CommandEvent::PlanGenerated { .. })
            ),
            "expected PlanGenerated event, got {:?}",
            result
        );
        assert_eq!(state.config.execution_mode, ExecutionMode::Team);
        assert!(matches!(
            state.orchestrator_state,
            OrchestratorState::Executing
        ));
    }

    #[test]
    fn invalid_workflow_returns_warning() {
        let mut state = AppState::default();
        let result = handle_workflow(&mut state, "");
        assert!(
            matches!(result, CommandResult::Warning(_)),
            "expected Warning, got {:?}",
            result
        );
    }

    #[test]
    fn subagent_id_format() {
        let id = generate_task_id("researcher", 2747);
        assert!(id.starts_with("researcher-"));
        let suffix = id.strip_prefix("researcher-").unwrap();
        assert_eq!(suffix.len(), 3);
        assert!(suffix.chars().all(|c| c.is_ascii_alphanumeric()));
    }
}
