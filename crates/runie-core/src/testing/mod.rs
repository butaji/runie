//! Testing utilities for Runie.
//!
//! This module provides testing infrastructure for actor-based state:
//! - `actor_harness::TestHarness` - test harness for actors
//! - `actor_harness::TestEventBus` - test event bus that records events

pub mod actor_harness;

pub use actor_harness::{TestEventBus, TestHarness};
