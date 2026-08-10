impl FeedState {
    fn reduce_workflow(&mut self, message: ScrollbackMsg) -> Result<(), ScrollbackMsg> {
        match message {
            ScrollbackMsg::WorkflowStart {
                run_id,
                name,
                objective,
            } => {
                self.start_workflow(run_id, name, objective);
                Ok(())
            }
            ScrollbackMsg::WorkflowProgress {
                run_id,
                phase,
                state,
                active_agents,
            } => {
                self.update_workflow(run_id, phase, state, active_agents);
                Ok(())
            }
            ScrollbackMsg::WorkflowEnd {
                run_id,
                status,
                elapsed_ms,
            } => {
                self.finish_workflow(run_id, status, elapsed_ms);
                Ok(())
            }
            message => Err(message),
        }
    }
}
