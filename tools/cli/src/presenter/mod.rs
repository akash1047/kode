pub mod files;
pub mod scan;
pub mod status;

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
