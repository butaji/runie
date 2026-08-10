impl FeedState {
    fn reduce_tool(&mut self, message: ScrollbackMsg) -> Result<(), ScrollbackMsg> {
        match message {
            ScrollbackMsg::ToolStart {
                tool_call_id,
                header,
                activity,
            } => {
                self.start_tool(tool_call_id, header, activity, false);
                Ok(())
            }
            ScrollbackMsg::ToolStartRunning {
                tool_call_id,
                header,
                activity,
            } => {
                self.start_tool(tool_call_id, header, activity, true);
                Ok(())
            }
            ScrollbackMsg::ToolUpdate {
                tool_call_id,
                header,
                output,
            } => {
                self.update_tool_output(tool_call_id, header, output);
                Ok(())
            }
            ScrollbackMsg::ToolEnd {
                tool_call_id,
                header,
                activity,
                output,
            } => {
                self.finish_tool(tool_call_id, header, activity, output);
                Ok(())
            }
            message => Err(message),
        }
    }
}
