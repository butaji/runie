//! Tests for runie-agent

use crate::events::{PermissionDecision, ContentPart, AgentEvent, AgentMessage};
use crate::hook::{Hook, HookDecision, SafetyHook};
use crate::loop_engine::AgentLoopConfig;
use crate::state::AgentState;
use crate::harness::compaction::find_cut_point;
use crate::harness::types::CompactionSettings;
use futures::StreamExt;
use runie_core::{Message, Session, Context, ToolCall, ToolOutput};
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use runie_ai::Provider;
use runie_tools::{create_default_toolkit, Workspace};
use std::path::PathBuf;

mod unit_tests;
mod agent_loop_tests;
mod provider_tests;
mod hook_tests;
mod retry_tests;
mod compaction_tests;
mod tool_id_tests;

pub use unit_tests::*;
pub use agent_loop_tests::*;
pub use provider_tests::*;
pub use hook_tests::*;
pub use retry_tests::*;
pub use compaction_tests::*;
pub use tool_id_tests::*;
