//! Google Calendar provider. Reuses `crate::oauth` for the PKCE flow and
//! `reqwest` for the v3 REST surface.

pub mod client;
pub mod config;
pub mod dto;
