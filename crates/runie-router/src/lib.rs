#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]

pub mod router;
pub mod strategies;

pub use router::{Router, RouterError, RoutingContext, ProviderMetadata};
