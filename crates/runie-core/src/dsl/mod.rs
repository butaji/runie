//! Declarative actor composition DSL.
//!
//! Provides a small, composable DSL that hides actor-message boilerplate and
//! makes state interactions declarative. Instead of manually constructing `Msg`
//! enums and sending them through `tx` handles, code reads like a data-flow
//! description:
//!
//! ```ignore
//! // Theme command handler
//! on(Event::SwitchTheme { name })
//!     .filter(|name| !name.is_empty())
//!     .intent(ConfigIntent::SetTheme { name })
//!     .notify("Theme updated")
//!
//! // Input submit
//! on(Event::Submit)
//!     .map(|_| take_submit_content())
//!     .intent(SessionIntent::AddUserMessage)
//! ```
//!
//! The DSL is **not** a new runtime — it compiles to the same actor messages.
//! It is a thin, type-safe veneer that makes the actor-ownership model
//! ergonomic and keeps business logic declarative.
//!
//! ## Modules
//!
//! - `intent.rs` — `Intent` enum re-exported from `event/intent.rs`
//! - `fact.rs` — `Fact` enum for broadcast state changes
//! - `effect.rs` — `Effect` type for fire-and-forget IO requests
//! - `flow.rs` — `Flow`, `Step`, `on()`, and all combinators
//! - `runtime.rs` — `Runtime` trait, `TestRuntime`, `RealRuntime`, and thread-local helpers
//! - `test_dsl.rs` — Test DSL (`AppStateDsl` trait) for driving AppState in tests

pub mod effect;
pub mod examples;
pub mod fact;
pub mod flow;
pub mod intent;
pub mod runtime;

pub use effect::Effect;
pub use fact::Fact;
pub use flow::{Flow, Step, on};
pub use intent::Intent;
pub use runtime::{set_runtime, with_runtime, run_flow, RealRuntime, Runtime, TestRuntime};

#[cfg(test)]
mod test_dsl;

#[cfg(test)]
pub use test_dsl::{AgentTurn, AppStateDsl};
