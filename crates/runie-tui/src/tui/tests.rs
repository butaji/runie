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
mod input_history;
mod e2e_flow_tests;
mod command_integration_tests;
mod mode_transitions;
mod agent_events;
mod reply_provider_tests;
mod reply_provider_e2e_tests;
mod reply_provider_system_e2e_tests;
mod reply_provider_visual_e2e_tests;
mod reply_provider_edge_tests;
mod reply_provider_full_cycle_tests;
mod reply_provider_scroll_tests;
mod reply_provider_tui_behavior_tests;
mod reply_provider_input_tests;
mod reply_provider_session_tests;
mod grok_element_tests;

// Re-export all test modules for convenience
