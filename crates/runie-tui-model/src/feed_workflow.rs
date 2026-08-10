// Workflow lifecycle state and its transcript projection.

impl FeedState {
    fn start_workflow(&mut self, run_id: String, name: String, objective: String) {
        let header = format!("Workflow {name}: {objective}");
        self.navigation.facts.workflow_headers.insert(run_id.clone(), header.clone());
        self.navigation.facts.workflow_phases.insert(run_id.clone(), Vec::new());
        self.append(Line::new(LineKind::ToolRunning, header).for_tool(run_id));
    }

    fn update_workflow(&mut self, run_id: String, phase: String, state: String, active_agents: u32) {
        let phases = self.navigation.facts.workflow_phases.entry(run_id.clone()).or_default();
        if let Some(existing) = phases.iter_mut().find(|(title, _)| title == &phase) { existing.1 = state; }
        else { phases.push((phase, state)); }
        self.replace_workflow(&run_id, "active", None, active_agents);
    }

    fn finish_workflow(&mut self, run_id: String, status: String, elapsed_ms: Option<u64>) {
        self.replace_workflow(&run_id, &status, elapsed_ms, 0);
    }

    fn replace_workflow(&mut self, run_id: &str, status: &str, elapsed_ms: Option<u64>, active_agents: u32) {
        let header = self.navigation.facts.workflow_headers.get(run_id).cloned().unwrap_or_else(|| "Workflow".into());
        let phases = self.navigation.facts.workflow_phases.get(run_id).cloned().unwrap_or_default();
        self.replace_tool(run_id, workflow_text(&header, &phases, status, elapsed_ms, active_agents));
    }
}
