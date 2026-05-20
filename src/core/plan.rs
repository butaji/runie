//! Plan generator - creates static DAGs from Intent
//! Generates step-by-step execution plans for agent tasks

use super::intent::{Intent, TaskType};
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

/// Plan action types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Read and understand context files
    ReadContext { files: Vec<String> },
    /// Generate or modify code
    GenerateCode { files: Vec<String> },
    /// Run tests
    RunTests { pattern: Option<String> },
    /// Run linter/formatter
    Lint { pattern: Option<String> },
    /// Build/compile
    Build { target: Option<String> },
    /// Commit changes
    Commit { message: Option<String> },
    /// Ask human for clarification
    AskHuman { question: String },
    /// Review and approve
    Review,
}

/// A single step in the execution plan
#[derive(Debug, Clone)]
pub struct PlanStep {
    pub id: String,
    pub action: Action,
    /// IDs of steps that must complete before this one
    pub depends_on: Vec<String>,
    /// Whether this step can run in parallel with others
    pub parallel_group: Option<String>,
    /// Estimated cost for this step
    pub estimated_cost: f64,
}

impl PlanStep {
    pub fn new(id: &str, action: Action) -> Self {
        let estimated_cost = match &action {
            Action::ReadContext { .. } => 0.01,
            Action::GenerateCode { .. } => 0.05,
            Action::RunTests { .. } => 0.02,
            Action::Lint { .. } => 0.01,
            Action::Build { .. } => 0.03,
            Action::Commit { .. } => 0.001,
            Action::AskHuman { .. } => 0.0,
            Action::Review => 0.01,
        };

        Self {
            id: id.to_string(),
            action,
            depends_on: Vec::new(),
            parallel_group: None,
            estimated_cost,
        }
    }

    pub fn after(mut self, step_id: &str) -> Self {
        self.depends_on.push(step_id.to_string());
        self
    }

    pub fn parallel_with(mut self, group: &str) -> Self {
        self.parallel_group = Some(group.to_string());
        self
    }
}

/// A complete execution plan (DAG)
#[derive(Debug, Clone)]
pub struct Plan {
    pub steps: Vec<PlanStep>,
    pub total_estimated_cost: f64,
}

impl Plan {
    /// Generate a plan from intent
    pub fn from_intent(intent: &Intent) -> Self {
        let mut steps = Vec::new();
        let mut total_cost = 0.0;

        match intent.task_type {
            TaskType::Refactor => {
                // Read → Generate → Test → Lint → Commit
                let read = PlanStep::new("read", Action::ReadContext {
                    files: intent.target_files.clone(),
                });
                total_cost += read.estimated_cost;
                steps.push(read);

                let generate = PlanStep::new("generate", Action::GenerateCode {
                    files: intent.target_files.clone(),
                }).after("read");
                total_cost += generate.estimated_cost;
                steps.push(generate);

                let test = PlanStep::new("test", Action::RunTests { pattern: None })
                    .after("generate");
                total_cost += test.estimated_cost;
                steps.push(test);

                let lint = PlanStep::new("lint", Action::Lint { pattern: None })
                    .after("generate");
                total_cost += lint.estimated_cost;
                steps.push(lint);

                let commit = PlanStep::new("commit", Action::Commit { message: None })
                    .after("test");
                total_cost += commit.estimated_cost;
                steps.push(commit);
            }
            TaskType::Architecture => {
                // Read → Analyze → Design → Review → Commit
                let read = PlanStep::new("read", Action::ReadContext {
                    files: intent.target_files.clone(),
                });
                total_cost += read.estimated_cost;
                steps.push(read);

                let analyze = PlanStep::new("analyze", Action::GenerateCode {
                    files: vec!["ARCHITECTURE.md".to_string()],
                }).after("read");
                total_cost += analyze.estimated_cost;
                steps.push(analyze);

                let review = PlanStep::new("review", Action::Review)
                    .after("analyze");
                total_cost += review.estimated_cost;
                steps.push(review);

                let commit = PlanStep::new("commit", Action::Commit { message: None })
                    .after("review");
                total_cost += commit.estimated_cost;
                steps.push(commit);
            }
            TaskType::TestGeneration => {
                // Read → Generate Tests → Run → Commit
                let read = PlanStep::new("read", Action::ReadContext {
                    files: intent.target_files.clone(),
                });
                total_cost += read.estimated_cost;
                steps.push(read);

                let generate = PlanStep::new("generate", Action::GenerateCode {
                    files: intent.target_files.iter()
                        .map(|f| format!("{}.test{}", f, if f.ends_with(".rs") { ".rs" } else { "" }))
                        .collect(),
                }).after("read");
                total_cost += generate.estimated_cost;
                steps.push(generate);

                let test = PlanStep::new("test", Action::RunTests { pattern: None })
                    .after("generate");
                total_cost += test.estimated_cost;
                steps.push(test);

                let commit = PlanStep::new("commit", Action::Commit { message: None })
                    .after("test");
                total_cost += commit.estimated_cost;
                steps.push(commit);
            }
            TaskType::Analysis | TaskType::Unknown => {
                // Read → Analyze → Report
                let read = PlanStep::new("read", Action::ReadContext {
                    files: intent.target_files.clone(),
                });
                total_cost += read.estimated_cost;
                steps.push(read);

                let analyze = PlanStep::new("analyze", Action::GenerateCode {
                    files: vec!["ANALYSIS.md".to_string()],
                }).after("read");
                total_cost += analyze.estimated_cost;
                steps.push(analyze);
            }
            TaskType::EmergencyFix => {
                // Read → Fix → Test (minimal steps)
                let read = PlanStep::new("read", Action::ReadContext {
                    files: intent.target_files.clone(),
                });
                total_cost += read.estimated_cost;
                steps.push(read);

                let generate = PlanStep::new("fix", Action::GenerateCode {
                    files: intent.target_files.clone(),
                }).after("read");
                total_cost += generate.estimated_cost;
                steps.push(generate);

                let test = PlanStep::new("test", Action::RunTests { pattern: None })
                    .after("fix");
                total_cost += test.estimated_cost;
                steps.push(test);
            }
            TaskType::General => {
                // Read → Generate → Test → Build → Commit
                let read = PlanStep::new("read", Action::ReadContext {
                    files: intent.target_files.clone(),
                });
                total_cost += read.estimated_cost;
                steps.push(read);

                let generate = PlanStep::new("generate", Action::GenerateCode {
                    files: intent.target_files.clone(),
                }).after("read");
                total_cost += generate.estimated_cost;
                steps.push(generate);

                let test = PlanStep::new("test", Action::RunTests { pattern: None })
                    .after("generate");
                total_cost += test.estimated_cost;
                steps.push(test);

                let build = PlanStep::new("build", Action::Build { target: None })
                    .after("generate");
                total_cost += build.estimated_cost;
                steps.push(build);

                let commit = PlanStep::new("commit", Action::Commit { message: None })
                    .after("test")
                    .after("build");
                total_cost += commit.estimated_cost;
                steps.push(commit);
            }
        }

        Self { steps, total_estimated_cost: total_cost }
    }

