//! Hot reload orchestration module
//!
//! Coordinates config hot-reload from multiple sources:
//! - File system changes to the config file (via notify debouncer)
//! - SIGHUP signal on Unix systems
//! - Future: Server-initiated config version changes
//!
//! ## Architecture
//!
//! This module follows a channel-based architecture that eliminates callback complexity:
//!
//! ```text
//! ┌─────────────┐
//! │ File Watcher│──┐
//! └─────────────┘  │
//!                  ├──► ReloadTrigger ──► Reload Coordinator ──► Config ──► Agent::start()
//! ┌─────────────┐  │                      (load + validate)        (mpsc)
//! │SIGHUP Handler │──┘
//! └─────────────┘
//! ```
//!
//! Instead of passing an `Arc<Agent>` callback (which would create circular dependencies),
//! the hot reload task loads and validates configs, then sends them through an mpsc channel
//! to the main `Agent::start()` event loop, which applies them via `Agent::reload_config()`.
//!
//! This design:
//! - Decouples hot reload from Agent internals
//! - Eliminates callback overhead
//! - Makes the data flow explicit via channels
//! - Simplifies error handling and shutdown coordination

mod config_file_watcher;
mod orchestrator;
mod sighup;

pub use orchestrator::run_hot_reload;
