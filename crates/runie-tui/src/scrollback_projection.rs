use std::collections::{HashMap, HashSet};

use tokio::sync::{mpsc, oneshot};

use runie_core::types::AgentEvent;

use crate::scrollback_actor::Command;
use crate::widgets::{Line, LineKind, ScrollbackMsg};

pub(crate) async fn run_bus_projection(
    mut events: tokio::sync::broadcast::Receiver<AgentEvent>,
    tx: mpsc::Sender<Command>,
) {
    loop {
        let event = match events.recv().await {
            Ok(event) => event,
            Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                let (reply, _ack) = oneshot::channel();
                let message = ScrollbackMsg::Append(Line::new(
                    LineKind::System,
                    format!("event stream lagged ({count} events)"),
                ));
                if tx
                    .send(Command::ApplyBatch(vec![message], reply))
                    .await
                    .is_err()
                {
                    return;
                }
                continue;
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
        };
        let (reply, _ack) = oneshot::channel();
        if tx
            .send(Command::ApplyEvent(Box::new(event), reply))
            .await
            .is_err()
        {
            return;
        }
    }
}

#[derive(Default)]
pub(crate) struct OwnedEventProjection {
    workspace: String,
    active_tools: HashSet<String>,
    tool_headers: HashMap<String, String>,
    tool_records: HashMap<String, runie_tui_model::ToolRecord>,
    active_tool_count: usize,
    activity_failures: usize,
    activity_dirs: usize,
    activity_files: usize,
    activity_commands: usize,
    activity_subagents: usize,
}

impl OwnedEventProjection {
    pub(crate) fn new(workspace: String) -> Self {
        Self {
            workspace,
            ..Self::default()
        }
    }

    pub(crate) fn messages(&mut self, event: AgentEvent) -> Vec<ScrollbackMsg> {
        self.record_start(&event);
        let completion = ordinary_tool_end_messages(self, &event);
        if !completion.is_empty() {
            return completion;
        }
        let messages = runie_tui_model::bus_messages_for_event(&event);
        if messages.is_empty() {
            tool_update_messages(&self.active_tools, &mut self.tool_headers, &event)
        } else {
            messages
        }
    }

    fn record_start(&mut self, event: &AgentEvent) {
        let AgentEvent::ToolExecutionStart {
            tool_call_id,
            tool_name,
            args,
        } = event
        else {
            return;
        };
        self.active_tools.insert(tool_call_id.clone());
        self.tool_headers.insert(
            tool_call_id.clone(),
            runie_tui_model::tool_header(tool_name, args, &self.workspace),
        );
        let mut record = runie_tui_model::ToolRecord::named(tool_name.clone());
        record.set_args(args.clone());
        self.tool_records.insert(tool_call_id.clone(), record);
        match runie_tui_model::classify_activity_tool(tool_name) {
            Some(runie_tui_model::ActivityKind::Dir) => self.activity_dirs += 1,
            Some(runie_tui_model::ActivityKind::File) => self.activity_files += 1,
            Some(runie_tui_model::ActivityKind::Command) => self.activity_commands += 1,
            Some(runie_tui_model::ActivityKind::Subagent) => self.activity_subagents += 1,
            None => {}
        }
        self.active_tool_count += 1;
    }
}

fn tool_update_messages(
    active: &HashSet<String>,
    headers: &mut HashMap<String, String>,
    event: &AgentEvent,
) -> Vec<ScrollbackMsg> {
    let structured = runie_tui_model::structured_update_messages(active, event);
    if !structured.is_empty() {
        return structured;
    }
    let AgentEvent::ToolExecutionUpdate {
        tool_call_id,
        partial_result,
        ..
    } = event
    else {
        return Vec::new();
    };
    if !active.contains(tool_call_id) || runie_tui_model::is_transport_only_update(partial_result) {
        return Vec::new();
    }
    let Some(header) = headers.get_mut(tool_call_id) else {
        return Vec::new();
    };
    *header = runie_tui_model::tool_update_header_text(header, partial_result);
    vec![ScrollbackMsg::ToolUpdate {
        tool_call_id: tool_call_id.clone(),
        header: Some(header.clone()),
        output: Vec::new(),
    }]
}

fn ordinary_tool_end_messages(
    state: &mut OwnedEventProjection,
    event: &AgentEvent,
) -> Vec<ScrollbackMsg> {
    let AgentEvent::ToolExecutionEnd {
        tool_call_id,
        tool_name,
        result,
        is_error,
        ..
    } = event
    else {
        return Vec::new();
    };
    if !state.active_tools.remove(tool_call_id) {
        return Vec::new();
    }
    let (header, activity, output) =
        tool_end_details(state, tool_call_id, tool_name, result, *is_error);
    let mut messages = vec![ScrollbackMsg::ToolEnd {
        tool_call_id: tool_call_id.clone(),
        header,
        activity,
        output,
    }];
    if *is_error {
        messages.push(ScrollbackMsg::MarkToolError(tool_call_id.clone()));
    }
    messages
}

fn tool_end_details(
    state: &mut OwnedEventProjection,
    tool_call_id: &str,
    tool_name: &str,
    result: &serde_json::Value,
    is_error: bool,
) -> (String, Option<String>, Vec<(LineKind, String)>) {
    state.active_tool_count = state.active_tool_count.saturating_sub(1);
    let pending = state.tool_headers.remove(tool_call_id).unwrap_or_default();
    let record = state.tool_records.remove(tool_call_id).unwrap_or_default();
    let name = record.name.unwrap_or_else(|| tool_name.to_owned());
    let args = record.args.unwrap_or_default();
    let header = if is_error {
        state.activity_failures += 1;
        pending
    } else {
        runie_tui_model::completed_tool_header_with_args(&pending, &name, &args, result)
    };
    let activity = (state.active_tool_count == 0
        && state.activity_dirs
            + state.activity_files
            + state.activity_commands
            + state.activity_subagents
            > 0)
    .then(|| {
        runie_tui_model::activity_text(
            state.activity_dirs,
            state.activity_files,
            state.activity_commands,
            state.activity_subagents,
            state.activity_failures,
            false,
        )
    });
    let output = tool_result_lines(&name, result, is_error);
    (header, activity, output)
}

fn tool_result_lines(
    name: &str,
    result: &serde_json::Value,
    is_error: bool,
) -> Vec<(LineKind, String)> {
    let output_kind = if runie_tui_model::is_output_tool(name) {
        LineKind::ToolOutput
    } else {
        LineKind::ToolResult
    };
    let result_text = runie_tui_model::tool_result_text(result);
    let mut output = result_text
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| (output_kind, line.to_owned()))
        .collect::<Vec<_>>();
    if !is_error && matches!(name, "web_search" | "web-search") {
        if let Some(sources) = runie_tui_model::web_search_sources_line(&result_text) {
            output.push((LineKind::ToolResult, sources));
        }
    }
    output
}
