//! Step - A single step in a flow

use crate::dialog::dsl::Panel;

/// A single step in a flow
#[derive(Debug, Clone)]
pub struct Step {
    pub id: String,
    pub panel: Panel,
    #[allow(clippy::type_complexity)]
    pub validator: Option<fn(&mut super::FlowContext, &Panel) -> Result<(), String>>,
    pub on_enter: Option<fn(&mut super::FlowContext)>,
    pub on_exit: Option<fn(&mut super::FlowContext)>,
}

impl Step {
    /// Create a step showing a panel
    pub fn show(panel: Panel) -> Self {
        Self {
            id: panel.id.clone(),
            panel,
            validator: None,
            on_enter: None,
            on_exit: None,
        }
    }

    /// Set step ID (defaults to panel id)
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Add validation
    pub fn validate(
        mut self,
        validator: fn(&mut super::FlowContext, &Panel) -> Result<(), String>,
    ) -> Self {
        self.validator = Some(validator);
        self
    }

    /// Add enter callback
    pub fn on_enter(mut self, f: fn(&mut super::FlowContext)) -> Self {
        self.on_enter = Some(f);
        self
    }

    /// Add exit callback
    pub fn on_exit(mut self, f: fn(&mut super::FlowContext)) -> Self {
        self.on_exit = Some(f);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::Step;
    use crate::dialog::dsl::panel;
    use crate::dialog::flow::context::FlowContext;

    fn make_enter_true(_: &mut FlowContext) {}
    fn make_exit_true(_: &mut FlowContext) {}

    #[test]
    fn test_step_with_callbacks() {
        let step = Step::show(panel("test", "Test"))
            .on_enter(make_enter_true)
            .on_exit(make_exit_true);

        assert!(step.on_enter.is_some());
        assert!(step.on_exit.is_some());
    }
}
