//! Comprehensive test suite module.
//!
//! This module contains 50+ tests organized into sections:
//! - Section 1: Harness Tests (codex pattern)
//! - Section 2: Table-Driven State Tests (crush pattern)
//! - Section 3: Mock Stream Tests (pi pattern)
//! - Section 4: Concurrent/Race Tests (crush pattern)
//! - Section 5: Cancellation/Timeout Tests (pi pattern)
//! - Section 6: Tool Execution Tests (crush + pi)
//! - Section 7: Error Recovery Tests (codex + pi)

pub mod harness;
pub mod state_tests;
pub mod stream_tests;
pub mod concurrent_tests;
pub mod timeout_tests;
pub mod tool_tests;
pub mod error_tests;

// Re-export for convenience
