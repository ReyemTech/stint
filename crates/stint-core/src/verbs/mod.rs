//! Single source of truth for the 8 primitive verbs.
//!
//! Every transport (CLI, Tauri command, HTTP, MCP) delegates here. Adding a
//! new verb means a new submodule here + ≤20 LoC of wiring per transport.
//!
//! See `docs/superpowers/specs/2026-05-23-stint-phase-6-deeper-integration-design.md#211-dry-principle`.

pub mod current;
pub mod list_entries;
pub mod list_projects;
pub mod list_tasks;
pub mod start;
pub mod stop;
pub mod types;

pub use current::current;
pub use list_entries::list_entries;
pub use list_projects::list_projects;
pub use list_tasks::list_tasks;
pub use start::start;
pub use stop::stop;
pub use types::*;
