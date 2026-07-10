pub mod cache;
pub mod files;
pub mod scan;
pub mod status;
pub mod symbols;

use kode_acquisition::{RepositorySnapshot, WorkspaceKind};

pub(crate) fn format_workspace(snapshot: &RepositorySnapshot) -> String {
    match snapshot.workspace().kind() {
        WorkspaceKind::CargoWorkspace { members, .. } => {
            if members.is_empty() {
                "CargoWorkspace".to_string()
            } else {
                format!(
                    "CargoWorkspace ({} member{})",
                    members.len(),
                    if members.len() == 1 { "" } else { "s" }
                )
            }
        }
        WorkspaceKind::SinglePackage { .. } => "SinglePackage".to_string(),
        WorkspaceKind::None => "None".to_string(),
    }
}

pub(crate) fn format_languages(snapshot: &RepositorySnapshot) -> String {
    snapshot
        .languages()
        .iter()
        .map(|l| l.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use kode_acquisition::*;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    fn snapshot_with_kind(kind: WorkspaceKind) -> RepositorySnapshot {
        let repo = Repository::new(".").unwrap();
        let ws = Workspace { kind };
        SnapshotBuilder::new()
            .repository(repo)
            .workspace(ws)
            .build()
            .unwrap()
    }

    #[test]
    fn test_format_workspace_none() {
        assert_eq!(
            format_workspace(&snapshot_with_kind(WorkspaceKind::None)),
            "None"
        );
    }

    #[test]
    fn test_format_workspace_single_package() {
        let kind = WorkspaceKind::SinglePackage {
            manifest: PathBuf::from("Cargo.toml"),
        };
        assert_eq!(format_workspace(&snapshot_with_kind(kind)), "SinglePackage");
    }

    #[test]
    fn test_format_workspace_cargo_empty() {
        let kind = WorkspaceKind::CargoWorkspace {
            root_manifest: PathBuf::from("Cargo.toml"),
            members: vec![],
        };
        assert_eq!(
            format_workspace(&snapshot_with_kind(kind)),
            "CargoWorkspace"
        );
    }

    #[test]
    fn test_format_workspace_cargo_one_member() {
        let kind = WorkspaceKind::CargoWorkspace {
            root_manifest: PathBuf::from("Cargo.toml"),
            members: vec![WorkspaceMember {
                relative_path: PathBuf::from("foo"),
                manifest_path: PathBuf::from("foo/Cargo.toml"),
            }],
        };
        assert_eq!(
            format_workspace(&snapshot_with_kind(kind)),
            "CargoWorkspace (1 member)"
        );
    }

    #[test]
    fn test_format_workspace_cargo_many_members() {
        let kind = WorkspaceKind::CargoWorkspace {
            root_manifest: PathBuf::from("Cargo.toml"),
            members: vec![
                WorkspaceMember {
                    relative_path: PathBuf::from("foo"),
                    manifest_path: PathBuf::from("foo/Cargo.toml"),
                },
                WorkspaceMember {
                    relative_path: PathBuf::from("bar"),
                    manifest_path: PathBuf::from("bar/Cargo.toml"),
                },
            ],
        };
        assert_eq!(
            format_workspace(&snapshot_with_kind(kind)),
            "CargoWorkspace (2 members)"
        );
    }

    fn snapshot_with_langs(langs: BTreeSet<Language>) -> RepositorySnapshot {
        let repo = Repository::new(".").unwrap();
        let inv = LanguageInventory::from_languages(langs);
        let ws = Workspace {
            kind: WorkspaceKind::None,
        };
        SnapshotBuilder::new()
            .repository(repo)
            .workspace(ws)
            .language_inventory(inv)
            .build()
            .unwrap()
    }

    #[test]
    fn test_format_languages_empty() {
        let langs = BTreeSet::new();
        assert_eq!(format_languages(&snapshot_with_langs(langs)), "");
    }

    #[test]
    fn test_format_languages_single() {
        let mut langs = BTreeSet::new();
        langs.insert(Language::Rust);
        assert_eq!(format_languages(&snapshot_with_langs(langs)), "Rust");
    }

    #[test]
    fn test_format_languages_multiple() {
        let mut langs = BTreeSet::new();
        langs.insert(Language::Markdown);
        langs.insert(Language::Rust);
        assert_eq!(
            format_languages(&snapshot_with_langs(langs)),
            "Rust, Markdown"
        );
    }
}
