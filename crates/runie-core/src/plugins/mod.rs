pub mod discovery;
pub mod manifest;
pub mod manager;
pub mod registry;

pub use discovery::{discover_plugins, DiscoveredPlugin, PluginDiscovery, PluginScope};
pub use manifest::{PluginError, PluginManifest};
pub use registry::{LoadedPlugin, PluginRegistry};
pub use manager::PluginManager;
