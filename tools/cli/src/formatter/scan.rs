use crate::presenter::scan::ScanView;

use super::style;

pub fn format(view: &ScanView) -> String {
    let mut out = style::header("scan");
    out.push_str(
        &format!(
            "  ● {} — {} — {}\n  ● {} files · {} dirs · {} manifests\n  ● {} parsed · {} recovered · {} skipped · {} failed\n  ● {:.3}s\n",
            view.repository_root,
            view.workspace,
            view.languages,
            view.files_discovered,
            view.directories,
            view.manifests,
            view.parsed,
            view.recovered,
            view.skipped,
            view.failed,
            view.elapsed_secs,
        )
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presenter::scan::ScanView;

    fn view(files: usize, parsed: usize) -> ScanView {
        ScanView {
            repository_root: "/repo".into(),
            workspace: "None".into(),
            files_discovered: files,
            directories: 5,
            languages: "rust".into(),
            manifests: 1,
            parsed,
            recovered: 2,
            skipped: 1,
            failed: 0,
            elapsed_secs: 1.5,
        }
    }

    #[test]
    fn test_scan_format_contains_values() {
        let out = format(&view(100, 80));
        assert!(out.contains("100 files"));
        assert!(out.contains("5 dirs"));
        assert!(out.contains("80 parsed"));
        assert!(out.contains("2 recovered"));
        assert!(out.contains("1.5"));
    }

    #[test]
    fn test_scan_format_header() {
        let out = format(&view(0, 0));
        assert!(out.starts_with("\u{2500}\u{2500} kode scan "));
    }
}
