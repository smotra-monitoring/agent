//! TUI module for interactive terminal interface

pub mod render;
pub mod runner;
pub mod ui_loop;

pub use runner::start_tui;
