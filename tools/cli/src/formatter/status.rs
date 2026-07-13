use crate::presenter::status::StatusView;

use super::style;

pub fn format(view: &StatusView) -> String {
    let mut out = style::header("status");
    let rev = view
        .revision
        .map(|r| format!("rev {r}"))
        .unwrap_or_else(|| "no revision".into());
    out.push_str(&format!(
        "  ● {} — {} — {}\n  ● {} files · {} entities · {} nodes · {} edges · {}\n  ● source: {}\n",
        view.repository_root,
        view.workspace,
        view.languages,
        view.files,
        view.parsed,
        view.graph_nodes,
        view.graph_relationships,
        rev,
        view.source,
    ));
    if view.source == "scan" {
        out.push_str(&format!(
            "  ● parse: {} recovered · {} skipped · {} failed\n",
            view.recovered, view.skipped, view.failed,
        ));
    }
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
            source: "scan".into(),
            graph_nodes: 100,
            graph_relationships: 40,
            revision: Some(1),
        }
    }

    #[test]
    fn test_status_format_contains_values() {
        let out = format(&view());
        assert!(out.contains("50 files"));
        assert!(out.contains("40 entities"));
        assert!(out.contains("source: scan"));
    }

    #[test]
    fn test_status_format_header() {
        let out = format(&view());
        assert!(out.starts_with("\u{2500}\u{2500} kode status "));
    }
}
