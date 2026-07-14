//! Core graph primitives for orchestration patterns (phase-gated).
//!
//! Phase 3 ships [`dag::Dag`]: dependency-graph building with topological
//! waves and cycle detection, backing the swarm dag variant.

pub mod dag;
