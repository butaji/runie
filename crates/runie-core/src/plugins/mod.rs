pub mod discovery;
pub mod manager;
pub mod manifest;
pub mod registry;

pub use discovery::{discover_plugins, DiscoveredPlugin, PluginDiscovery, PluginScope};
pub use manager::PluginManager;
pub use manifest::{PluginError, PluginManifest};
pub use registry::{LoadedPlugin, PluginRegistry};
