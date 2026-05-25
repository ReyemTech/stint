//! `stint skill install` machinery.
//!
//! Each supported editor/agent (Claude Code, Codex, OpenCode, …) implements
//! the [`harness::Harness`] trait. The CLI dispatches via [`find`] (explicit
//! `<harness>` arg) or [`picker::pick`] (interactive).

pub mod claude_code;
pub mod codex;
pub mod harness;
pub mod opencode;
pub mod picker;

use harness::Harness;

/// All harnesses known to this build, in display order.
pub fn all_harnesses() -> Vec<Box<dyn Harness>> {
    vec![
        Box::new(claude_code::ClaudeCode),
        Box::new(codex::Codex),
        Box::new(opencode::OpenCode),
    ]
}

/// Look up a harness by its lowercase CLI name.
pub fn find(name: &str) -> Option<Box<dyn Harness>> {
    all_harnesses().into_iter().find(|h| h.name() == name)
}
