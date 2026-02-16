//! md-docs library crate.
//!
//! Re-exports public modules for integration tests.
//! The `cli` module is crate-internal -- it is only used by the binary entry point.

pub mod app;
pub mod domain;
pub mod infra;

// cli is not part of the public API; it is only used by main.rs.
// main.rs declares its own `mod cli` for binary-only access.
