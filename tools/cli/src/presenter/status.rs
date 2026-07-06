use kode_app::ScanResult;

use super::{format_languages, format_workspace};

pub struct StatusView {
    pub repository_root: String,
    pub workspace: String,
    pub files: usize,
    pub languages: String,
    pub parsed: usize,
    pub recovered: usize,
    pub skipped: usize,
    pub failed: usize,
}

impl StatusView {
    pub fn from_scan_result(result: &ScanResult) -> Self {
        let snapshot = &result.snapshot;
        let stats = &result.statistics;
        Self {
            repository_root: snapshot.repository().root().display().to_string(),
            workspace: format_workspace(snapshot),
            files: stats.files_discovered,
            languages: format_languages(snapshot),
            parsed: stats.parsed,
            recovered: stats.recovered,
            skipped: stats.skipped,
            failed: stats.failed,
        }
    }
}
