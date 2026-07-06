pub mod cargo;
pub mod detector;
pub mod model;
pub mod registry;

pub use cargo::CargoManifestDetector;
pub use detector::ManifestDetector;
pub use model::{Manifest, ManifestKind};
pub use registry::ManifestRegistry;
