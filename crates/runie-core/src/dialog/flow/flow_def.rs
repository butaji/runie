//! Flow - A sequence of steps with branching

use super::{FlowContext, Step};
use crate::commands::CommandResult;
use crate::dialog::dsl::panel;
use std::collections::HashMap;

/// A flow is a sequence of steps with branching support
#[derive(Debug, Clone, Default)]
pub struct Flow {
    pub id: String,
    pub steps: Vec<Step>,
    pub branches: HashMap<String, Vec<Step>>,
    pub on_error: Option<fn(&mut FlowContext, &str) -> Option<Step>>,
    pub on_complete: Option<fn(&FlowContext) -> CommandResult>,
}

impl Flow {
    /// Create a new flow
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            ..Default::default()
        }
    }

    /// Add a step
    pub fn step<F>(mut self, builder: F) -> Self
    where
        F: FnOnce(&mut FlowContext) -> Step,
    {
        let mut ctx = FlowContext::new();
        let step = builder(&mut ctx);
        self.steps.push(step);
        self
    }

    /// Add a named branch
    pub fn branch<F>(mut self, name: &str, builder: F) -> Self
    where
        F: FnOnce(&mut FlowContext) -> Step,
    {
        let mut ctx = FlowContext::new();
        let step = builder(&mut ctx);
        self.branches.insert(name.into(), vec![step]);
        self
    }

    /// Add multiple steps to a branch
    pub fn branch_steps<F>(mut self, name: &str, builder: F) -> Self
    where
        F: FnOnce(&mut FlowContext) -> Vec<Step>,
    {
        let mut ctx = FlowContext::new();
        let steps = builder(&mut ctx);
        self.branches.insert(name.into(), steps);
        self
    }

    /// Set error handler
    pub fn on_error(mut self, handler: fn(&mut FlowContext, &str) -> Option<Step>) -> Self {
        self.on_error = Some(handler);
        self
    }

    /// Set completion handler
    pub fn on_complete(mut self, handler: fn(&FlowContext) -> CommandResult) -> Self {
        self.on_complete = Some(handler);
        self
    }

    /// Build into a PanelStack starting from step 0
    pub fn start(&self) -> crate::dialog::PanelStack {
        if let Some(first) = self.steps.first() {
            let mut stack = crate::dialog::PanelStack::new(first.panel.clone().into_core());
            for step in self.steps.iter().skip(1) {
                stack.push(step.panel.clone().into_core());
            }
            stack
        } else {
            crate::dialog::PanelStack::new(panel("empty", "Empty Flow").into_core())
        }
    }

    /// Build from a specific step
    pub fn from_step(&self, index: usize) -> Option<crate::dialog::PanelStack> {
        self.steps.get(index).map(|start| {
            let mut stack = crate::dialog::PanelStack::new(start.panel.clone().into_core());
            for step in self.steps.iter().skip(index + 1) {
                stack.push(step.panel.clone().into_core());
            }
            stack
        })
    }

    /// Get step by index
    pub fn get_step(&self, index: usize) -> Option<&Step> {
        self.steps.get(index)
    }

    /// Get step by ID
    pub fn find_step(&self, id: &str) -> Option<&Step> {
        self.steps.iter().find(|s| s.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog::dsl::panel;
    use crate::dialog::dsl::ItemAction;

    #[test]
    fn test_flow_builder() {
        let flow = Flow::new("test")
            .step(|_| Step::show(panel("a", "Step A").action("Next", ItemAction::Push("b".into()))))
            .step(|_| {
                Step::show(
                    panel("b", "Step B")
                        .action("Back", ItemAction::Pop)
                        .action("Done", ItemAction::Close),
                )
            });

        assert_eq!(flow.steps.len(), 2);
        assert_eq!(flow.steps[0].id, "a");
        assert_eq!(flow.steps[1].id, "b");
    }

    #[test]
    fn test_flow_with_branch() {
        let flow = Flow::new("decide")
            .step(|_| {
                Step::show(
                    panel("choice", "Choose")
                        .action("Path A", ItemAction::Push("path_a".into()))
                        .action("Path B", ItemAction::Push("path_b".into())),
                )
            })
            .branch("path_a", |_| {
                Step::show(panel("a", "Path A").action("Done", ItemAction::Close))
            })
            .branch("path_b", |_| {
                Step::show(panel("b", "Path B").action("Done", ItemAction::Close))
            });

        assert_eq!(flow.steps.len(), 1);
        assert_eq!(flow.branches.len(), 2);
    }

    #[test]
    fn test_flow_start() {
        let flow = Flow::new("test")
            .step(|_| Step::show(panel("a", "A")))
            .step(|_| Step::show(panel("b", "B")));

        let stack = flow.start();
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.breadcrumb(), vec!["A", "B"]);
    }
}
