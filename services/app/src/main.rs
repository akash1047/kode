//! Application binary: wires together all subsystems into a runnable service.
//!
//! This is the composition root for the `kode` service. It owns dependency
//! initialization and lifecycle management for all subsystems.
//!
//! # Philosophy
//!
//! - The binary is thin: all logic lives in crates.
//! - The binary owns configuration, wiring, and startup.
//! - The binary does not implement business logic.

fn main() {}
