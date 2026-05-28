#![allow(clippy::unwrap_used)]
#![cfg(test)]

mod data_structures;
mod reducer;
mod dirty_flag;
mod palette_integration_tests;
mod render_tests;
mod state_management;
mod input_handling;

// Re-export all test modules for convenience
