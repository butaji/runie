//! ToolActor — Executes tools asynchronously and emits ToolEnd events
//!
//! Each tool invocation spawns a dedicated ToolActor. The actor:
//! - Receives tool definition via channel
//! - Executes the tool (blocking) in a dedicated thread
//! - Emits ToolEnd event on completion

use crate::event_bus::{ActorChannel, BusEventEnvelope, DomainEvent, EventBus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub use crate::actors::tools::ToolInvocation;

/// Tool execution output
#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub success: bool,
    pub output: String,
}

/// Run a ToolActor until shutdown or channel close.
///
/// The actor executes the tool in a blocking thread and emits:
/// - AgentToolStart on begin
/// - AgentToolEnd on completion
pub fn run_tool_actor(
    bus: EventBus,
    _channel: ActorChannel<BusEventEnvelope>,
    invocation: ToolInvocation,
    shutdown: Arc<AtomicBool>,
) {
    let tool_name = invocation.name.clone();
    let tool_id = invocation.id.clone();

    // Emit ToolStart
    bus.publish_domain(DomainEvent::AgentToolStart {
        id: tool_id.clone(),
        name: tool_name.clone(),
    });

    let tool_start = Instant::now();

    // Execute tool in a blocking thread (tools can be slow)
    let result = std::thread::spawn(move || {
        super::tools::execute_tool(&invocation)
    }).join().unwrap_or(ToolOutput {
        success: false,
        output: "Tool execution panicked".to_string(),
    });

    let duration_secs = tool_start.elapsed().as_secs_f64();

    // Emit ToolEnd
    bus.publish_domain(DomainEvent::AgentToolEnd {
        id: tool_id,
        name: tool_name,
        duration_secs,
        output: result.output,
    });

    // Signal shutdown complete
    shutdown.store(true, Ordering::SeqCst);
}
