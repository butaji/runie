impl FeedState {
    fn update_workflow(
        &mut self,
        run_id: String,
        phase: String,
        state: String,
        active_agents: u32,
    ) {
        let phases = self
            .navigation
            .facts.workflow_phases
            .entry(run_id.clone())
            .or_default();
        if let Some(existing) = phases.iter_mut().find(|(title, _)| title == &phase) {
            existing.1 = state;
        } else {
            phases.push((phase, state));
        }
        self.replace_workflow(&run_id, "active", None, active_agents);
    }
}
