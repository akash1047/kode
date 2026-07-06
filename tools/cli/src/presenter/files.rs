use kode_acquisition::Language;
use kode_app::ScanResult;

pub struct FilesView {
    pub entries: Vec<(String, String)>,
}

impl FilesView {
    pub fn from_scan_result(result: &ScanResult, filter: Option<&Language>) -> Self {
        let entries = result
            .snapshot
            .files()
            .iter()
            .filter(|f| {
                if let Some(lang) = filter {
                    f.language() == Some(lang)
                } else {
                    true
                }
            })
            .map(|f| {
                let lang = f.language().map(|l| l.to_string()).unwrap_or_default();
                (lang, f.relative_path().display().to_string())
            })
            .collect();
        Self { entries }
    }
}
