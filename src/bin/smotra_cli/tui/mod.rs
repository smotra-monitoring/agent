//! TUI module for interactive terminal interface

pub mod render;
mod runner;
mod ui_loop;

pub use runner::run_tui;
pub use ui_loop::run_ui_loop;
