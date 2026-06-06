//! `kode` — codebase evidence service for humans and AI agents.
//!
//! Builds a local, incremental cache of any repo (files, symbols, manifests)
//! and exposes it via an MCP server or interactive chat REPL. Every answer is
//! backed by a live file read with `path:line` citations — no training-data
//! guessing.
//!
//! See [`DESIGN.md`](https://github.com/akash1047/kode/blob/main/DESIGN.md)
//! for the architecture and cache invariants.

pub mod cache;
pub mod chat;
pub mod cli;
pub mod commands;
pub mod config;
pub mod fs;
pub mod mcp;
pub mod project;
