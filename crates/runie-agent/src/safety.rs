//! Bash command safety checks.
//!
//! Re-exported from `runie_core::bash_safety` so the agent and engine share
//! the same heuristic safety logic.

pub use runie_core::bash_safety::check_bash_safety;
