//! Comprehensive end-to-end tests for the `/agents` command flow.
//!
//! Tests cover:
//! - Panel structure for each state (root, view, edit, delete)
//! - Action types on each item (Push, Emit, Pop, Close)
//! - Event dispatch (Open, Save, Delete)
//! - State transitions (root → view → edit → save → root)
//! - File persistence (save → load → delete)

mod action_coverage;
mod delete_panel;
mod dispatch;
mod edit_panel;
mod panel_ids;
mod persistence;
mod registration;
mod root_panel;
mod view_panel;