    /// Convert to petgraph DAG for visualization/execution
    pub fn to_dag(&self) -> DiGraph<PlanStep, ()> {
        let mut graph: DiGraph<PlanStep, ()> = DiGraph::new();
        let mut node_map: HashMap<String, NodeIndex> = HashMap::new();

        // Add nodes
        for step in &self.steps {
            let idx = graph.add_node(step.clone());
            node_map.insert(step.id.clone(), idx);
        }

        // Add edges
        for step in &self.steps {
            let from_idx = node_map[&step.id];
            for dep in &step.depends_on {
                if let Some(&to_idx) = node_map.get(dep) {
                    graph.add_edge(to_idx, from_idx, ());
                }
            }
        }

        graph
    }

    /// Get steps that have no dependencies (can start immediately)
    pub fn entry_steps(&self) -> Vec<&PlanStep> {
        self.steps.iter()
            .filter(|s| s.depends_on.is_empty())
            .collect()
    }

    /// Get steps ready to execute (all dependencies completed)
    pub fn ready_steps(&self, completed: &[String]) -> Vec<&PlanStep> {
        let completed_set: std::collections::HashSet<&str> =
            completed.iter().map(|s| s.as_str()).collect();

        self.steps.iter()
            .filter(|s| !completed_set.contains(s.id.as_str()))
            .filter(|s| s.depends_on.iter().all(|d| completed_set.contains(d.as_str())))
            .collect()
    }

    /// Get parallel groups (steps that can run concurrently)
    pub fn parallel_groups(&self, completed: &[String]) -> Vec<Vec<&PlanStep>> {
        let ready = self.ready_steps(completed);
        let mut groups: HashMap<Option<String>, Vec<&PlanStep>> = HashMap::new();

        for step in ready {
            groups
                .entry(step.parallel_group.clone())
                .or_default()
                .push(step);
        }

        groups.into_values().filter(|g| !g.is_empty()).collect()
    }
}

/// Render plan as text for display
impl std::fmt::Display for Plan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Plan (estimated cost: ${:.2}):", self.total_estimated_cost)?;
        writeln!(f)?;
        for step in &self.steps {
            let deps = if step.depends_on.is_empty() {
                "(entry)".to_string()
            } else {
                format!("after: {}", step.depends_on.join(", "))
            };
            writeln!(f, "  {}: {:?} [{}]", step.id, step.action, deps)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refactor_plan() {
        let intent = Intent::from_text("refactor src/auth.rs");
        let plan = Plan::from_intent(&intent);
        assert!(!plan.steps.is_empty());
        assert!(plan.steps.iter().any(|s| s.id == "read"));
        assert!(plan.steps.iter().any(|s| s.id == "generate"));
    }

    #[test]
    fn test_architecture_plan() {
        let intent = Intent::from_text("design new architecture");
        let plan = Plan::from_intent(&intent);
        assert!(plan.steps.iter().any(|s| matches!(s.action, Action::Review)));
    }

    #[test]
    fn test_entry_steps() {
        let intent = Intent::from_text("fix the bug");
        let plan = Plan::from_intent(&intent);
        let entries = plan.entry_steps();
        assert!(!entries.is_empty());
        assert!(entries.iter().all(|s| s.depends_on.is_empty()));
    }

    #[test]
    fn test_dag_creation() {
        let intent = Intent::from_text("refactor src/main.rs");
        let plan = Plan::from_intent(&intent);
        let dag = plan.to_dag();
        assert_eq!(dag.node_count(), plan.steps.len());
    }

    #[test]
    fn test_ready_steps() {
        let intent = Intent::from_text("refactor src/main.rs");
        let plan = Plan::from_intent(&intent);
        let completed: Vec<String> = vec!["read".to_string()];
        let ready = plan.ready_steps(&completed);
        // Should include generate (depends only on read)
        assert!(ready.iter().any(|s| s.id == "generate"));
    }
}
