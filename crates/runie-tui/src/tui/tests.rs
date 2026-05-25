#![allow(clippy::unwrap_used)]
#![cfg(test)]

mod data_structures;
mod reducer;
mod dirty_flag;
mod palette_integration_tests;

// Re-export all test modules for convenience
pub use data_structures::*;
pub use reducer::*;
pub use dirty_flag::*;
pub use palette_integration_tests::*;
