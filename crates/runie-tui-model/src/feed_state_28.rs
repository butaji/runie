impl FeedState {
    fn start_workflow(&mut self, run_id: String, name: String, objective: String) {
        let header = format!("Workflow {name}: {objective}");
        self.navigation
            .facts.workflow_headers
            .insert(run_id.clone(), header.clone());
        self.navigation
            .facts.workflow_phases
            .insert(run_id.clone(), Vec::new());
        self.append(Line::new(LineKind::ToolRunning, header).for_tool(run_id));
    }
}
