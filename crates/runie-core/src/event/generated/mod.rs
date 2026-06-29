//! Generated event taxonomy — DO NOT EDIT.
//!
//! Derived from `taxonomy.json`. Files:
//! - `category.rs` — `EventCategory` enum + `Event::category()` impl
//! - `kind.rs` — `Event::kind()` impl + `EVENT_NAMES` + `EventCtor`
//! - `facts.rs` — `is_fact_variant()` fast-path predicate

pub mod category;
pub mod facts;
pub mod kind;

pub use category::EventCategory;
pub use facts::is_fact_variant;
pub use kind::EventCtor;
pub use kind::EVENT_NAMES;
