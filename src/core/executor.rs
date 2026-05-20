//! Execution engine — wires intent → plan → DAG → stream
//! Connects the parser/generator/executor to the TUI via tokio channels

use crate::core::dag::{DagExecutor, ExecContext, StepState};
use crate::core::git::GitOps;
use crate::core::intent::Intent;
use crate::core::plan::Plan;
use crate::script::runtime::JsRuntime;
use crate::tui::{StreamEntry, EntryType};
use crate::router::ModelDatabase;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Events emitted by the executor back to the TUI
#[derive(Debug, Clone)]
pub enum ExecEvent {
    /// A plan was generated — user should review
    PlanGenerated(Plan),
    /// A step started running
    StepStarted { step_id: String, action: String },
    /// A step completed
    StepCompleted { step_id: String, output: String, cost: f64 },
    /// A step failed
    StepFailed { step_id: String, error: String },
    /// All steps done
    PlanComplete { total_cost: f64 },
    /// JS runtime status
    JsReady { agent_count: usize },
}

/// Input command sent from TUI to executor
#[derive(Debug, Clone)]
pub enum ExecutorInput {
    Execute(String),
    Shutdown,
}

/// Run the executor in headless (non-TUI) mode.
/// Parses the intent, generates a plan, executes the DAG, and prints results.
pub fn run_headless(intent_text: &str) -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime for headless executor");

    rt.block_on(async {
        let (exec_tx, _exec_rx) = mpsc::channel(64);
        let mut executor = Executor::new(exec_tx)
            .map_err(|e| anyhow::anyhow!("Failed to create executor: {}", e))?;

        executor.init().await;
        executor.execute_input(intent_text).await?;

        // Drain any pending events and print them
        // (Events were sent via exec_tx but we don't collect them here
        // since headless just needs the side-effects and final result)
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}

/// Start the executor in a background thread. Returns (exec_rx, input_tx).
pub fn start_executor_thread() -> (mpsc::Receiver<ExecEvent>, mpsc::Sender<ExecutorInput>) {
    let (exec_tx, exec_rx) = mpsc::channel(64);
    let (input_tx, mut input_rx) = mpsc::channel(32);

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime for executor");

        rt.block_on(async {
            let mut executor = match Executor::new(exec_tx) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("[anvil] Failed to create executor: {}", e);
                    return;
                }
            };

            // Init: discover agents + fetch models.dev
            executor.init().await;

            // Main loop: listen for input from TUI
            while let Some(cmd) = input_rx.recv().await {
                match cmd {
                    ExecutorInput::Execute(text) => {
                        if let Err(e) = executor.execute_input(&text).await {
                            eprintln!("[anvil] Execution error: {}", e);
                        }
                    }
                    ExecutorInput::Shutdown => break,
                }
            }
        });
    });

    (exec_rx, input_tx)
}

/// Execution engine — manages the intent→plan→DAG pipeline
pub struct Executor {
    intent_parser: IntentParser,
    plan_generator: PlanGenerator,
    js_runtime: Arc<JsRuntime>,
    model_db: Arc<RwLock<ModelDatabase>>,
    current_plan: Arc<RwLock<Option<Plan>>>,
    event_tx: mpsc::Sender<ExecEvent>,
}

pub struct IntentParser;

impl IntentParser {
    pub fn parse(&self, text: &str) -> Intent {
        Intent::from_text(text)
    }

    pub fn route_to_model(&self, intent: &Intent, db: &ModelDatabase) -> Option<String> {
        intent.route(db).map(|(id, _)| id)
    }
}

pub struct PlanGenerator;

impl PlanGenerator {
    pub fn generate(&self, intent: &Intent) -> Plan {
        Plan::from_intent(intent)
    }

    /// Load agent pack plan override if available
    pub fn generate_with_override(
        &self,
        intent: &Intent,
        js_runtime: &JsRuntime,
        agent_name: &str,
    ) -> Plan {
        let default = self.generate(intent);

        let agent_dir = dirs::home_dir()
            .map(|h| h.join(".anvil/agents").join(agent_name).join("anvil.js"))
            .unwrap_or_else(|| PathBuf::from(".anvil/agents/default/anvil.js"));

        if !agent_dir.exists() {
            return default;
        }

        if !js_runtime.has_function(&agent_dir, "plan") {
            return default;
        }

        let rt = tokio::runtime::Handle::current();
        let result = rt.block_on(js_runtime.run_agent_script(
            &agent_dir,
            "plan",
            vec![serde_json::json!({
                "intent": intent.text,
                "task_type": format!("{:?}", intent.task_type),
                "target_files": intent.target_files,
            })],
        ));

        match result {
            Ok(serde_json::Value::Array(steps)) => {
                self.steps_from_json(&steps)
            }
            Ok(_) => default,
            Err(e) => {
                eprintln!("[anvil] Agent plan override failed, using default: {}", e);
                default
            }
        }
    }

