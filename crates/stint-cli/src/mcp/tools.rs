//! Tool surface for the MCP server. Each tool delegates to a single
//! `stint_core::verbs::*` function — no business logic here.
//!
//! Task 21 will replace the empty `#[tool_router]` block with the 8 real
//! tools.

use rmcp::{tool_router, ServerHandler};
use stint_core::store::Store;

#[derive(Clone)]
pub struct StintServer {
    #[allow(dead_code)] // wired in Task 21
    store: std::sync::Arc<Store>,
}

impl StintServer {
    pub fn new(store: Store) -> Self {
        Self {
            store: std::sync::Arc::new(store),
        }
    }
}

#[tool_router(server_handler)]
impl StintServer {
    // Task 21 wires tools here. Empty for now — server still initializes,
    // tools/list returns an empty list.
}

// `#[tool_router(server_handler)]` auto-derives `ServerHandler` with a
// default `get_info` whose name comes from `Implementation::from_build_env()`
// (i.e. "stint-cli"). If we ever need a custom display name we can drop
// `server_handler` and emit a manual `#[tool_handler] impl ServerHandler`
// pair per the rmcp README.

#[allow(dead_code)]
fn _assert_server_handler()
where
    StintServer: ServerHandler,
{
}
