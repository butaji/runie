//! # Runie Router
//!
//! Multi-provider routing abstraction for future integration into the CLI.
//!
//! ## Status: Not Yet Wired
//!
//! This crate provides clean routing abstractions ([`Router`], [`RoutingContext`],
//! [`ProviderMetadata`]) but is **not yet integrated** into the CLI or agent loop.
//!
//! - `EngineFactory::create_router()` exists in `runie-cli` but is never called
//! - `provider_factory.rs` creates single providers directly without routing
//! - The router trait and strategies are defined but dormant
//!
//! ## Planned Integration
//!
//! When wired in, the router will enable:
//! - Dynamic provider selection based on task context
//! - Cost/latency-aware routing decisions
//! - Automatic failover between providers
//! - Multi-provider handoffs in the agent loop
//!
//! ## Current Usage
//!
//! For now, the CLI uses a single provider configured via `Settings`. To experiment
//! with routing, call `EngineFactory::create_router()` and plug it into the agent
//! loop where provider decisions are made.
//!
//! ## Crates
//!
//! - `router` - Core routing trait and context types
//! - `strategies` - Routing strategy implementations

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]

pub mod router;
pub mod strategies;

pub use router::{Router, RouterError, RoutingContext, ProviderMetadata};
