//! TUI-side IPC endpoint for the submission/event queue pair.
//!
//! The handle is defined in `runie_core::ipc` so both sides can be constructed
//! together without a dependency cycle; this module re-exports it for the TUI.
pub use runie_core::ipc::TuiIpc;
