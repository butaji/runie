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
pub use data_structures::*;
pub use reducer::*;
pub use dirty_flag::*;
pub use palette_integration_tests::*;
pub use render_tests::*;
pub use state_management::*;
pub use input_handling::*;
