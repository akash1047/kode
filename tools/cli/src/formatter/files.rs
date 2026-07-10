use crate::presenter::files::FilesView;

use super::style;

pub fn format(view: &FilesView) -> String {
    let mut out = style::header("files");
    for (i, (lang, path)) in view.entries.iter().enumerate() {
        let _ = i;
        out.push_str(&format!("  {:<8} {}\n", lang, path));
    }
    if view.entries.is_empty() {
        out.push_str("  (no files)\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presenter::files::FilesView;

    #[test]
    fn test_files_format_with_entries() {
        let view = FilesView {
            entries: vec![("rust".into(), "src/main.rs".into())],
        };
        let out = format(&view);
        assert!(out.contains("src/main.rs"));
        assert!(out.contains("rust"));
    }

    #[test]
    fn test_files_format_empty() {
        let view = FilesView { entries: vec![] };
        let out = format(&view);
        assert!(out.contains("(no files)"));
    }
}