    fn steps_from_json(&self, steps: &[serde_json::Value]) -> Plan {
        use crate::core::plan::{Action, PlanStep};

        let steps: Vec<PlanStep> = steps
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let id = v.get("id").and_then(|s| s.as_str()).unwrap_or("step");
                let action_str = v.get("action").and_then(|s| s.as_str()).unwrap_or("review");
                let action = match action_str {
                    "read_context" => Action::ReadContext {
                        files: v.get("files")
                            .and_then(|f| serde_json::from_value(f.clone()).ok())
                            .unwrap_or_default(),
                    },
                    "generate_code" => Action::GenerateCode {
                        files: v.get("files")
                            .and_then(|f| serde_json::from_value(f.clone()).ok())
                            .unwrap_or_default(),
                    },
                    "run_tests" => Action::RunTests {
                        pattern: v.get("pattern").and_then(|p| p.as_str().map(String::from)),
                    },
                    "lint" => Action::Lint { pattern: None },
                    "build" => Action::Build { target: None },
                    "commit" => Action::Commit { message: None },
                    _ => Action::Review,
                };
                let depends_on: Vec<String> = v
                    .get("after")
                    .and_then(|a| serde_json::from_value(a.clone()).ok())
                    .unwrap_or_default();
                let step = PlanStep::new(&format!("{}_{}", id, i), action);
                step.after(&depends_on.join(""))
            })
            .collect();

        let total_cost: f64 = steps.iter().map(|s| s.estimated_cost).sum();
        Plan { steps, total_estimated_cost: total_cost }
    }
}

impl Executor {
    /// Create a new executor with channels
    pub fn new(event_tx: mpsc::Sender<ExecEvent>) -> anyhow::Result<Self> {
        let js_runtime = JsRuntime::new()
            .map(Arc::new)?;
        let model_db = Arc::new(RwLock::new(ModelDatabase::new()));

        Ok(Self {
            intent_parser: IntentParser,
            plan_generator: PlanGenerator,
            js_runtime,
            model_db,
            current_plan: Arc::new(RwLock::new(None)),
            event_tx,
        })
    }

    /// Initialize: discover agents and fetch models.dev
    pub async fn init(&mut self) {
        // Discover agent packs
        let agent_count = {
            let mut rt = (*self.js_runtime).clone();
            rt.discover_agents().len()
        };
        let _ = self.event_tx.send(ExecEvent::JsReady { agent_count }).await;

        // Fetch models.dev if online
        {
            let mut db = self.model_db.write().await;
            if let Err(e) = db.fetch_from_models_dev().await {
                eprintln!("[anvil] models.dev fetch failed (using defaults): {}", e);
            } else {
                eprintln!("[anvil] models.dev synced successfully");
            }
        }
    }

    /// Handle user input — parse intent, generate plan, execute
    pub async fn execute_input(&self, text: &str) -> anyhow::Result<()> {
        // 1. Parse intent
        let intent = self.intent_parser.parse(text);

        // 2. Generate plan
        let plan = self.plan_generator.generate(&intent);

        // Store current plan
        {
            let mut p = self.current_plan.write().await;
            *p = Some(plan.clone());
        }

        // 3. Emit plan for review
        let _ = self.event_tx.send(ExecEvent::PlanGenerated(plan.clone())).await;

        // 4. Execute the DAG
        let model_db = self.model_db.read().await;
        let current_model = model_db.models.keys().next().cloned().unwrap_or_default();
        drop(model_db);

        let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let git = GitOps::new(working_dir.clone());

        let ctx = ExecContext {
            working_dir,
            model_id: current_model.clone(),
            git: Some(git),
        };

        let executor = DagExecutor::new(plan.clone());
        let results = executor.execute(&ctx).await?;

        // 5. Emit results back
        let mut total_cost = 0.0;
        for result in results {
            total_cost += result.cost;
            match &result.state {
                StepState::Completed => {
                    let _ = self.event_tx.send(ExecEvent::StepCompleted {
                        step_id: result.step_id.clone(),
                        output: result.output.unwrap_or_default(),
                        cost: result.cost,
                    }).await;
                }
                StepState::Failed { error } => {
                    let _ = self.event_tx.send(ExecEvent::StepFailed {
                        step_id: result.step_id.clone(),
                        error: error.clone(),
                    }).await;
                }
                _ => {
                    let _ = self.event_tx.send(ExecEvent::StepStarted {
                        step_id: result.step_id,
                        action: result.output.unwrap_or_default(),
                    }).await;
                }
            }
        }

        let _ = self.event_tx.send(ExecEvent::PlanComplete { total_cost }).await;
        Ok(())
    }

