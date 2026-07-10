use crate::presenter::status::StatusView;

use super::style;

pub fn format(view: &StatusView) -> String {
    let mut out = style::header("status");
    out.push_str(&format!(
        "  ● {} — {} — {}\n  ● {} files · {} parsed · {} recovered · {} skipped · {} failed\n",
        view.repository_root,
        view.workspace,
        view.languages,
        view.files,
        view.parsed,
        view.recovered,
        view.skipped,
        view.failed,
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presenter::status::StatusView;

    fn view() -> StatusView {
        StatusView {
            repository_root: "/repo".into(),
            workspace: "None".into(),
            files: 50,
            languages: "rust".into(),
            parsed: 40,
            recovered: 3,
            skipped: 5,
            failed: 2,
        }
    }

    #[test]
    fn test_status_format_contains_values() {
        let out = format(&view());
        assert!(out.contains("50 files"));
        assert!(out.contains("40 parsed"));
        assert!(out.contains("3 recovered"));
        assert!(out.contains("5 skipped"));
        assert!(out.contains("2 failed"));
    }

    #[test]
    fn test_status_format_header() {
        let out = format(&view());
        assert!(out.starts_with("\u{2500}\u{2500} kode status "));
    }
}
