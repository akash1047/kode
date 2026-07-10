use crate::presenter::symbols::SymbolsView;

use super::style;

pub fn format(view: &SymbolsView) -> String {
    let mut out = style::header("symbols");
    if view.entries.is_empty() {
        out.push_str("  (no symbols)\n");
        return out;
    }
    for (name, kind, file, line, verified) in &view.entries {
        let mark = if *verified { "\u{2713}" } else { "?" };
        out.push_str(&format!("  {mark} {name:<20} {kind:<10} {file}:{line}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presenter::symbols::SymbolsView;

    #[test]
    fn test_symbols_format_with_entries() {
        let view = SymbolsView {
            entries: vec![
                (
                    "foo".into(),
                    "Function".into(),
                    "src/main.rs".into(),
                    10,
                    true,
                ),
                ("Bar".into(), "Struct".into(), "src/lib.rs".into(), 5, false),
            ],
        };
        let out = format(&view);
        assert!(out.contains("foo"));
        assert!(out.contains("Function"));
        assert!(out.contains("Bar"));
        assert!(out.contains("?"));
        assert!(out.contains("\u{2713}"));
    }

    #[test]
    fn test_symbols_format_empty() {
        let view = SymbolsView { entries: vec![] };
        let out = format(&view);
        assert!(out.contains("(no symbols)"));
    }
}
