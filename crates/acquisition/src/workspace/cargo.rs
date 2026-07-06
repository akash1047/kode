use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::error::Error;
use crate::workspace::detector::WorkspaceDetector;
use crate::workspace::model::{Workspace, WorkspaceKind, WorkspaceMember};

pub struct CargoWorkspaceDetector;

impl WorkspaceDetector for CargoWorkspaceDetector {
    fn detect(&self, root: &Path) -> Result<Workspace, Error> {
        let cargo_toml = root.join("Cargo.toml");
        if !cargo_toml.exists() {
            return Ok(Workspace {
                kind: WorkspaceKind::None,
            });
        }

        let content = std::fs::read_to_string(&cargo_toml).map_err(|source| {
            Error::ManifestRead {
                path: cargo_toml.clone(),
                source,
            }
        })?;

        let parsed: CargoManifest = toml::from_str(&content).map_err(|source| {
            Error::ManifestParse {
                path: cargo_toml.clone(),
                source,
            }
        })?;

        if let Some(ws) = &parsed.workspace {
            let members = resolve_workspace_members(root, &ws.members)?;

            Ok(Workspace {
                kind: WorkspaceKind::CargoWorkspace {
                    root_manifest: PathBuf::from("Cargo.toml"),
                    members,
                },
            })
        } else if parsed.package.is_some() {
            Ok(Workspace {
                kind: WorkspaceKind::SinglePackage {
                    manifest: PathBuf::from("Cargo.toml"),
                },
            })
        } else {
            Ok(Workspace {
                kind: WorkspaceKind::None,
            })
        }
    }
}

fn resolve_workspace_members(
    root: &Path,
    members: &Option<Vec<String>>,
) -> Result<Vec<WorkspaceMember>, Error> {
    let Some(patterns) = members else {
        return Ok(Vec::new());
    };

    let mut result = Vec::new();
    let mut seen = BTreeSet::new();

    for pattern in patterns {
        if pattern.contains('*') {
            let parent = root.join(
                pattern
                    .rsplit_once('/')
                    .map(|(p, _)| p)
                    .unwrap_or("."),
            );

            if let Ok(entries) = std::fs::read_dir(&parent) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let manifest = path.join("Cargo.toml");
                        if manifest.exists() {
                            if let Ok(relative) = path.strip_prefix(root) {
                                if seen.insert(relative.to_path_buf()) {
                                    result.push(WorkspaceMember {
                                        manifest_path: relative.join("Cargo.toml"),
                                        relative_path: relative.to_path_buf(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        } else {
            let path = root.join(pattern);
            let manifest = path.join("Cargo.toml");
            if manifest.exists() {
                let relative = PathBuf::from(pattern);
                if seen.insert(relative.clone()) {
                    result.push(WorkspaceMember {
                        manifest_path: relative.join("Cargo.toml"),
                        relative_path: relative,
                    });
                }
            }
        }
    }

    result.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(result)
}

#[derive(serde::Deserialize)]
struct CargoManifest {
    package: Option<Package>,
    workspace: Option<WorkspaceDef>,
}

#[derive(serde::Deserialize)]
struct Package {}

#[derive(serde::Deserialize)]
struct WorkspaceDef {
    members: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn default_detector() -> CargoWorkspaceDetector {
        CargoWorkspaceDetector
    }

    #[test]
    fn none_when_no_cargo_toml() {
        let dir = tempfile::TempDir::new().unwrap();
        let detector = default_detector();
        let ws = detector.detect(dir.path()).unwrap();
        assert_eq!(ws.kind(), &WorkspaceKind::None);
        assert!(ws.manifest_paths().is_empty());
    }

    #[test]
    fn single_package() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            r#"[package]
name = "test-pkg"
version = "0.1.0"
"#,
        )
        .unwrap();

        let detector = default_detector();
        let ws = detector.detect(dir.path()).unwrap();
        assert_eq!(
            ws.kind(),
            &WorkspaceKind::SinglePackage {
                manifest: PathBuf::from("Cargo.toml")
            }
        );
        assert_eq!(ws.manifest_paths().len(), 1);
    }

    #[test]
    fn cargo_workspace() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();

        fs::write(
            root.join("Cargo.toml"),
            r#"[workspace]
members = ["crates/alpha", "crates/beta"]

[package]
name = "root"
version = "0.1.0"
"#,
        )
        .unwrap();

        fs::create_dir_all(root.join("crates/alpha/src")).unwrap();
        fs::write(
            root.join("crates/alpha/Cargo.toml"),
            r#"[package]
name = "alpha"
version = "0.1.0"
"#,
        )
        .unwrap();

        fs::create_dir_all(root.join("crates/beta/src")).unwrap();
        fs::write(
            root.join("crates/beta/Cargo.toml"),
            r#"[package]
name = "beta"
version = "0.1.0"
"#,
        )
        .unwrap();

        let detector = default_detector();
        let ws = detector.detect(root).unwrap();
        match ws.kind() {
            WorkspaceKind::CargoWorkspace {
                root_manifest,
                members,
            } => {
                assert_eq!(*root_manifest, PathBuf::from("Cargo.toml"));
                assert_eq!(members.len(), 2);
                assert_eq!(members[0].relative_path(), Path::new("crates/alpha"));
                assert_eq!(members[1].relative_path(), Path::new("crates/beta"));
            }
            _ => panic!("expected CargoWorkspace"),
        }
        assert_eq!(ws.manifest_paths().len(), 3);
    }

    #[test]
    fn workspace_with_glob_members() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();

        fs::write(
            root.join("Cargo.toml"),
            r#"[workspace]
members = ["crates/*"]
"#,
        )
        .unwrap();

        fs::create_dir_all(root.join("crates/one/src")).unwrap();
        fs::write(
            root.join("crates/one/Cargo.toml"),
            r#"[package]
name = "one"
version = "0.1.0"
"#,
        )
        .unwrap();

        fs::create_dir_all(root.join("crates/two/src")).unwrap();
        fs::write(
            root.join("crates/two/Cargo.toml"),
            r#"[package]
name = "two"
version = "0.1.0"
"#,
        )
        .unwrap();

        fs::create_dir_all(root.join("crates/three")).unwrap();

        let detector = default_detector();
        let ws = detector.detect(root).unwrap();
        match ws.kind() {
            WorkspaceKind::CargoWorkspace { members, .. } => {
                assert_eq!(members.len(), 2);
            }
            _ => panic!("expected CargoWorkspace"),
        }
    }
}