    /// Get current plan for review
    pub async fn get_plan(&self) -> Option<Plan> {
        self.current_plan.read().await.clone()
    }
}

/// Convert an ExecEvent into stream entries for the TUI
impl ExecEvent {
    pub fn to_stream_entry(&self) -> StreamEntry {
        match self {
            ExecEvent::PlanGenerated(plan) => StreamEntry {
                entry_type: EntryType::Plan,
                file: Some("plan.md".to_string()),
                content: plan.steps.iter().map(|s| {
                    format!("  • {}: {:?}", s.id, s.action)
                }).collect(),
                elapsed_secs: None,
                expanded: true,
            },
            ExecEvent::StepStarted { step_id, action } => StreamEntry {
                entry_type: EntryType::Thought,
                file: None,
                content: vec![format!("Running step '{}': {}", step_id, action)],
                elapsed_secs: None,
                expanded: true,
            },
            ExecEvent::StepCompleted { step_id, output, cost } => StreamEntry {
                entry_type: EntryType::Thought,
                file: None,
                content: vec![format!("✓ {} completed (${:.4}) — {}", step_id, cost, output)],
                elapsed_secs: None,
                expanded: true,
            },
            ExecEvent::StepFailed { step_id, error } => StreamEntry {
                entry_type: EntryType::Edit,
                file: Some(format!("step:{}", step_id)),
                content: vec![format!("✗ {} failed: {}", step_id, error)],
                elapsed_secs: None,
                expanded: true,
            },
            ExecEvent::PlanComplete { total_cost } => StreamEntry {
                entry_type: EntryType::Thought,
                file: None,
                content: vec![format!("Plan complete. Total cost: ${:.4}", total_cost)],
                elapsed_secs: None,
                expanded: true,
            },
            ExecEvent::JsReady { agent_count } => StreamEntry {
                entry_type: EntryType::Thought,
                file: None,
                content: vec![format!("JS runtime ready. {} agent packs discovered.", agent_count)],
                elapsed_secs: None,
                expanded: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_executor_creates_ok() {
        let (exec_tx, _exec_rx) = mpsc::channel(16);
        let executor = Executor::new(exec_tx).expect("executor creates ok");
        let plan = executor.get_plan().await;
        assert!(plan.is_none()); // No plan yet — just created
    }

    #[test]
    fn test_exec_event_plan_generated_to_stream() {
        let intent = crate::core::intent::Intent::from_text("refactor auth module");
        let plan = Plan::from_intent(&intent);
        let event = ExecEvent::PlanGenerated(plan);
        let entry = event.to_stream_entry();
        assert!(matches!(entry.entry_type, EntryType::Plan));
        assert!(!entry.content.is_empty());
    }

    #[test]
    fn test_exec_event_completed_to_stream() {
        let event = ExecEvent::StepCompleted {
            step_id: "test".into(),
            output: "ok".into(),
            cost: 0.05,
        };
        let entry = event.to_stream_entry();
        assert!(matches!(entry.entry_type, EntryType::Thought));
    }

    #[test]
    fn test_exec_event_failed_to_stream() {
        let event = ExecEvent::StepFailed {
            step_id: "test".into(),
            error: "boom".into(),
        };
        let entry = event.to_stream_entry();
        assert!(matches!(entry.entry_type, EntryType::Edit));
    }

    #[test]
    fn test_exec_event_js_ready_to_stream() {
        let event = ExecEvent::JsReady { agent_count: 3 };
        let entry = event.to_stream_entry();
        assert!(matches!(entry.entry_type, EntryType::Thought));
        assert!(entry.content[0].contains("3 agent packs"));
    }

    #[test]
    fn test_start_executor_thread_channels_valid() {
        let (exec_rx, input_tx) = start_executor_thread();
        assert!(exec_rx.max_capacity() > 0);
        assert!(input_tx.max_capacity() > 0);
    }
}
