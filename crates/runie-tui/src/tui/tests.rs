#![allow(clippy::unwrap_used)]
#![cfg(test)]

mod comprehensive_suite;
mod data_structures;
mod reducer;
mod dirty_flag;
mod palette_integration_tests;
mod render_tests;
mod state_management;
mod input_handling;
mod e2e_flow_tests;
mod command_integration_tests;

// Re-export all test modules for convenience
