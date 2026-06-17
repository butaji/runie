use super::Tool;
use crate::context7::Context7Client;
use runie_core::tool::{ToolOutput, ToolStatus};
use std::time::Instant;

pub(crate) fn run_fetch_docs(tool: &Tool, start: Instant) -> ToolOutput {
    let name = tool.name();
    let args = tool.to_args();
    let library = if let Tool::FetchDocs { library } = tool {
        library
    } else {
        unreachable!()
    };
    let client = Context7Client::new();
    match client.fetch(library) {
        Ok(content) => ToolOutput {
            tool_name: name.to_string(),
            tool_args: args,
            content,
            bytes_transferred: None,
            duration: start.elapsed(),
            status: ToolStatus::Success,
        },
        Err(e) => ToolOutput {
            tool_name: name.to_string(),
            tool_args: args,
            content: format!("Error fetching docs for '{}': {}", library, e),
            bytes_transferred: None,
            duration: start.elapsed(),
            status: ToolStatus::Error,
        },
    }
}
