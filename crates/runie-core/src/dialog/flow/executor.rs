//! Flow Executor - Runs a flow with context

use super::{Flow, FlowContext, FlowResult};
use crate::commands::CommandResult;
use crate::dialog::dsl::{ItemAction, Panel};

/// Executes a flow with context
pub struct FlowExecutor {
    pub flow: Flow,
    pub context: FlowContext,
    pub current_panel: usize,
}

impl FlowExecutor {
    pub fn new(flow: Flow) -> Self {
        let current_panel = 0;
        Self {
            flow,
            context: FlowContext::new(),
            current_panel,
        }
    }

    /// Get current panel
    pub fn current_panel(&self) -> Option<&Panel> {
        self.flow.steps.get(self.current_panel).map(|s| &s.panel)
    }

    /// Move to next panel
    pub fn advance(&mut self) -> Option<&Panel> {
        if self.current_panel + 1 < self.flow.steps.len() {
            self.current_panel += 1;
            self.context.step = self.current_panel;
        }
        self.current_panel()
    }

    /// Move to previous panel
    pub fn prev(&mut self) -> Option<&Panel> {
        if self.current_panel > 0 {
            self.current_panel -= 1;
            self.context.step = self.current_panel;
        }
        self.current_panel()
    }

    /// Jump to specific panel
    pub fn jump(&mut self, index: usize) -> Option<&Panel> {
        if index < self.flow.steps.len() {
            self.current_panel = index;
            self.context.step = index;
        }
        self.current_panel()
    }

    /// Validate current panel
    pub fn validate(&mut self) -> Result<(), String> {
        if let Some(step) = self.flow.steps.get(self.current_panel) {
            if let Some(validator) = step.validator {
                validator(&mut self.context, &step.panel)
            } else {
                Ok(())
            }
        } else {
            Err("No current panel".into())
        }
    }

    /// Process an action
    pub fn handle_action(&mut self, action: &ItemAction) -> FlowResult {
        match action {
            ItemAction::Push(id) => {
                if let Some((flow_id, step_str)) = id.split_once(':') {
                    if flow_id == self.flow.id {
                        if let Ok(step_idx) = step_str.parse::<usize>() {
                            return FlowResult::Jump(step_idx);
                        }
                    }
                }
                if let Some(branch_steps) = self.flow.branches.get(id) {
                    if !branch_steps.is_empty() {
                        self.context.data.insert("_branch".into(), id.clone());
                        return FlowResult::Branch(id.clone());
                    }
                }
                FlowResult::Next
            }
            ItemAction::Pop => FlowResult::Prev,
            ItemAction::Close => FlowResult::Done(CommandResult::None),
            ItemAction::Emit(_) => FlowResult::Next,
            _ => FlowResult::Next,
        }
    }

    /// Complete the flow
    pub fn complete(&self) -> CommandResult {
        if let Some(handler) = self.flow.on_complete {
            handler(&self.context)
        } else {
            CommandResult::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Flow, FlowExecutor};
    use crate::dialog::dsl::panel;
    use crate::dialog::flow::Step;

    #[test]
    fn test_flow_executor() {
        let flow = Flow::new("test")
            .step(|_| Step::show(panel("a", "A")))
            .step(|_| Step::show(panel("b", "B")))
            .step(|_| Step::show(panel("c", "C")));

        let mut exec = FlowExecutor::new(flow);
        assert_eq!(exec.current_panel, 0);

        exec.advance();
        assert_eq!(exec.current_panel, 1);

        exec.jump(0);
        assert_eq!(exec.current_panel, 0);

        exec.advance();
        exec.advance();
        assert_eq!(exec.current_panel, 2);

        exec.advance();
        assert_eq!(exec.current_panel, 2);

        exec.prev();
        assert_eq!(exec.current_panel, 1);
    }
}
