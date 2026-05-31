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
mod mode_transitions;
mod agent_events;

// Palette test modules
mod palette_open_tests;
mod palette_filter_tests;
mod palette_navigation_tests;
mod palette_execution_tests;
mod palette_close_tests;
mod palette_usage_tests;

// Input handling test modules
mod input_submission;
mod input_history;
mod input_paste;
mod input_unicode;

// Scroll and navigation test modules
mod scroll_tests;
mod scroll_auto_tests;
mod scroll_edge_tests;
mod session_management_tests;

// Re-export all test modules for convenience
