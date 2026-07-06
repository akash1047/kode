use kode_app::ScanResult;

use super::{format_languages, format_workspace};

pub struct ScanView {
    pub repository_root: String,
    pub workspace: String,
    pub files_discovered: usize,
    pub directories: usize,
    pub languages: String,
    pub manifests: usize,
    pub parsed: usize,
    pub recovered: usize,
    pub skipped: usize,
    pub failed: usize,
    pub elapsed_secs: f64,
}

impl ScanView {
    pub fn from_scan_result(result: &ScanResult) -> Self {
        let snapshot = &result.snapshot;
        let stats = &result.statistics;
        Self {
            repository_root: snapshot.repository().root().display().to_string(),
            workspace: format_workspace(snapshot),
            files_discovered: stats.files_discovered,
            directories: stats.directories,
            languages: format_languages(snapshot),
            manifests: stats.manifests,
            parsed: stats.parsed,
            recovered: stats.recovered,
            skipped: stats.skipped,
            failed: stats.failed,
            elapsed_secs: stats.elapsed.as_secs_f64(),
        }
    }
}
