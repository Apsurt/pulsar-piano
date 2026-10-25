//! Pulsar engine: MIDI in -> instruments -> audio out, entirely off the UI thread.
//!
//! This crate must never depend on Tauri, so it can be run and tested as a plain CLI.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
