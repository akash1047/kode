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
