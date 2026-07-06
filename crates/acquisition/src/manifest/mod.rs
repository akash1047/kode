//! Manifest detection: identifies and categorizes build manifest files.
//!
//! # Architecture
//!
//! The [`ManifestDetector`] trait defines the subsystem contract. Detectors
//! are composed through [`ManifestRegistry`] with first-match semantics.
//!
//! # Extension
//!
//! New manifest types (e.g., `Cargo.toml`, `package.json`, `pyproject.toml`)
//! require a new [`ManifestKind`] variant and a corresponding [`ManifestDetector`]
//! implementation registered with [`ManifestRegistry`].

pub mod cargo;
pub mod detector;
pub mod model;
pub mod registry;

pub use cargo::CargoManifestDetector;
pub use detector::ManifestDetector;
pub use model::{Manifest, ManifestKind};
pub use registry::ManifestRegistry;
