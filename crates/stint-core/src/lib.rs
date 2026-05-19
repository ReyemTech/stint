//! stint-core: shared library for the stint time tracker.
//!
//! All business logic lives here. Both the CLI and the Tauri GUI link to this
//! crate; neither contains domain code of its own.

pub mod config;
pub mod error;
pub mod ids;
pub mod oauth;
pub mod paths;
pub mod recovery;
pub mod solidtime;
pub mod store;
pub mod sync;
pub mod time;
pub mod timer;

pub use error::{Error, Result};
