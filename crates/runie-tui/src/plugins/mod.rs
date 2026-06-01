//! Plugin system for runie-tui
//!
//! Core principle: Plugins are PURE message transformers. No state. No channels. No threads.

pub mod examples;

use crate::tui::state::{AppState, Msg, Cmd};

pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    /// Transform or drop message before update()
    fn before_update(&self, _msg: &Msg) -> Option<Msg> { Some(_msg.clone()) }
    /// Emit additional messages after update()
    fn after_update(&self, _state: &AppState, _cmds: &[Cmd]) -> Vec<Msg> { vec![] }
}

pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginRegistry {

    #[must_use]
    #[must_use]
    pub fn new() -> Self { Self { plugins: vec![] } }
    pub fn register(&mut self, plugin: Box<dyn Plugin>) { self.plugins.push(plugin); }
    pub fn before_update(&self, msg: Msg) -> Option<Msg> {
        let mut msg = msg;
        for plugin in &self.plugins {
            msg = plugin.before_update(&msg)?;
        }
        Some(msg)
    }
    pub fn after_update(&self, state: &AppState, cmds: &[Cmd]) -> Vec<Msg> {
        let mut msgs = vec![];
        for plugin in &self.plugins {
            msgs.extend(plugin.after_update(state, cmds));
        }
        msgs
    }
}

impl Default for PluginRegistry {
    fn default() -> Self { Self::new() }
}
