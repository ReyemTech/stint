//! stint-app: GUI shell over stint-core.
//!
//! Business logic lives in `stint-core`. This crate contains only Tauri
//! commands, window management, and tray plumbing.

pub mod app_state;
pub mod commands;
pub mod tray;
pub mod windows;
