use kode_query::SymbolResult;

pub struct SymbolsView {
    pub entries: Vec<(String, String, String, usize, bool)>,
}

impl SymbolsView {
    pub fn from_symbols(results: &[SymbolResult]) -> Self {
        let entries = results
            .iter()
            .map(|r| {
                let kind = format!("{:?}", r.kind);
                let file = r.file_path.display().to_string();
                (r.name.clone(), kind, file, r.start_line, r.verified)
            })
            .collect();
        Self { entries }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kode_graph::NodeKind;
    use kode_query::SymbolResult;
    use std::path::PathBuf;

    fn sym(name: &str, kind: NodeKind, verified: bool) -> SymbolResult {
        SymbolResult {
            name: name.into(),
            kind,
            file_path: PathBuf::from("src/main.rs"),
            start_line: 10,
            start_column: 0,
            end_line: 20,
            end_column: 0,
            verified,
        }
    }

    #[test]
    fn test_symbols_view_with_entries() {
        let results = vec![
            sym("foo", NodeKind::Function, true),
            sym("Bar", NodeKind::Struct, false),
        ];
        let view = SymbolsView::from_symbols(&results);
        assert_eq!(view.entries.len(), 2);
        assert_eq!(view.entries[0].0, "foo");
        assert!(view.entries[0].4);
        assert!(!view.entries[1].4);
    }

    #[test]
    fn test_symbols_view_empty() {
        let view = SymbolsView::from_symbols(&[]);
        assert!(view.entries.is_empty());
    }
}
