//! Infrastructure layer: external I/O and platform concerns.
//!
//! Modules in this layer handle configuration files, font discovery,
//! template/brand discovery, Typst compilation, self-update, and other
//! filesystem or system interactions. They are used by the `app` layer
//! but have no knowledge of CLI concerns.

pub mod compiler;
pub mod system;
pub mod templates;
pub mod updater;

#[cfg(test)]
mod tests;
