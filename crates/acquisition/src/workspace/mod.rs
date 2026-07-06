//! Workspace detection: identifies project workspace structure.
//!
//! # Architecture
//!
//! The [`WorkspaceDetector`] trait detects whether a repository root is:
//! - A multi-package workspace (e.g., Cargo workspace)
//! - A single-package project
//! - Not a recognized workspace at all
//!
//! Detectors are composed through [`WorkspaceRegistry`] with first-match
//! semantics, skipping detectors that return [`WorkspaceKind::None`].
//!
//! # Extension
//!
//! New workspace types (e.g., npm workspaces, Bazel) require a new
//! [`WorkspaceKind`] variant and a corresponding [`WorkspaceDetector`]
//! implementation registered with [`WorkspaceRegistry`].

pub mod cargo;
pub mod detector;
pub mod model;
pub mod registry;

pub use cargo::CargoWorkspaceDetector;
pub use detector::WorkspaceDetector;
pub use model::{Workspace, WorkspaceKind, WorkspaceMember};
pub use registry::WorkspaceRegistry;
