//! Flow Orchestration Module
//!
//! Multi-step dialogs and complex interaction patterns.

mod context;
mod step;
mod flow;
mod executor;
mod validators;
mod transitions;

pub use context::{FlowContext, FlowResult};
pub use step::Step;
pub use flow::Flow;
pub use executor::FlowExecutor;

pub use transitions::{push, pop, close, emit, goto_step};
