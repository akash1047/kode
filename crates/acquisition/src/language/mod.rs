//! Language detection: identifies programming languages from file paths.
//!
//! # Architecture
//!
//! The [`LanguageDetector`] trait defines the subsystem contract. Multiple
//! detector implementations can be composed through [`LanguageRegistry`],
//! which delegates to registered detectors in registration order and returns
//! the first match.
//!
//! # Extension
//!
//! New language detection strategies implement [`LanguageDetector`] and
//! register with [`LanguageRegistry`]. The default registry includes
//! [`ExtensionLanguageDetector`] which maps file extensions to languages.

pub mod detector;
pub mod extension;
pub mod model;
pub mod registry;

pub use detector::LanguageDetector;
pub use extension::ExtensionLanguageDetector;
pub use model::{Language, ParseLanguageError};
pub use registry::LanguageRegistry;
