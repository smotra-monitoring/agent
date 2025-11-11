//! Plugin system for extending agent functionality

mod registry;
mod trait_def;

pub use registry::PluginRegistry;
pub use trait_def::MonitoringPlugin;
