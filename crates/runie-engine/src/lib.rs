#![warn(clippy::all)]

//! Runie Engine — concrete tool implementations and execution logic.
//!
//! This crate holds the built-in [`Tool`] implementations that were previously
//! part of `runie-core`. The tool trait, registry, and shared types remain in
//! `runie-core` so that downstream crates can define tools without depending on
//! the engine.

pub mod tool;
