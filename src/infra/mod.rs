//! Infrastructure layer: external I/O and platform concerns.
//!
//! Modules in this layer handle configuration files, font discovery,
//! template/brand discovery, and other filesystem or system interactions.
//! They are used by the `app` layer but have no knowledge of CLI concerns.

pub mod config;
pub mod fonts;
pub mod logger;
pub mod templates;

#[cfg(test)]
mod tests;
