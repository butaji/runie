impl FeedState {
    fn replace_workflow(
        &mut self,
        run_id: &str,
        status: &str,
        elapsed_ms: Option<u64>,
        active_agents: u32,
    ) {
        let header = self
            .navigation
            .facts.workflow_headers
            .get(run_id)
            .cloned()
            .unwrap_or_else(|| "Workflow".into());
        let phases = self
            .navigation
            .facts.workflow_phases
            .get(run_id)
            .cloned()
            .unwrap_or_default();
        self.replace_tool(
            run_id,
            workflow_text(&header, &phases, status, elapsed_ms, active_agents),
        );
    }
}
