//! Library surface for `stint-cli`.
//!
//! The crate ships primarily as a binary (`stint`), but a thin library is
//! exposed so integration tests and (eventually) other tools can drive the
//! skill-install machinery without going through the binary entry point.

pub mod skill;
