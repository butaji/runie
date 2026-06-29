//! Generated event taxonomy — DO NOT EDIT.
//!
//! Derived from `taxonomy.json`. Files:
//! - `event_enum.rs` — `Event` enum definition
//! - `category.rs` — `EventCategory` enum + `Event::category()` impl
//! - `kind.rs` — `Event::kind()` impl + `EVENT_NAMES` + `EventCtor`
//! - `facts.rs` — `is_fact_variant()` fast-path predicate

pub mod category;
pub mod event_enum;
pub mod facts;
pub mod intent_impl;
pub mod kind;

pub use category::EventCategory;
pub use event_enum::Event;
pub use kind::EVENT_NAMES;
