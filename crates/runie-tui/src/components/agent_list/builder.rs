use crate::components::status_bar::BackgroundJob;
use crate::components::message_list::PlanStatus;
use crate::tui::view_models::AgentListViewModel;

pub struct AgentListBuilder {
    plan_steps: Vec<(usize, String, PlanStatus)>,
    running_jobs: Vec<BackgroundJob>,
    active_count: usize,
    tokens: u64,
    cost: f64,
    agent_running: bool,
    braille_frame: usize,
}

impl AgentListBuilder {
    pub fn new() -> Self {
        Self {
            plan_steps: Vec::new(),
            running_jobs: Vec::new(),
            active_count: 0,
            tokens: 0,
            cost: 0.0,
            agent_running: false,
            braille_frame: 0,
        }
    }

    pub fn plan_step(mut self, step: usize, text: &str, status: PlanStatus) -> Self {
        self.plan_steps.push((step, text.to_string(), status));
        self
    }

    pub fn running_job(mut self, name: &str) -> Self {
        self.running_jobs.push(BackgroundJob {
            name: name.to_string(),
            status: crate::components::status_bar::JobStatus::Running,
        });
        self.active_count = self.running_jobs.len();
        self
    }

    pub fn active_count(mut self, count: usize) -> Self {
        self.active_count = count;
        self
    }

    pub fn tokens(mut self, tokens: u64) -> Self {
        self.tokens = tokens;
        self
    }

    pub fn cost(mut self, cost: f64) -> Self {
        self.cost = cost;
        self
    }

    pub fn agent_running(mut self, running: bool) -> Self {
        self.agent_running = running;
        self
    }

    pub fn braille_frame(mut self, frame: usize) -> Self {
        self.braille_frame = frame;
        self
    }

    pub fn build(self) -> AgentListViewModel {
        AgentListViewModel {
            plan_steps: self.plan_steps,
            running_jobs: self.running_jobs,
            active_count: self.active_count,
            tokens: self.tokens,
            cost: self.cost,
            agent_running: self.agent_running,
            braille_frame: self.braille_frame,
        }
    }
}

impl Default for AgentListBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_empty_agent_list() {
        let vm = AgentListBuilder::new().build();
        assert!(vm.plan_steps.is_empty());
        assert!(vm.running_jobs.is_empty());
        assert_eq!(vm.active_count, 0);
        assert_eq!(vm.tokens, 0);
        assert_eq!(vm.cost, 0.0);
        assert!(!vm.agent_running);
    }

    #[test]
    fn test_build_with_plan_steps() {
        let vm = AgentListBuilder::new()
            .plan_step(1, "Step 1", PlanStatus::Pending)
            .plan_step(2, "Step 2", PlanStatus::Active)
            .build();
        assert_eq!(vm.plan_steps.len(), 2);
        assert_eq!(vm.plan_steps[0].0, 1);
        assert_eq!(vm.plan_steps[1].0, 2);
    }

    #[test]
    fn test_build_with_running_job() {
        let vm = AgentListBuilder::new()
            .running_job("Test Job")
            .build();
        assert_eq!(vm.running_jobs.len(), 1);
        assert_eq!(vm.running_jobs[0].name, "Test Job");
        assert_eq!(vm.active_count, 1);
    }

    #[test]
    fn test_build_with_tokens_and_cost() {
        let vm = AgentListBuilder::new()
            .tokens(1000)
            .cost(0.05)
            .build();
        assert_eq!(vm.tokens, 1000);
        assert_eq!(vm.cost, 0.05);
    }
}
